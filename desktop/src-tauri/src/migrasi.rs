//! Migrasi toko: seluruh isi satu toko keluar-masuk lewat berkas `.gpos`.
//!
//! **Berkasnya dibaca aplikasi lain.** Bentuk byte-nya dikunci di
//! `contracts/MIGRASI.md` di repo `Gpos2`, dan implementasi keduanya ada di
//! `core/src/galaxyas_core/migrasi/` (Python). Apa pun yang berubah di modul ini
//! berubah di sana juga, di commit yang sama — dan fixture uji silang
//! (`contracts/fixtures/migrasi-contoh.gpos`) yang jadi tempat pertemuannya.
//!
//! Tiga hal yang paling gampang dirusak tanpa sadar, ditulis di depan supaya
//! tidak perlu ditemukan lagi:
//!
//! 1. **Tanda pada `stock_movements`.** Di database ini besarannya disimpan
//!    tanpa tanda dan arahnya cuma tersirat di `kind`; di bundel ia delta
//!    bertanda. Konversinya di dua tempat dan harus tetap saling kebalikan.
//! 2. **`products.id` tidak pernah diturunkan ulang** di arah mana pun — ia
//!    paku yang menahan sync ke `jjapps.net`.
//! 3. **Impor mengganti, bukan menggabung.** Isi toko aktif dikosongkan lebih
//!    dulu, sesudah database-nya disalin. Gagal menyalin berarti batal.

use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use aes_gcm::aead::{Aead, KeyInit, Payload};
use aes_gcm::{Aes256Gcm, Key, Nonce};
use chrono::Utc;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use flate2::Compression;
use hkdf::Hkdf;
use rusqlite::{params, Connection, Row};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sha2::Sha256;
use std::io::{Read, Write};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

// ---------------------------------------------------------------------------
// Wadah — MIGRASI.md §2
// ---------------------------------------------------------------------------

const MAGIC: &[u8; 8] = b"GXPOSMIG";
const VERSI_WADAH: u16 = 1;
const BENDERA_ZLIB: u16 = 1;

const PANJANG_GARAM: usize = 16;
const PANJANG_NONCE: usize = 12;
const PANJANG_TAG: usize = 16;

/// Panjang seluruh bagian tetap sebelum kepala.
const TETAP: usize = 8 + 2 + 2 + PANJANG_GARAM + PANJANG_NONCE + 4;

/// Bahan kunci yang tertanam di **dua** aplikasi. Nilainya wajib sama persis
/// dengan `RAHASIA` di `core/src/galaxyas_core/migrasi/wadah.py` di repo
/// `Gpos2`; meleset satu nibble saja membuat berkas dari sebelah gagal dibuka
/// dengan pesan "rusak" yang menyesatkan.
///
/// Perlu dikatakan terus terang: kunci yang tertanam di aplikasi bisa
/// dikeluarkan orang yang serius dan punya exe-nya. Ini **penghalang, bukan
/// pengaman** — ia menahan berkas dibuka Excel atau disunting tangan, bukan
/// menahan penyerang. Yang benar-benar terpakai sehari-hari justru tag AEAD-nya:
/// berkas yang rusak di tengah jalan ketahuan sebelum satu baris pun masuk ke
/// database.
const RAHASIA: [u8; 32] = [
    0x7b, 0x2f, 0x4c, 0x9a, 0x1e, 0x6d, 0x38, 0xb0, 0x5a, 0xf7, 0xc2, 0xe9, 0x4d, 0x1b, 0x60, 0x38,
    0xa5, 0xe8, 0xf3, 0xc7, 0x0d, 0x92, 0xb4, 0x1e, 0x6f, 0xa8, 0xc3, 0x5d, 0x70, 0x92, 0xe1, 0xb4,
];

const INFO_HKDF: &[u8] = b"galaxyas-migrasi-v1";

const APP_V1: &str = "gpos1";
const APP_V2: &str = "gpos2";
const VERSI_BUNDEL: i64 = 1;

/// Kunci `settings` tempat lampiran POS 2 yang tidak dimengerti aplikasi ini
/// dititipkan (MIGRASI.md §4.5). Disimpan utuh dan dikembalikan saat ekspor,
/// jadi perjalanan POS 2 → v1 → POS 2 tidak menghapus template laporan yang
/// tidak pernah kelihatan di sini.
const KUNCI_LAMPIRAN: &str = "migrasi_lampiran";

fn kunci(garam: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(garam), &RAHASIA);
    let mut keluar = [0u8; 32];
    hk.expand(INFO_HKDF, &mut keluar)
        .expect("32 byte selalu muat untuk HKDF-SHA256");
    keluar
}

fn acak(n: usize) -> Vec<u8> {
    // Sumber acaknya `getrandom` lewat `Uuid::new_v4` — sengaja memakai crate
    // yang sudah ada di sini ketimbang menambah `rand` hanya demi 28 byte.
    let mut keluar = Vec::with_capacity(n);
    while keluar.len() < n {
        keluar.extend_from_slice(Uuid::new_v4().as_bytes());
    }
    keluar.truncate(n);
    keluar
}

fn bungkus(kepala: &Value, bundel: &Value) -> AppResult<Vec<u8>> {
    let garam = acak(PANJANG_GARAM);
    let nonce = acak(PANJANG_NONCE);

    let mut enc = ZlibEncoder::new(Vec::new(), Compression::best());
    enc.write_all(serde_json::to_string(bundel).map_err(json_err)?.as_bytes())?;
    let isi_jelas = enc.finish()?;

    let kepala_byte = serde_json::to_vec(kepala).map_err(json_err)?;

    // Panjang isi sudah pasti sebelum dienkripsi: AES-GCM tidak memuaikan apa
    // pun selain menambahkan tag 16 byte. Kepastian itu yang membuat AAD bisa
    // mencakup bidang panjang isi.
    let panjang_isi = (isi_jelas.len() + PANJANG_TAG) as u64;

    let mut depan = Vec::with_capacity(TETAP + kepala_byte.len() + 8);
    depan.extend_from_slice(MAGIC);
    depan.extend_from_slice(&VERSI_WADAH.to_le_bytes());
    depan.extend_from_slice(&BENDERA_ZLIB.to_le_bytes());
    depan.extend_from_slice(&garam);
    depan.extend_from_slice(&nonce);
    depan.extend_from_slice(&(kepala_byte.len() as u32).to_le_bytes());
    depan.extend_from_slice(&kepala_byte);
    depan.extend_from_slice(&panjang_isi.to_le_bytes());

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&kunci(&garam)));
    let isi = cipher
        .encrypt(
            Nonce::from_slice(&nonce),
            Payload {
                msg: &isi_jelas,
                aad: &depan,
            },
        )
        .map_err(|_| AppError::Other("Gagal menyusun berkas migrasi.".into()))?;

    let mut keluar = depan;
    keluar.extend_from_slice(&isi);
    Ok(keluar)
}

struct Bedah<'a> {
    garam: &'a [u8],
    nonce: &'a [u8],
    kepala: Value,
    isi: &'a [u8],
    depan: &'a [u8],
}

/// Pisahkan berkas jadi bagian-bagiannya, atau tolak dengan kalimat yang tepat.
///
/// Ketiga penolakan sengaja berbunyi berbeda: tindakan pemakainya berbeda
/// (pilih berkas lain · perbarui aplikasi · minta berkas baru), jadi
/// menggabungkannya jadi satu "berkas tidak valid" membuang satu-satunya
/// petunjuk yang berguna.
fn bedah(data: &[u8]) -> AppResult<Bedah<'_>> {
    if data.len() < TETAP || &data[..8] != MAGIC {
        return Err(AppError::Other(
            "Ini bukan berkas migrasi GALAXYAS POS.".into(),
        ));
    }

    let versi = u16::from_le_bytes([data[8], data[9]]);
    let bendera = u16::from_le_bytes([data[10], data[11]]);
    if versi > VERSI_WADAH {
        return Err(AppError::Other(format!(
            "Berkas ini dibuat aplikasi yang lebih baru (versi wadah {versi}). \
             Perbarui GALAXYAS POS dulu."
        )));
    }
    if bendera & !BENDERA_ZLIB != 0 {
        return Err(AppError::Other(
            "Berkas migrasi memakai penanda yang tidak dikenal.".into(),
        ));
    }

    let awal = 12;
    let garam = &data[awal..awal + PANJANG_GARAM];
    let nonce = &data[awal + PANJANG_GARAM..awal + PANJANG_GARAM + PANJANG_NONCE];
    let panjang_kepala =
        u32::from_le_bytes(data[TETAP - 4..TETAP].try_into().expect("4 byte")) as usize;

    let akhir_kepala = TETAP.saturating_add(panjang_kepala);
    if data.len() < akhir_kepala + 8 {
        return Err(AppError::Other("Berkas migrasi terpotong.".into()));
    }

    let panjang_isi = u64::from_le_bytes(
        data[akhir_kepala..akhir_kepala + 8]
            .try_into()
            .expect("8 byte"),
    ) as usize;
    let awal_isi = akhir_kepala + 8;

    // Panjangnya harus **tepat**, bukan "cukup". Byte berlebih di ujung berarti
    // ada yang menempelkan sesuatu, dan itu bukan berkas yang mau dimasukkan ke
    // database toko.
    if data.len() != awal_isi.saturating_add(panjang_isi) || panjang_isi < PANJANG_TAG {
        return Err(AppError::Other(
            "Berkas migrasi terpotong atau ada tambahan di ujungnya.".into(),
        ));
    }

    let kepala: Value = serde_json::from_slice(&data[TETAP..akhir_kepala])
        .map_err(|_| AppError::Other("Keterangan di berkas migrasi tidak bisa dibaca.".into()))?;
    if !kepala.is_object() {
        return Err(AppError::Other(
            "Keterangan di berkas migrasi tidak bisa dibaca.".into(),
        ));
    }

    Ok(Bedah {
        garam,
        nonce,
        kepala,
        isi: &data[awal_isi..],
        depan: &data[..awal_isi],
    })
}

