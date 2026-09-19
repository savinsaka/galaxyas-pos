# GALAXYAS POS — app desktop (Tauri + SvelteKit + TypeScript)

Ini app `.exe` yang dipakai kasir. Database SQLite lokal, dibuat otomatis di app
data dir saat pertama kali jalan.

```bash
npm install
npm run tauri dev
```

Rilis: lihat [RELEASE.md](RELEASE.md).

## Migrasi ke/dari GALAXYAS POS 2 (berkas `.gpos`)

*Pengaturan → Migrasi Data* memindahkan **seluruh isi satu toko** — barang,
stok, seluruh riwayat penjualan, mutasi stok, nota masuk/keluar, shift,
pengeluaran, merek, pelanggan, diskon periodik, pengguna, dan setelan toko —
antara app ini dan GALAXYAS POS 2, lewat satu berkas berekstensi `.gpos`.
Dua arah: keluar dan masuk.

Yang ikut hanya **toko yang sedang terbuka**. Instalasi yang memegang tiga toko
menghasilkan tiga berkas; pindah dulu ke toko yang dimaksud, lalu ulangi.

**Impor mengganti seluruh isi toko, bukan menggabungkan.** Database toko
disalin lebih dulu ke `<nama>.sebelum-migrasi-<stempel>.bak` di folder yang
sama, dan gagal menyalin berarti impor batal. Konfirmasinya kalimat yang harus
diketik ulang, bukan tombol.

Beberapa hal **tidak bisa** ikut masuk ke sini karena app ini tidak punya
kolomnya. Semuanya dilaporkan di layar, bukan didiamkan:

- penjualan & nota stok yang **dibatalkan** di POS 2 — app ini tidak punya
  konsep pembatalan, jadi kalau ikut masuk ia terhitung sebagai penjualan
  sungguhan dan omzet melar;
- **PIN kasir** dari POS 2 — di sana ia disimpan sebagai hash yang tidak bisa
  dikembalikan, jadi tiap kasir harus menyetel PIN-nya lagi lewat Hak Akses;
- merek, pelanggan, dan diskon yang sudah dihapus di POS 2.

Daftar HP terdaftar tidak pernah ikut ke arah mana pun: tokennya milik pasangan
PC–perangkat tertentu, dan memindahkannya berarti memindahkan hak akses.

Migrasi **dimatikan saat PC ini sedang jadi klien Server Pusat**: ia selalu
bekerja pada database PC ini, dan di mode klien database itu bukan yang dipakai
berjualan.

> **Format berkasnya kontrak antar dua aplikasi**, bukan urusan internal:
> `contracts/MIGRASI.md` di repo `Gpos2`. Implementasi di sini
> `src-tauri/src/migrasi.rs`, dan berkas contoh yang dibuat sisi POS 2
> (`src-tauri/fixtures/migrasi-contoh.gpos`) ikut dibaca `cargo test`. Yang
> mengubah formatnya di satu sisi saja akan melihat gejalanya di toko sebagai
> "berkas migrasi rusak" untuk berkas yang sebenarnya baik-baik saja.

## Server Pusat: beberapa kasir, satu database

Satu PC bisa dijadikan **Server Pusat** (Pengaturan → Server Pusat → centang
"Jadikan PC ini Server Pusat"). PC kasir lain memakai app yang sama dalam mode
klien: semua data (barang, stok, transaksi, akun) dibaca dan ditulis langsung ke
database PC pusat — tanpa cloud, tanpa sinkronisasi.

Klien bisa menyambung lewat **dua jalur**, dan kasir memilih sendiri tiap membuka
app (layar "Pilih Server"). App **tidak pernah** berpindah jalur sendiri.

| Jalur | Alamat | Kapan dipakai |
|---|---|---|
| 🖧 **WIFI** | `http://<ip-pc-pusat>:8899` | PC klien satu jaringan dengan PC pusat. Paling cepat, tidak butuh internet. |
| 🌐 **INTERNET** | `https://<relay>/s/<store_id>` | PC klien di tempat lain (rumah, cabang). Perlu internet di kedua PC + Akses Online aktif di PC pusat. |

