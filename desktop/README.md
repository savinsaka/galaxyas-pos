# GALAXYAS POS — app desktop (Tauri + SvelteKit + TypeScript)

Ini app `.exe` yang dipakai kasir. Database SQLite lokal, dibuat otomatis di app
data dir saat pertama kali jalan.

```bash
npm install
npm run tauri dev
```

Rilis: lihat [RELEASE.md](RELEASE.md).

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

## Recommended IDE Setup

[VS Code](https://code.visualstudio.com/) + [Svelte](https://marketplace.visualstudio.com/items?itemName=svelte.svelte-vscode) + [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode) + [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer).