fn baca_kepala_mentah(data: &[u8]) -> AppResult<Value> {
    Ok(bedah(data)?.kepala)
}

fn buka(data: &[u8]) -> AppResult<Value> {
    let b = bedah(data)?;

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&kunci(b.garam)));
    let isi_jelas = cipher
        .decrypt(
            Nonce::from_slice(b.nonce),
            Payload {
                msg: b.isi,
                aad: b.depan,
            },
        )
        .map_err(|_| {
            AppError::Other(
                "Berkas migrasi rusak atau sudah diubah. Minta berkas baru dari PC asalnya.".into(),
            )
        })?;

    let bendera = u16::from_le_bytes([data[10], data[11]]);
    let mentah = if bendera & BENDERA_ZLIB != 0 {
        let mut dec = ZlibDecoder::new(&isi_jelas[..]);
        let mut keluar = Vec::new();
        dec.read_to_end(&mut keluar)
            .map_err(|_| AppError::Other("Isi berkas migrasi tidak bisa dibuka.".into()))?;
        keluar
    } else {
        isi_jelas
    };

    let bundel: Value = serde_json::from_slice(&mentah)
        .map_err(|_| AppError::Other("Isi berkas migrasi tidak dikenali.".into()))?;
    if !bundel.is_object() {
        return Err(AppError::Other("Isi berkas migrasi tidak dikenali.".into()));
    }
    Ok(bundel)
}

fn json_err(e: serde_json::Error) -> AppError {
    AppError::Other(format!("Isi berkas migrasi tidak bisa disusun: {e}"))
}

// ---------------------------------------------------------------------------
// Bentuk yang dilihat frontend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SumberBerkas {
    pub app: String,
    pub versi_app: String,
    pub versi_bundel: i64,
    pub dibuat: String,
    pub toko_id: String,
    pub toko_nama: String,
    pub jumlah: std::collections::BTreeMap<String, i64>,
    pub omzet_total: i64,
    pub penjualan_pertama: Option<String>,
    pub penjualan_terakhir: Option<String>,
    /// Kalimat dari aplikasi yang **membuat** berkas: apa yang akan hilang
    /// kalau berkas ini diimpor ke sini. Ada di kepala yang terbuka justru
    /// supaya bisa dibaca **sebelum** tombol Impor ditekan.
    pub catatan: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HasilMigrasi {
    pub berkas: String,
    pub ukuran: i64,
    pub sumber: SumberBerkas,
    pub baris: std::collections::BTreeMap<String, i64>,
    pub dilewati: std::collections::BTreeMap<String, i64>,
    pub peringatan: Vec<String>,
    pub cadangan: Option<String>,
}

fn teks(v: &Value, kunci: &str) -> String {
    v.get(kunci)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn teks_opsional(v: &Value, kunci: &str) -> Option<String> {
    match v.get(kunci) {
        Some(Value::String(s)) if !s.is_empty() => Some(s.clone()),
        _ => None,
    }
}

fn angka(v: &Value, kunci: &str) -> i64 {
    v.get(kunci).and_then(Value::as_i64).unwrap_or(0)
}

fn angka_opsional(v: &Value, kunci: &str) -> Option<i64> {
    match v.get(kunci) {
        None | Some(Value::Null) => None,
        Some(lain) => lain.as_i64(),
    }
}

fn bendera(v: &Value, kunci: &str, bawaan: bool) -> bool {
    v.get(kunci).and_then(Value::as_bool).unwrap_or(bawaan)
}

fn baca_sumber(kepala: &Value) -> SumberBerkas {
    let toko = kepala.get("toko").cloned().unwrap_or(Value::Null);
    let ringkas = kepala.get("ringkas").cloned().unwrap_or(Value::Null);

    let mut jumlah = std::collections::BTreeMap::new();
    if let Some(Value::Object(map)) = kepala.get("jumlah") {
        for (k, v) in map {
            jumlah.insert(k.clone(), v.as_i64().unwrap_or(0));
        }
    }

    SumberBerkas {
        app: teks(kepala, "app"),
        versi_app: teks(kepala, "versi_app"),
        versi_bundel: angka(kepala, "versi_bundel"),
        dibuat: teks(kepala, "dibuat"),
        toko_id: teks(&toko, "id"),
        toko_nama: teks(&toko, "nama"),
        jumlah,
        omzet_total: angka(&ringkas, "omzet_total"),
        penjualan_pertama: teks_opsional(&ringkas, "penjualan_pertama"),
        penjualan_terakhir: teks_opsional(&ringkas, "penjualan_terakhir"),
        catatan: kepala
            .get("catatan")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(str::to_string))
                    .collect()
            })
            .unwrap_or_default(),
    }
}

// ---------------------------------------------------------------------------
// Konversi satuan — MIGRASI.md §4.1
// ---------------------------------------------------------------------------

/// `REAL` rupiah → INTEGER rupiah, pembulatan setengah ke atas.
fn ke_rupiah(nilai: f64) -> i64 {
    (nilai.abs() + 0.5).floor() as i64 * if nilai < 0.0 { -1 } else { 1 }
}

/// `REAL` kuantitas → INTEGER mili-satuan.
fn ke_mili(nilai: f64) -> i64 {
    let skala = nilai * 1000.0;
    (skala.abs() + 0.5).floor() as i64 * if skala < 0.0 { -1 } else { 1 }
}

fn dari_mili(nilai: i64) -> f64 {
    nilai as f64 / 1000.0
}

/// Jenis mutasi yang **mengurangi** stok. Di database ini besarannya tanpa
/// tanda; di bundel ia delta bertanda.
fn mengurangi(kind: &str) -> bool {
    kind == "out" || kind == "sale"
}

/// Besaran gaya v1 → delta bertanda gaya bundel. `opname` tidak disentuh:
/// isinya nilai absolut, bukan delta.
fn ke_delta(kind: &str, mili: i64) -> i64 {
    if mengurangi(kind) {
        -mili.abs()
    } else {
        mili
    }
}

// ---------------------------------------------------------------------------
// Setelan yang tidak ikut pindah — MIGRASI.md §4.4
// ---------------------------------------------------------------------------

const SETELAN_TAK_IKUT: &[&str] = &[
    "last_pull_at",
    "app_running",
    "lan_server_enabled",
    "lan_server_token",
];

fn setelan_ikut(kunci: &str) -> bool {
    !SETELAN_TAK_IKUT.contains(&kunci) && !kunci.starts_with("relay_") && kunci != KUNCI_LAMPIRAN
}

// ---------------------------------------------------------------------------
// Ekspor
// ---------------------------------------------------------------------------