Jalur internet memakai relay yang sama dengan app HP (lihat
[`relay/README.md`](../relay/README.md)): PC pusat yang **menelepon keluar** ke
relay, jadi tidak perlu port forwarding atau IP publik.

### Memasang PC kasir klien

Di **PC pusat** (Pengaturan → Server Pusat):

1. Centang "Jadikan PC ini Server Pusat".
2. Untuk jalur internet: isi & nyalakan **Akses Online** (URL relay, Store ID,
   Agent Key dari `relay/scripts/add_store.py`).
3. Klik **"Buat Kode Setup"** → **Salin**. Kirim kodenya ke PC klien (WhatsApp,
   flashdisk, apa saja). Kode ini memuat alamat wifi, alamat internet, dan kode
   pairing sekaligus — isinya sama dengan QR untuk HP, cuma berbentuk teks
   karena PC tidak punya kamera.

Di **PC klien** (layar awal):

4. **+ Tambah Server** → tempel di kolom *Kode Setup* → **Isi Otomatis** (semua
   kolom terisi; masih bisa diedit manual).
5. Pilih jalur (Wifi/Internet) → **Uji Koneksi** → **Hubungkan**.

Sekali mendaftar, PC klien mendapat token perangkat sendiri (64 hex) dan
**kedua jalur langsung bisa dipakai** — tinggal pilih WIFI atau INTERNET di layar
"Pilih Server" tiap kali buka app. PC klien muncul di daftar "Perangkat
Terhubung" di PC pusat (dengan nama komputernya) dan bisa dicabut satu-satu.

### Yang perlu diingat

- **PC pusat harus menyala.** Kalau mati atau internetnya putus, klien langsung
  diberi tahu — **tidak ada antrian**. Tidak akan ada checkout/opname tertahan
  yang menyusul jalan belakangan, jadi stok selalu mencerminkan keadaan sekarang.
- **Checkout tidak akan tercatat dobel.** Transaksi baru dianggap selesai kalau
  PC pusat sudah menyimpannya. Kalau koneksi putus tepat setelah tersimpan
  sehingga jawabannya tidak sampai, kasir cukup **menekan Bayar lagi**: setiap
  percobaan membawa kunci yang sama (`client_ref`), jadi PC pusat mengembalikan
  transaksi yang sudah ada alih-alih mencatat yang kedua — stok pun tidak
  berkurang dua kali. Kalau isi keranjang sudah berubah sejak percobaan pertama,
  kasir diberi tahu transaksi mana yang sebenarnya tersimpan.
- Kode pairing 6 karakter **hanya sah di jalur wifi**; lewat internet yang
  diterima cuma token perangkat, jadi kodenya tidak bisa ditebak dari luar.
- "Buat Kode Baru" (kode pairing) tidak memutus perangkat yang sudah terdaftar —
  hanya mencegah pendaftaran baru. Untuk memutus satu perangkat, cabut dari
  daftar "Perangkat Terhubung".
- Struk, printer, dan laci kasir tetap memakai pengaturan PC masing-masing.

## Cek Harga: akun yang hanya boleh melihat harga

Modul **Cek Harga** adalah satu kolom scan dan satu kartu jawaban: nama barang,
harga, diskon, dan harga bayar. Diskonnya dihitung dengan aturan yang sama
persis dengan kasir (diskon periodik menang atas diskon default barang, cakupan
item menang atas merek, yang terbesar dipakai) untuk 1 buah — jadi angka di
layar ini sama dengan yang nanti keluar di kasir.

Kolom scannya berperilaku sama dengan kolom scan kasir: barcode yang tidak
ketemu membuka popup pencarian nama (F5 membukanya kapan saja). Bedanya, di
popup itu **kolom stok disembunyikan** — layar ini sengaja tidak menampilkan
stok maupun harga modal.

Hak aksesnya berdiri sendiri: centang **Cek Harga** di *Pengaturan → Hak Akses*
dan biarkan modul lain kosong, maka akun itu hanya melihat satu tab Cek Harga —
tidak ada kasir, stok, laporan, atau pengaturan. Akun kasir yang sudah ada tidak
otomatis mendapatkannya; centang sendiri kalau memang mau.

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