fn kumpulkan<T, F>(conn: &Connection, sql: &str, mut baca: F) -> AppResult<Vec<T>>
where
    F: FnMut(&Row<'_>) -> rusqlite::Result<T>,
{
    let mut stmt = conn.prepare(sql)?;
    let baris = stmt.query_map([], |r| baca(r))?;
    let mut keluar = Vec::new();
    for b in baris {
        keluar.push(b?);
    }
    Ok(keluar)
}

/// Susun berkas migrasi untuk toko yang koneksinya diberikan.
pub fn ekspor(conn: &Connection, toko_nama: &str) -> AppResult<(Vec<u8>, HasilMigrasi)> {
    let mut tabel = Map::new();

    tabel.insert("settings".into(), Value::Array(ambil_settings(conn)?));
    tabel.insert("users".into(), Value::Array(ambil_users(conn)?));
    tabel.insert("brands".into(), Value::Array(ambil_brands(conn)?));
    tabel.insert("customers".into(), Value::Array(ambil_customers(conn)?));
    tabel.insert("products".into(), Value::Array(ambil_products(conn)?));
    tabel.insert("stock".into(), Value::Array(ambil_stock(conn)?));
    tabel.insert(
        "discount_periods".into(),
        Value::Array(ambil_discounts(conn)?),
    );
    tabel.insert("shifts".into(), Value::Array(ambil_shifts(conn)?));
    tabel.insert("sales".into(), Value::Array(ambil_sales(conn)?));
    tabel.insert("sale_items".into(), Value::Array(ambil_sale_items(conn)?));
    tabel.insert("stock_batches".into(), Value::Array(ambil_batches(conn)?));
    tabel.insert(
        "stock_movements".into(),
        Value::Array(ambil_movements(conn)?),
    );
    tabel.insert("expenses".into(), Value::Array(ambil_expenses(conn)?));

    let mut jumlah = std::collections::BTreeMap::new();
    for (nama, isi) in &tabel {
        jumlah.insert(nama.clone(), isi.as_array().map_or(0, |a| a.len() as i64));
    }

    let catatan = peringatan_ekspor(&tabel);
    let toko_id = crate::db::get_setting(conn, "store_id")?.unwrap_or_default();

    let (pertama, terakhir, omzet) = ringkas_penjualan(conn)?;
    let kepala = json!({
        "app": APP_V1,
        "versi_app": env!("CARGO_PKG_VERSION"),
        "versi_bundel": VERSI_BUNDEL,
        "dibuat": Utc::now().to_rfc3339(),
        "toko": { "id": toko_id, "nama": toko_nama },
        "jumlah": jumlah,
        "ringkas": {
            "penjualan_pertama": pertama,
            "penjualan_terakhir": terakhir,
            "omzet_total": omzet,
        },
        "catatan": catatan,
    });

    let bundel = json!({
        "format": VERSI_BUNDEL,
        // Kepala ikut masuk ke isi yang terenkripsi juga: yang di luar boleh
        // dibaca tanpa kunci, yang dipercaya saat mengimpor yang di dalam —
        // ia terlindungi tag GCM.
        "sumber": kepala,
        "tabel": Value::Object(tabel),
        "lampiran": lampiran_titipan(conn)?,
    });

    let data = bungkus(&kepala, &bundel)?;
    let hasil = HasilMigrasi {
        berkas: String::new(),
        ukuran: data.len() as i64,
        sumber: baca_sumber(&kepala),
        baris: jumlah,
        dilewati: Default::default(),
        peringatan: catatan,
        cadangan: None,
    };
    Ok((data, hasil))
}

fn ringkas_penjualan(conn: &Connection) -> AppResult<(Option<String>, Option<String>, i64)> {
    let (min, max, total): (Option<String>, Option<String>, f64) = conn.query_row(
        "SELECT MIN(created_at), MAX(created_at), COALESCE(SUM(total), 0) FROM transactions",
        [],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    Ok((min, max, ke_rupiah(total)))
}

/// Lampiran yang pernah dititipkan POS 2 dan disimpan apa adanya (§4.5).
fn lampiran_titipan(conn: &Connection) -> AppResult<Value> {
    match crate::db::get_setting(conn, KUNCI_LAMPIRAN)? {
        Some(isi) if !isi.trim().is_empty() => {
            Ok(serde_json::from_str(&isi).unwrap_or_else(|_| json!({})))
        }
        _ => Ok(json!({})),
    }
}

fn peringatan_ekspor(tabel: &Map<String, Value>) -> Vec<String> {
    let mut pesan = Vec::new();

    let pengguna = tabel
        .get("users")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    if pengguna > 0 {
        pesan.push(format!(
            "{pengguna} pengguna ikut berikut PIN-nya. Kalau berkas ini diimpor ke \
             GALAXYAS POS 2, PIN-nya diubah jadi hash di sana dan kasir tetap memakai \
             PIN yang sama."
        ));
    }

    // Kelemahan yang diwarisi database ini dan tidak bisa diperbaiki dari sini:
    // stok adalah angka berjalan, dan buku besarnya tidak selalu menjumlah jadi
    // angka itu. POS 2 akan menutup selisihnya dengan baris opname bertanggal
    // migrasi — dan pemilik toko berhak tahu itu akan terjadi.
    pesan.push(
        "Di GALAXYAS POS 1 stok adalah angka berjalan dan riwayat mutasinya tidak selalu \
         menjumlah jadi angka itu. GALAXYAS POS 2 akan menutup selisihnya dengan satu \
         baris opname bertanggal migrasi per barang; stoknya sendiri tidak berubah."
            .into(),
    );

    pesan
}

fn ambil_settings(conn: &Connection) -> AppResult<Vec<Value>> {
    let semua = kumpulkan(
        conn,
        "SELECT key, COALESCE(value,'') FROM settings ORDER BY key",
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    )?;
    Ok(semua
        .into_iter()
        .filter(|(k, _)| setelan_ikut(k))
        .map(|(k, v)| json!({ "key": k, "value": v }))
        .collect())
}

fn ambil_users(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, username, name, role, pin, permissions FROM users",
        |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "username": r.get::<_, String>(1)?,
                "name": r.get::<_, String>(2)?,
                "role": r.get::<_, String>(3)?,
                // PIN apa adanya. Di sisi POS 2 ia langsung di-hash Argon2id dan
                // teks aslinya tidak pernah tersimpan di sana (MIGRASI.md §4.3).
                "pin": r.get::<_, Option<String>>(4)?,
                "pin_hash": Value::Null,
                "permissions": r.get::<_, String>(5)?,
                "is_active": true,
                "created_at": Value::Null,
                "updated_at": Value::Null,
            }))
        },
    )
}

fn ambil_brands(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(conn, "SELECT id, name, updated_at FROM brands", |r| {
        let t: String = r.get(2)?;
        Ok(json!({
            "id": r.get::<_, String>(0)?,
            "name": r.get::<_, String>(1)?,
            "is_deleted": false,
            "created_at": t,
            "updated_at": t,
        }))
    })
}

fn ambil_customers(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, name, phone, email, address, note, is_active, updated_at FROM customers",
        |r| {
            let t: String = r.get(7)?;
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "name": r.get::<_, String>(1)?,
                "phone": r.get::<_, Option<String>>(2)?,
                "email": r.get::<_, Option<String>>(3)?,
                "address": r.get::<_, Option<String>>(4)?,
                "note": r.get::<_, Option<String>>(5)?,
                "is_active": r.get::<_, i64>(6)? != 0,
                "is_deleted": false,
                "created_at": t,
                "updated_at": t,
            }))
        },
    )
}

fn ambil_products(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, name, barcode, category, brand, unit, sell_price, cost_price,
                default_discount, is_active, is_deleted, updated_at, dirty, ever_synced
         FROM products",
        |r| {
            let t: String = r.get(11)?;
            Ok(json!({
                // ID DIPERTAHANKAN APA ADANYA — paku yang menahan sync ke SSoT.
                "id": r.get::<_, String>(0)?,
                "name": r.get::<_, String>(1)?,
                "barcode": r.get::<_, Option<String>>(2)?,
                "category": r.get::<_, Option<String>>(3)?,
                "brand": r.get::<_, Option<String>>(4)?,
                "unit": r.get::<_, Option<String>>(5)?,
                "sell_price": ke_rupiah(r.get::<_, f64>(6)?),
                "cost_price": ke_rupiah(r.get::<_, f64>(7)?),
                "default_discount": ke_rupiah(r.get::<_, f64>(8)?),
                "is_active": r.get::<_, i64>(9)? != 0,
                "is_deleted": r.get::<_, i64>(10)? != 0,
                "updated_at": t,
                "dirty": r.get::<_, i64>(12)? != 0,
                "ever_synced": r.get::<_, i64>(13)? != 0,
                "created_at": t,
            }))
        },
    )
}

fn ambil_stock(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(conn, "SELECT product_id, qty, updated_at FROM stock", |r| {
        Ok(json!({
            "product_id": r.get::<_, String>(0)?,
            "qty_milli": ke_mili(r.get::<_, f64>(1)?),
            "updated_at": r.get::<_, String>(2)?,
        }))
    })
}

fn ambil_discounts(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, code, scope, target, target_label, discount_type, value, days,
                priority, is_active, updated_at
         FROM discount_periods",
        |r| {
            let jenis: String = r.get(5)?;
            let mentah: f64 = r.get(6)?;
            // Persen jadi basis poin (1 % = 100) supaya persen pun tetap
            // integer dan tidak menyeret pecahan ke jalur uang.
            let nilai = if jenis == "percent" {
                (mentah * 100.0).round() as i64
            } else {
                ke_rupiah(mentah)
            };
            let t: String = r.get(10)?;
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "code": r.get::<_, String>(1)?,
                "scope": r.get::<_, String>(2)?,
                "target": r.get::<_, String>(3)?,
                "target_label": r.get::<_, Option<String>>(4)?,
                "discount_type": jenis,
                "value": nilai,
                "days": r.get::<_, String>(7)?,
                "priority": r.get::<_, i64>(8)?,
                "is_active": r.get::<_, i64>(9)? != 0,
                "is_deleted": false,
                "created_at": t,
                "updated_at": t,
            }))
        },
    )
}

fn ambil_shifts(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, user_id, user_name, opening_cash, closing_cash, expected_cash,
                difference, note, opened_at, closed_at
         FROM shifts",
        |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "user_id": r.get::<_, String>(1)?,
                "user_name": r.get::<_, String>(2)?,
                "opening_cash": ke_rupiah(r.get::<_, f64>(3)?),
                // Kosong tetap kosong, bukan nol: kosong berarti "shift belum
                // ditutup", nol berarti "sudah dihitung dan uangnya memang nol".
                "closing_cash": r.get::<_, Option<f64>>(4)?.map(ke_rupiah),
                "expected_cash": r.get::<_, Option<f64>>(5)?.map(ke_rupiah),
                "difference": r.get::<_, Option<f64>>(6)?.map(ke_rupiah),
                "note": r.get::<_, Option<String>>(7)?,
                "opened_at": r.get::<_, String>(8)?,
                "closed_at": r.get::<_, Option<String>>(9)?,
            }))
        },
    )
}

fn ambil_sales(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, invoice_no, cashier_id, customer_id, shift_id, subtotal, discount,
                total, paid, change, payment_method, paid_cash, paid_qris, client_ref,
                created_at
         FROM transactions",
        |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "invoice_no": r.get::<_, String>(1)?,
                "cashier_id": r.get::<_, String>(2)?,
                "customer_id": r.get::<_, Option<String>>(3)?,
                "shift_id": r.get::<_, Option<String>>(4)?,
                "subtotal": ke_rupiah(r.get::<_, f64>(5)?),
                "discount": ke_rupiah(r.get::<_, f64>(6)?),
                "total": ke_rupiah(r.get::<_, f64>(7)?),
                "paid": ke_rupiah(r.get::<_, f64>(8)?),
                "change": ke_rupiah(r.get::<_, f64>(9)?),
                "payment_method": r.get::<_, String>(10)?,
                "paid_cash": r.get::<_, Option<f64>>(11)?.map(ke_rupiah),
                "paid_qris": r.get::<_, Option<f64>>(12)?.map(ke_rupiah),
                "client_ref": r.get::<_, Option<String>>(13)?,
                // Aplikasi ini tidak punya konsep pembatalan: mengoreksi struk
                // berarti menimpanya. Keempat kolom ini karena itu selalu kosong
                // dari sini.
                "voided_at": Value::Null,
                "voided_by": Value::Null,
                "void_reason": Value::Null,
                "replaces_id": Value::Null,
                "created_at": r.get::<_, String>(14)?,
            }))
        },
    )
}

fn ambil_sale_items(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, transaction_id, product_id, name, price, qty, discount, line_total
         FROM transaction_items ORDER BY id",
        |r| {
            Ok(json!({
                // Angka autoincrement, bukan UUID. POS 2 menurunkannya jadi UUID
                // dengan rumus tetap (MIGRASI.md §5.3).
                "id": r.get::<_, i64>(0)?,
                "sale_id": r.get::<_, String>(1)?,
                "product_id": r.get::<_, String>(2)?,
                "name": r.get::<_, String>(3)?,
                "price": ke_rupiah(r.get::<_, f64>(4)?),
                "qty_milli": ke_mili(r.get::<_, f64>(5)?),
                "discount": ke_rupiah(r.get::<_, f64>(6)?),
                "line_total": ke_rupiah(r.get::<_, f64>(7)?),
                "line_no": Value::Null,
            }))
        },
    )
}

fn ambil_batches(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, no, kind, note, user_id, created_at FROM stock_movement_batches",
        |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "no": r.get::<_, String>(1)?,
                "kind": r.get::<_, String>(2)?,
                "note": r.get::<_, Option<String>>(3)?,
                "user_id": r.get::<_, Option<String>>(4)?,
                "created_at": r.get::<_, String>(5)?,
                "voided_at": Value::Null,
                "voided_by": Value::Null,
                "void_reason": Value::Null,
                "replaces_id": Value::Null,
            }))
        },
    )
}

fn ambil_movements(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, product_id, kind, qty, stock_after, note, user_id, batch_id, created_at
         FROM stock_movements ORDER BY id",
        |r| {
            let kind: String = r.get(2)?;
            let mili = ke_mili(r.get::<_, f64>(3)?);
            Ok(json!({
                "id": r.get::<_, i64>(0)?,
                "product_id": r.get::<_, String>(1)?,
                "kind": kind.clone(),
                "qty_milli": ke_delta(&kind, mili),
                "qty_after_milli": ke_mili(r.get::<_, f64>(4)?),
                "note": r.get::<_, Option<String>>(5)?,
                "user_id": r.get::<_, Option<String>>(6)?,
                "batch_id": r.get::<_, Option<String>>(7)?,
                // Tautan mutasi ke struknya tidak pernah ada di sini.
                "sale_id": Value::Null,
                "created_at": r.get::<_, String>(8)?,
            }))
        },
    )
}

fn ambil_expenses(conn: &Connection) -> AppResult<Vec<Value>> {
    kumpulkan(
        conn,
        "SELECT id, date, category, amount, note, user_id, created_at FROM expenses",
        |r| {
            Ok(json!({
                "id": r.get::<_, String>(0)?,
                "date": r.get::<_, String>(1)?,
                "category": r.get::<_, String>(2)?,
                "amount": ke_rupiah(r.get::<_, f64>(3)?),
                "note": r.get::<_, Option<String>>(4)?,
                "user_id": r.get::<_, Option<String>>(5)?,
                "created_at": r.get::<_, String>(6)?,
            }))
        },
    )
}

// ---------------------------------------------------------------------------
// Impor
// ---------------------------------------------------------------------------

/// Salin berkas database sebelum apa pun dihapus.
///
/// Yang menahan "salah pilih berkas lalu kehilangan setahun riwayat" bukan
/// dialog konfirmasi di layar, melainkan salinan yang sudah ada di disk sebelum
/// baris pertama dihapus. Karena itu gagal menyalin berarti **batal**, bukan
/// diteruskan.
fn cadangkan(db_path: &Path) -> AppResult<Option<PathBuf>> {
    if !db_path.exists() {
        return Ok(None);
    }
    let stempel = Utc::now().format("%Y%m%d-%H%M%S");
    let nama = db_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("galaxyas.sqlite");
    let tujuan = db_path.with_file_name(format!("{nama}.sebelum-migrasi-{stempel}.bak"));
    fs::copy(db_path, &tujuan).map_err(|e| {
        AppError::Other(format!(
            "Gagal membuat cadangan data toko ({e}). Impor dibatalkan — tidak ada satu \
             baris pun yang diubah."
        ))
    })?;
    Ok(Some(tujuan))
}

fn daftar<'a>(bundel: &'a Value, nama: &str) -> &'a [Value] {
    bundel
        .get("tabel")
        .and_then(|t| t.get(nama))
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

/// Urutan pengosongan: anak dulu, induk belakangan.
const URUTAN_HAPUS: &[&str] = &[
    "expenses",
    "stock_movements",
    "stock_movement_batches",
    "transaction_items",
    "transactions",
    "shifts",
    "discount_periods",
    "stock",
    "products",
    "customers",
    "brands",
    "users",
];

/// Ganti seluruh isi toko dengan isi berkas `.gpos`.
pub fn impor(conn: &mut Connection, db_path: &Path, data: &[u8]) -> AppResult<HasilMigrasi> {
    // Berkas dibuka & diperiksa **sebelum** apa pun disentuh: yang salah pilih
    // atau rusak ditolak selagi data lama masih utuh.
    let bundel = buka(data)?;
    let sumber = baca_sumber(bundel.get("sumber").unwrap_or(&Value::Null));
    if sumber.app.is_empty() {
        return Err(AppError::Other(
            "Berkas migrasi tidak menyebutkan aplikasi asalnya — kemungkinan dibuat versi \
             yang terlalu lama."
                .into(),
        ));
    }

    let cadangan = cadangkan(db_path)?;

    let mut peringatan = Vec::new();
    let mut dilewati: std::collections::BTreeMap<String, i64> = Default::default();
    let mut baris: std::collections::BTreeMap<String, i64> = Default::default();

    let tx = conn.transaction()?;

    for tabel in URUTAN_HAPUS {
        tx.execute(&format!("DELETE FROM {tabel}"), [])?;
    }
    // `store_id` milik PC ini, bukan milik berkas: ia identitas toko ini
    // terhadap server sinkronisasi. Kalau ikut diganti, toko ini berpindah
    // identitas dan sync berikutnya menganggapnya toko lain.
    tx.execute("DELETE FROM settings WHERE key <> 'store_id'", [])?;

    baris.insert("settings".into(), sisip_settings(&tx, &bundel)?);
    baris.insert("users".into(), sisip_users(&tx, &bundel, &mut peringatan)?);
    baris.insert("brands".into(), sisip_brands(&tx, &bundel, &mut dilewati)?);
    baris.insert(
        "customers".into(),
        sisip_customers(&tx, &bundel, &mut dilewati)?,
    );
    baris.insert("products".into(), sisip_products(&tx, &bundel)?);
    baris.insert("stock".into(), sisip_stock(&tx, &bundel)?);
    baris.insert(
        "discount_periods".into(),
        sisip_discounts(&tx, &bundel, &mut dilewati)?,
    );
    baris.insert("shifts".into(), sisip_shifts(&tx, &bundel)?);

    let batal = kumpulkan_batal(&bundel);
    baris.insert(
        "sales".into(),
        sisip_sales(&tx, &bundel, &batal, &mut dilewati)?,
    );
    baris.insert("sale_items".into(), sisip_sale_items(&tx, &bundel, &batal)?);

    let batch_batal = kumpulkan_batch_batal(&bundel);
    baris.insert(
        "stock_batches".into(),
        sisip_batches(&tx, &bundel, &batch_batal, &mut dilewati)?,
    );
    baris.insert(
        "stock_movements".into(),
        sisip_movements(&tx, &bundel, &batal, &batch_batal, &mut dilewati)?,
    );
    baris.insert("expenses".into(), sisip_expenses(&tx, &bundel)?);

    simpan_lampiran(&tx, &bundel, &mut peringatan)?;

    tx.commit()?;

    laporkan(&dilewati, &mut peringatan);
    if sumber.app == APP_V2 {
        peringatan.push(
            "Berkas ini dari GALAXYAS POS 2. PIN kasir tidak bisa ikut pindah ke sini \
             (POS 2 menyimpannya sebagai hash), jadi tiap kasir harus menyetel PIN-nya \
             lagi lewat Hak Akses."
                .into(),
        );
    }

    Ok(HasilMigrasi {
        berkas: String::new(),
        ukuran: data.len() as i64,
        sumber,
        baris,
        dilewati,
        peringatan,
        cadangan: cadangan.map(|p| p.to_string_lossy().into_owned()),
    })
}

/// Id penjualan yang dibatalkan di POS 2 — **tidak** diimpor ke sini.
///
/// Aplikasi ini tidak punya konsep pembatalan: kalau struk batal ikut masuk, ia
/// jadi penjualan sungguhan dan omzet melar tanpa ada yang bisa membedakannya.
/// Baris item dan mutasi stoknya ikut dilewati supaya buku besarnya tetap
/// sepadan (MIGRASI.md §5.4).
fn kumpulkan_batal(bundel: &Value) -> HashSet<String> {
    id_yang_dibatalkan(daftar(bundel, "sales"))
}

fn kumpulkan_batch_batal(bundel: &Value) -> HashSet<String> {
    id_yang_dibatalkan(daftar(bundel, "stock_batches"))
}

/// Id yang `voided_at`-nya terisi. **Id kosong tidak pernah ikut**, dan itu
/// bukan kehati-hatian berlebihan: himpunan ini juga dipakai menyaring mutasi
/// lewat `sale_id`/`batch_id` yang kosongnya diwakili `""`. Satu baris cacat
/// beruas id kosong akan membuat seluruh mutasi yang memang tidak menempel pada
/// struk mana pun ikut terbuang — dan stoknya melenceng tanpa satu pun galat.
fn id_yang_dibatalkan(baris: &[Value]) -> HashSet<String> {
    baris
        .iter()
        .filter(|r| r.get("voided_at").map(|v| !v.is_null()).unwrap_or(false))
        .map(|r| teks(r, "id"))
        .filter(|id| !id.is_empty())
        .collect()
}

fn sisip_settings(tx: &rusqlite::Transaction<'_>, bundel: &Value) -> AppResult<i64> {
    let mut n = 0;
    for r in daftar(bundel, "settings") {
        let k = teks(r, "key");
        if k.is_empty() || !setelan_ikut(&k) || k == "store_id" {
            continue;
        }
        tx.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![k, teks(r, "value")],
        )?;
        n += 1;
    }
    // Watermark dikosongkan, bukan sekadar tidak ikut: tarikan pertama PC ini
    // ke server sinkronisasi harus penuh, dan kunci yang absen tidak menyatakan
    // itu.
    tx.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES ('last_pull_at', '')",
        [],
    )?;
    Ok(n)
}

fn sisip_users(
    tx: &rusqlite::Transaction<'_>,
    bundel: &Value,
    peringatan: &mut Vec<String>,
) -> AppResult<i64> {
    let mut n = 0;
    let mut tanpa_pin = 0;
    for r in daftar(bundel, "users") {
        // Hash Argon2id dari POS 2 tidak bisa dikembalikan jadi PIN, jadi yang
        // datang dari sana masuk **tanpa PIN**. Jumlahnya wajib dilaporkan —
        // kalau tidak, pemilik toko baru tahu besok pagi saat kasir tidak bisa
        // masuk (MIGRASI.md §4.3).
        let pin = teks_opsional(r, "pin");
        if pin.is_none() {
            tanpa_pin += 1;
        }
        tx.execute(
            "INSERT OR REPLACE INTO users (id, username, name, role, pin, permissions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                teks(r, "id"),
                teks(r, "username"),
                teks(r, "name"),
                if teks(r, "role").is_empty() {
                    "kasir".to_string()
                } else {
                    teks(r, "role")
                },
                pin,
                if teks(r, "permissions").is_empty() {
                    "[]".to_string()
                } else {
                    teks(r, "permissions")
                },
            ],
        )?;
        n += 1;
    }
    if tanpa_pin > 0 {
        peringatan.push(format!(
            "{tanpa_pin} pengguna masuk tanpa PIN dan tidak bisa login sampai PIN-nya \
             disetel di Pengaturan → Hak Akses."
        ));
    }
    Ok(n)
}

fn sisip_brands(
    tx: &rusqlite::Transaction<'_>,
    bundel: &Value,
    dilewati: &mut std::collections::BTreeMap<String, i64>,
) -> AppResult<i64> {
    let mut n = 0;
    let mut nisan = 0;
    for r in daftar(bundel, "brands") {
        // Aplikasi ini tidak punya nisan untuk merek — yang terhapus di POS 2
        // memang tidak punya tempat di sini.
        if bendera(r, "is_deleted", false) {
            nisan += 1;
            continue;
        }
        tx.execute(
            "INSERT OR REPLACE INTO brands (id, name, updated_at) VALUES (?1, ?2, ?3)",
            params![teks(r, "id"), teks(r, "name"), waktu(r, "updated_at")],
        )?;
        n += 1;
    }
    if nisan > 0 {
        dilewati.insert("merek_terhapus".into(), nisan);
    }
    Ok(n)
}

fn sisip_customers(
    tx: &rusqlite::Transaction<'_>,
    bundel: &Value,
    dilewati: &mut std::collections::BTreeMap<String, i64>,
) -> AppResult<i64> {
    let mut n = 0;
    let mut nisan = 0;
    for r in daftar(bundel, "customers") {
        if bendera(r, "is_deleted", false) {
            nisan += 1;
            continue;
        }
        tx.execute(
            "INSERT OR REPLACE INTO customers
                (id, name, phone, email, address, note, is_active, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![
                teks(r, "id"),
                teks(r, "name"),
                teks_opsional(r, "phone"),
                teks_opsional(r, "email"),
                teks_opsional(r, "address"),
                teks_opsional(r, "note"),
                i64::from(bendera(r, "is_active", true)),
                waktu(r, "updated_at"),
            ],
        )?;
        n += 1;
    }
    if nisan > 0 {
        dilewati.insert("pelanggan_terhapus".into(), nisan);
    }
    Ok(n)
}

fn sisip_products(tx: &rusqlite::Transaction<'_>, bundel: &Value) -> AppResult<i64> {
    let mut n = 0;
    for r in daftar(bundel, "products") {
        tx.execute(
            "INSERT OR REPLACE INTO products
                (id, name, barcode, category, brand, unit, sell_price, cost_price,
                 default_discount, is_active, is_deleted, updated_at, dirty, ever_synced)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
            params![
                // Sekali lagi, dan ini yang paling menentukan: id barang tidak
                // pernah diturunkan ulang di arah mana pun.
                teks(r, "id"),
                teks(r, "name"),
                teks_opsional(r, "barcode"),
                teks_opsional(r, "category"),
                teks_opsional(r, "brand"),
                teks_opsional(r, "unit"),
                angka(r, "sell_price") as f64,
                angka(r, "cost_price") as f64,
                angka(r, "default_discount") as f64,
                i64::from(bendera(r, "is_active", true)),
                i64::from(bendera(r, "is_deleted", false)),
                waktu(r, "updated_at"),
                i64::from(bendera(r, "dirty", false)),
                i64::from(bendera(r, "ever_synced", false)),
            ],
        )?;
        n += 1;
    }
    Ok(n)
}

fn sisip_stock(tx: &rusqlite::Transaction<'_>, bundel: &Value) -> AppResult<i64> {
    let mut n = 0;
    for r in daftar(bundel, "stock") {
        tx.execute(
            "INSERT OR REPLACE INTO stock (product_id, qty, updated_at) VALUES (?1, ?2, ?3)",
            params![
                teks(r, "product_id"),
                dari_mili(angka(r, "qty_milli")),
                waktu(r, "updated_at")
            ],
        )?;
        n += 1;
    }
    Ok(n)
}

fn sisip_discounts(
    tx: &rusqlite::Transaction<'_>,
    bundel: &Value,
    dilewati: &mut std::collections::BTreeMap<String, i64>,
) -> AppResult<i64> {
    let mut n = 0;
    let mut nisan = 0;
    for r in daftar(bundel, "discount_periods") {
        if bendera(r, "is_deleted", false) {
            nisan += 1;
            continue;
        }
        let jenis = teks(r, "discount_type");
        let mentah = angka(r, "value");
        // Kebalikan persis dari `ambil_discounts`: basis poin balik jadi persen.
        let nilai = if jenis == "percent" {
            mentah as f64 / 100.0
        } else {
            mentah as f64
        };
        tx.execute(
            "INSERT OR REPLACE INTO discount_periods
                (id, scope, target, target_label, discount_type, value, days, is_active,
                 updated_at, code, priority)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)",
            params![
                teks(r, "id"),
                teks(r, "scope"),
                teks(r, "target"),
                teks_opsional(r, "target_label"),
                jenis,
                nilai,
                teks(r, "days"),
                i64::from(bendera(r, "is_active", true)),
                waktu(r, "updated_at"),
                teks(r, "code"),
                angka(r, "priority").max(1),
            ],
        )?;
        n += 1;
    }
    if nisan > 0 {
        dilewati.insert("diskon_terhapus".into(), nisan);
    }
    Ok(n)
}

fn sisip_shifts(tx: &rusqlite::Transaction<'_>, bundel: &Value) -> AppResult<i64> {
    let mut n = 0;
    for r in daftar(bundel, "shifts") {
        tx.execute(
            "INSERT OR REPLACE INTO shifts
                (id, user_id, user_name, opening_cash, closing_cash, expected_cash,
                 difference, note, opened_at, closed_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![
                teks(r, "id"),
                teks(r, "user_id"),
                teks(r, "user_name"),
                angka(r, "opening_cash") as f64,
                angka_opsional(r, "closing_cash").map(|v| v as f64),
                angka_opsional(r, "expected_cash").map(|v| v as f64),
                angka_opsional(r, "difference").map(|v| v as f64),
                teks_opsional(r, "note"),
                waktu(r, "opened_at"),
                teks_opsional(r, "closed_at"),
            ],
        )?;
        n += 1;
    }
    Ok(n)
}

fn sisip_sales(
    tx: &rusqlite::Transaction<'_>,
    bundel: &Value,
    batal: &HashSet<String>,
    dilewati: &mut std::collections::BTreeMap<String, i64>,
) -> AppResult<i64> {
    let mut n = 0;
    for r in daftar(bundel, "sales") {
        let id = teks(r, "id");
        if batal.contains(&id) {
            continue;
        }
        tx.execute(
            "INSERT OR REPLACE INTO transactions
                (id, invoice_no, cashier_id, subtotal, discount, total, paid, change,
                 payment_method, created_at, customer_id, shift_id, paid_cash, paid_qris,
                 client_ref)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)",
            params![
                id,
                teks(r, "invoice_no"),
                teks(r, "cashier_id"),
                angka(r, "subtotal") as f64,
                angka(r, "discount") as f64,
                angka(r, "total") as f64,
                angka(r, "paid") as f64,
                angka(r, "change") as f64,
                teks(r, "payment_method"),
                waktu(r, "created_at"),
                teks_opsional(r, "customer_id"),
                teks_opsional(r, "shift_id"),
                angka_opsional(r, "paid_cash").map(|v| v as f64),
                angka_opsional(r, "paid_qris").map(|v| v as f64),
                teks_opsional(r, "client_ref"),
            ],
        )?;
        n += 1;
    }
    if !batal.is_empty() {
        dilewati.insert("penjualan_dibatalkan".into(), batal.len() as i64);
    }
    Ok(n)
}

fn sisip_sale_items(
    tx: &rusqlite::Transaction<'_>,
    bundel: &Value,
    batal: &HashSet<String>,
) -> AppResult<i64> {
    let mut n = 0;
    for r in daftar(bundel, "sale_items") {
        let sale = teks(r, "sale_id");
        if batal.contains(&sale) {
            continue;
        }
        // `id` dari bundel sengaja **diabaikan**: kolomnya di sini
        // autoincrement, dan impor yang mengganti seluruh isi tidak punya baris
        // lama yang bisa bertabrakan.
        tx.execute(
            "INSERT INTO transaction_items
                (transaction_id, product_id, name, price, qty, discount, line_total)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                sale,
                teks(r, "product_id"),
                teks(r, "name"),
                angka(r, "price") as f64,
                dari_mili(angka(r, "qty_milli")),
                angka(r, "discount") as f64,
                angka(r, "line_total") as f64,
            ],
        )?;
        n += 1;
    }
    Ok(n)
}

fn sisip_batches(
    tx: &rusqlite::Transaction<'_>,
    bundel: &Value,
    batal: &HashSet<String>,
    dilewati: &mut std::collections::BTreeMap<String, i64>,
) -> AppResult<i64> {
    let mut n = 0;
    for r in daftar(bundel, "stock_batches") {
        let id = teks(r, "id");
        if batal.contains(&id) {
            continue;
        }
        tx.execute(
            "INSERT OR REPLACE INTO stock_movement_batches
                (id, no, kind, note, user_id, created_at)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![
                id,
                teks(r, "no"),
                teks(r, "kind"),
                teks_opsional(r, "note"),
                teks_opsional(r, "user_id"),
                waktu(r, "created_at"),
            ],
        )?;
        n += 1;
    }
    if !batal.is_empty() {
        dilewati.insert("nota_dibatalkan".into(), batal.len() as i64);
    }
    Ok(n)
}

fn sisip_movements(
    tx: &rusqlite::Transaction<'_>,
    bundel: &Value,
    sale_batal: &HashSet<String>,
    batch_batal: &HashSet<String>,
    dilewati: &mut std::collections::BTreeMap<String, i64>,
) -> AppResult<i64> {
    let mut n = 0;
    let mut lewat = 0;
    for r in daftar(bundel, "stock_movements") {
        let kind = teks(r, "kind");

        // Mutasi milik struk/nota yang dibatalkan ikut dilewati, dan mutasi
        // berjenis `void` sama sekali tidak punya arti di sini. Kalau salah
        // satunya ikut masuk sementara pasangannya tidak, stoknya melenceng.
        let sale = teks_opsional(r, "sale_id").unwrap_or_default();
        let batch = teks_opsional(r, "batch_id").unwrap_or_default();
        if kind == "void" || sale_batal.contains(&sale) || batch_batal.contains(&batch) {
            lewat += 1;
            continue;
        }

        tx.execute(
            "INSERT INTO stock_movements
                (product_id, kind, qty, stock_after, note, user_id, created_at, batch_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                teks(r, "product_id"),
                kind,
                // Kebalikan `ke_delta`: di sini besaran disimpan tanpa tanda dan
                // arahnya tersirat di `kind`.
                dari_mili(angka(r, "qty_milli").abs()),
                dari_mili(angka(r, "qty_after_milli")),
                teks_opsional(r, "note"),
                teks_opsional(r, "user_id"),
                waktu(r, "created_at"),
                if batch.is_empty() { None } else { Some(batch) },
            ],
        )?;
        n += 1;
    }
    if lewat > 0 {
        dilewati.insert("mutasi_dibatalkan".into(), lewat);
    }
    Ok(n)
}

fn sisip_expenses(tx: &rusqlite::Transaction<'_>, bundel: &Value) -> AppResult<i64> {
    let mut n = 0;
    for r in daftar(bundel, "expenses") {
        tx.execute(
            "INSERT OR REPLACE INTO expenses
                (id, date, category, amount, note, user_id, created_at)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![
                teks(r, "id"),
                teks(r, "date"),
                teks(r, "category"),
                angka(r, "amount") as f64,
                teks_opsional(r, "note"),
                teks_opsional(r, "user_id"),
                waktu(r, "created_at"),
            ],
        )?;
        n += 1;
    }
    Ok(n)
}

/// Simpan `lampiran` apa adanya walau isinya tidak dimengerti di sini (§4.5).
///
/// Isinya template laporan POS 2. Membuangnya akan membuat perjalanan
/// POS 2 → v1 → POS 2 menghapus template yang tidak pernah kelihatan di layar
/// mana pun di aplikasi ini — kerusakan yang mustahil ditelusuri pemakainya.
fn simpan_lampiran(
    tx: &rusqlite::Transaction<'_>,
    bundel: &Value,
    peringatan: &mut Vec<String>,
) -> AppResult<()> {
    let lampiran = bundel.get("lampiran").cloned().unwrap_or(Value::Null);
    let kosong = match &lampiran {
        Value::Object(m) => m.is_empty(),
        _ => true,
    };
    if kosong {
        return Ok(());
    }

    let isi = serde_json::to_string(&lampiran).map_err(json_err)?;
    tx.execute(
        "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
        params![KUNCI_LAMPIRAN, isi],
    )?;
    peringatan.push(
        "Berkas ini membawa pengaturan milik GALAXYAS POS 2 (mis. template laporan) yang \
         tidak dipakai di sini. Isinya disimpan utuh dan ikut lagi kalau data ini nanti \
         dikirim balik ke sana."
            .into(),
    );
    Ok(())
}

fn waktu(r: &Value, kunci: &str) -> String {
    let t = teks(r, kunci);
    if t.trim().is_empty() {
        "1970-01-01T00:00:00+00:00".to_string()
    } else {
        t
    }
}

fn laporkan(dilewati: &std::collections::BTreeMap<String, i64>, peringatan: &mut Vec<String>) {
    for (kunci, n) in dilewati {
        if *n == 0 {
            continue;
        }
        let pesan = match kunci.as_str() {
            "penjualan_dibatalkan" => format!(
                "{n} penjualan yang dibatalkan di GALAXYAS POS 2 tidak diimpor — aplikasi \
                 ini tidak punya konsep pembatalan, dan kalau ikut masuk ia akan terhitung \
                 sebagai penjualan sungguhan."
            ),
            "nota_dibatalkan" => {
                format!("{n} nota stok yang dibatalkan tidak diimpor, dengan alasan yang sama.")
            }
            "mutasi_dibatalkan" => format!(
                "{n} mutasi stok milik struk/nota yang dibatalkan ikut dilewati supaya \
                 angka stoknya tetap sepadan."
            ),
            "merek_terhapus" => format!("{n} merek yang sudah dihapus tidak diimpor."),
            "pelanggan_terhapus" => format!("{n} pelanggan yang sudah dihapus tidak diimpor."),
            "diskon_terhapus" => format!("{n} diskon periodik yang sudah dihapus tidak diimpor."),
            _ => continue,
        };
        peringatan.push(pesan);
    }
}

// ---------------------------------------------------------------------------
// Yang dipanggil frontend
// ---------------------------------------------------------------------------

/// Asal-usul berkas **tanpa mengimpornya**. Isinya tidak didekripsi di sini.
pub fn periksa(data: &[u8]) -> AppResult<SumberBerkas> {
    Ok(baca_sumber(&baca_kepala_mentah(data)?))
}

/// Nama berkas yang diusulkan ke dialog "Simpan".
pub fn nama_berkas(toko_nama: &str) -> String {
    let aman: String = toko_nama
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let aman = aman.trim();
    let nama = if aman.is_empty() { "Toko" } else { aman };
    format!("{nama} {}.gpos", Utc::now().format("%Y-%m-%d"))
}

#[cfg(test)]
mod uji {
    use super::*;

    /// Berkas contoh yang **dibuat GALAXYAS POS 2**, dibekukan di repo ini.
    ///
    /// Salinannya sendiri, bukan tautan ke repo sebelah — dua repo tidak
    /// berbagi berkas, sama seperti fixture golden ESC/POS. Yang menyegarkannya
    /// `tools/terbitkan-fixture-migrasi.py` di repo `Gpos2`, dan ia menulis ke
    /// dua tempat sekaligus.
    ///
    /// Kalau uji ini gagal, format berkas berubah tanpa sisi ini ikut diberi
    /// tahu — dan gejalanya di toko adalah "berkas migrasi rusak" untuk berkas
    /// yang sebenarnya baik-baik saja.
    const FIXTURE: &[u8] = include_bytes!("../fixtures/migrasi-contoh.gpos");

    #[test]
    fn bolak_balik() {
        let kepala = json!({ "app": "gpos1", "toko": { "id": "x", "nama": "Toko" } });
        let bundel = json!({ "format": 1, "tabel": { "products": [] } });

        let data = bungkus(&kepala, &bundel).unwrap();

        assert_eq!(baca_kepala_mentah(&data).unwrap(), kepala);
        assert_eq!(buka(&data).unwrap(), bundel);
    }

    #[test]
    fn isi_benar_benar_terenkripsi() {
        let data = bungkus(
            &json!({}),
            &json!({ "tabel": { "users": [{ "name": "Budi Santoso" }] } }),
        )
        .unwrap();

        assert!(!data.windows(12).any(|w| w == b"Budi Santoso"));
    }

    #[test]
    fn berkas_asing_ditolak() {
        let galat = buka(&[
            b'P', b'K', 3, 4, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ])
        .unwrap_err();

        assert!(galat.to_string().contains("bukan berkas migrasi"));
    }

    #[test]
    fn versi_lebih_baru_dibedakan_dari_rusak() {
        let mut data = bungkus(&json!({}), &json!({})).unwrap();
        data[8..10].copy_from_slice(&99u16.to_le_bytes());

        assert!(buka(&data).unwrap_err().to_string().contains("Perbarui"));
    }

    #[test]
    fn satu_byte_berubah_langsung_ketahuan() {
        let mut data = bungkus(&json!({}), &json!({ "a": 1 })).unwrap();
        let n = data.len();
        data[n - 20] ^= 1;

        assert!(buka(&data)
            .unwrap_err()
            .to_string()
            .contains("rusak atau sudah diubah"));
    }

    #[test]
    fn tambahan_di_ujung_ditolak() {
        let mut data = bungkus(&json!({}), &json!({ "a": 1 })).unwrap();
        data.extend_from_slice(b"tempelan");

        assert!(buka(&data)
            .unwrap_err()
            .to_string()
            .contains("tambahan di ujungnya"));
    }

    #[test]
    fn berkas_dari_pos2_bisa_dibuka() {
        let sumber = periksa(FIXTURE).unwrap();

        assert_eq!(sumber.app, APP_V2);
        assert_eq!(sumber.toko_nama, "Toko Contoh");
        assert_eq!(sumber.omzet_total, 15000);
        assert_eq!(sumber.jumlah.get("products"), Some(&1));
    }

    #[test]
    fn isi_berkas_dari_pos2_terbaca_utuh() {
        let bundel = buka(FIXTURE).unwrap();
        let produk = &daftar(&bundel, "products")[0];

        assert_eq!(teks(produk, "name"), "Kopi Bubuk");
        assert_eq!(angka(produk, "sell_price"), 15000);

        // Mutasi keluar bertanda negatif di bundel; impor ke sini membalikkannya
        // jadi besaran tanpa tanda.
        let keluar: Vec<_> = daftar(&bundel, "stock_movements")
            .iter()
            .filter(|m| teks(m, "kind") == "out")
            .collect();
        assert!(!keluar.is_empty());
        assert!(keluar.iter().all(|m| angka(m, "qty_milli") < 0));
    }

    fn db_uji() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        crate::db::init_schema(&conn).unwrap();
        conn
    }

    #[test]
    fn impor_dari_pos2_lalu_ekspor_lagi_tidak_kehilangan_angka() {
        let mut conn = db_uji();
        let hasil = impor(&mut conn, Path::new("tidak-ada.sqlite"), FIXTURE).unwrap();

        assert_eq!(hasil.baris.get("products"), Some(&1));
        assert_eq!(hasil.baris.get("sales"), Some(&1));

        let stok: f64 = conn
            .query_row("SELECT qty FROM stock", [], |r| r.get(0))
            .unwrap();
        assert!((stok - 9.0).abs() < 1e-9, "stok jadi {stok}, harusnya 9");

        let (data, _) = ekspor(&conn, "Toko Contoh").unwrap();
        let ulang = buka(&data).unwrap();
        let produk = &daftar(&ulang, "products")[0];
        assert_eq!(angka(produk, "sell_price"), 15000);
        assert_eq!(angka(&daftar(&ulang, "stock")[0], "qty_milli"), 9000);
    }

    #[test]
    fn penjualan_yang_dibatalkan_tidak_pernah_masuk() {
        let mut conn = db_uji();
        let kepala = json!({
            "app": APP_V2, "versi_app": "1.0.0", "versi_bundel": 1,
            "dibuat": "2026-08-07T00:00:00+00:00",
            "toko": { "id": "t", "nama": "T" }, "jumlah": {}, "ringkas": {}, "catatan": []
        });
        let bundel = json!({
            "format": 1, "sumber": kepala, "lampiran": {},
            "tabel": {
                "sales": [
                    { "id": "hidup", "invoice_no": "INV-1", "cashier_id": "u1",
                      "total": 15000, "payment_method": "tunai",
                      "created_at": "2026-08-07T00:00:00+00:00" },
                    { "id": "batal", "invoice_no": "INV-2", "cashier_id": "u1",
                      "total": 99000, "payment_method": "tunai",
                      "voided_at": "2026-08-07T01:00:00+00:00",
                      "created_at": "2026-08-07T00:30:00+00:00" }
                ],
                "sale_items": [
                    { "id": 1, "sale_id": "batal", "product_id": "p", "name": "X",
                      "price": 99000, "qty_milli": 1000, "line_total": 99000 }
                ],
                "stock_movements": [
                    { "id": 1, "product_id": "p", "kind": "sale", "qty_milli": -1000,
                      "sale_id": "batal", "created_at": "2026-08-07T00:30:00+00:00" }
                ]
            }
        });
        let data = bungkus(&kepala, &bundel).unwrap();

        let hasil = impor(&mut conn, Path::new("tidak-ada.sqlite"), &data).unwrap();

        assert_eq!(hasil.baris.get("sales"), Some(&1));
        assert_eq!(hasil.baris.get("sale_items"), Some(&0));
        assert_eq!(hasil.baris.get("stock_movements"), Some(&0));
        let omzet: f64 = conn
            .query_row("SELECT COALESCE(SUM(total),0) FROM transactions", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert!((omzet - 15000.0).abs() < 1e-9, "omzet melar jadi {omzet}");
    }

    #[test]
    fn impor_mengganti_bukan_menumpuk() {
        let mut conn = db_uji();
        conn.execute(
            "INSERT INTO products (id,name,sell_price,cost_price,default_discount,
                                   is_active,is_deleted,updated_at,dirty,ever_synced)
             VALUES ('lama','Barang Lama',1,1,0,1,0,'2026-01-01T00:00:00Z',0,0)",
            [],
        )
        .unwrap();

        impor(&mut conn, Path::new("tidak-ada.sqlite"), FIXTURE).unwrap();

        let sisa: i64 = conn
            .query_row("SELECT COUNT(*) FROM products WHERE id='lama'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(sisa, 0);
    }

    #[test]
    fn lampiran_pos2_disimpan_utuh_dan_ikut_pulang() {
        let mut conn = db_uji();
        let kepala = json!({
            "app": APP_V2, "versi_app": "1.0.0", "versi_bundel": 1,
            "dibuat": "2026-08-07T00:00:00+00:00",
            "toko": { "id": "t", "nama": "T" }, "jumlah": {}, "ringkas": {}, "catatan": []
        });
        let bundel = json!({
            "format": 1, "sumber": kepala, "tabel": {},
            "lampiran": { "template_laporan": [{ "id": "abc", "nama": "Kop Toko" }] }
        });
        let data = bungkus(&kepala, &bundel).unwrap();

        impor(&mut conn, Path::new("tidak-ada.sqlite"), &data).unwrap();
        let (keluar, _) = ekspor(&conn, "T").unwrap();

        let ulang = buka(&keluar).unwrap();
        assert_eq!(
            ulang["lampiran"]["template_laporan"][0]["nama"],
            json!("Kop Toko")
        );
        // ...dan ia tidak bocor jadi setelan biasa yang muncul di layar
        // Pengaturan.
        let bocor = daftar(&ulang, "settings")
            .iter()
            .any(|s| teks(s, "key") == KUNCI_LAMPIRAN);
        assert!(!bocor);
    }

    #[test]
    fn store_id_pc_ini_tidak_ikut_diganti() {
        let mut conn = db_uji();
        crate::db::set_setting(&conn, "store_id", "punya-pc-ini").unwrap();

        impor(&mut conn, Path::new("tidak-ada.sqlite"), FIXTURE).unwrap();

        assert_eq!(
            crate::db::get_setting(&conn, "store_id")
                .unwrap()
                .as_deref(),
            Some("punya-pc-ini")
        );
    }

    #[test]
    fn struk_batal_beruas_id_kosong_tidak_menyeret_mutasi_lain() {
        // Sebelum `id_yang_dibatalkan` menyaring id kosong, satu baris cacat
        // seperti ini membuang SELURUH mutasi yang tidak menempel pada struk —
        // termasuk seluruh Item Masuk — dan stoknya melenceng tanpa galat.
        let mut conn = db_uji();
        let kepala = json!({
            "app": APP_V2, "versi_app": "1.0.0", "versi_bundel": 1,
            "dibuat": "2026-08-07T00:00:00+00:00",
            "toko": { "id": "t", "nama": "T" }, "jumlah": {}, "ringkas": {}, "catatan": []
        });
        let bundel = json!({
            "format": 1, "sumber": kepala, "lampiran": {},
            "tabel": {
                "sales": [
                    { "id": "", "invoice_no": "?", "cashier_id": "u1", "total": 0,
                      "payment_method": "tunai", "voided_at": "2026-08-07T01:00:00+00:00",
                      "created_at": "2026-08-07T00:00:00+00:00" }
                ],
                "stock_movements": [
                    { "id": 1, "product_id": "p", "kind": "in", "qty_milli": 5000,
                      "qty_after_milli": 5000, "created_at": "2026-08-07T00:00:00+00:00" }
                ]
            }
        });
        let data = bungkus(&kepala, &bundel).unwrap();

        let hasil = impor(&mut conn, Path::new("tidak-ada.sqlite"), &data).unwrap();

        assert_eq!(hasil.baris.get("stock_movements"), Some(&1));
    }

    #[test]
    fn tanda_mutasi_bolak_balik_konsisten() {
        // Yang dijaga: dua konversi harus tetap saling kebalikan. Kalau salah
        // satunya dibalik dua kali, stok hasil migrasi bergerak ke arah yang
        // salah dan baru ketahuan saat ada yang mencocokkan.
        assert_eq!(ke_delta("out", 2000), -2000);
        assert_eq!(ke_delta("sale", -1000), -1000);
        assert_eq!(ke_delta("in", 5000), 5000);
        assert_eq!(ke_delta("opname", 9000), 9000);
        assert!((dari_mili(ke_delta("out", 2000).abs()) - 2.0).abs() < 1e-9);
    }
}
