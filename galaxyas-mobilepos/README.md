# GALAXYAS Mobile POS

Aplikasi kasir Android (Kotlin + Jetpack Compose) yang menjadi **kasir tambahan**
untuk GALAXYAS POS desktop. HP **tidak menyimpan database sendiri** — semua data
(barang, stok, transaksi) ada di PC yang menjalankan **Server Pusat**.

Ada dua cara menjangkau PC itu, dipilih kasir sendiri saat membuka aplikasi:

| | Kapan dipakai | Catatan |
|---|---|---|
| **[LOCAL]** | HP di wifi toko | Langsung ke IP PC. Paling cepat, tanpa kuota, tetap jalan walau internet toko mati. |
| **[ONLINE]** | Dari mana saja | Lewat relay di VPS. Perlu PC kasir menyala + internet di kedua sisi. |

Karena tidak ada antrian di jalur ONLINE, transaksi tidak pernah "nyangkut":
kalau PC kasir mati, HP langsung diberi tahu dan tidak ada yang tersimpan
diam-diam untuk dijalankan belakangan.

Kontrak protokol dengan desktop: lihat [PROTOCOL.md](PROTOCOL.md).
Pemasangan relay: lihat [../relay/DEPLOY.md](../relay/DEPLOY.md).

## Untuk pengguna (kasir)

### 1. Pasang aplikasi
Unduh `galaxyas-mobilepos-X.Y.Z.apk` dari halaman rilis, buka file-nya di HP,
lalu izinkan pemasangan dari "sumber tidak dikenal" bila diminta.

### 2. Nyalakan Server Pusat di PC
Di GALAXYAS POS desktop: **Pengaturan → Server Pusat** → centang
"Jadikan PC ini Server Pusat". Catat **IP**, **Port** (biasanya 8899), dan
**Kode Pairing** yang tampil.

Untuk pemakaian dari luar toko, nyalakan juga **Akses Online** di layar yang
sama dan catat **URL Relay** + **Store ID**-nya.

### 3. Hubungkan HP
Lakukan **sekali saja**, sebaiknya saat HP masih di wifi toko.

Cara cepat: di PC klik **Tampilkan QR**, lalu di HP tekan **📷 Scan QR dari PC**.
Semua kolom terisi sendiri — termasuk Store ID relay yang 32 karakter dan tidak
mungkin diketik benar. Tinggal **Hubungkan**.

Cara manual (kalau kamera bermasalah): isi Nama Server, lalu jalur yang tersedia
— IP + Port untuk LOCAL, URL Relay + Store ID untuk ONLINE (boleh keduanya
sekaligus) — dan Kode Pairing → **Hubungkan**.

Kode Pairing hanya dipakai saat pendaftaran ini. Sesudahnya HP memegang kunci
sendiri, dan pemilik toko bisa mencabut akses per-HP dari **Pengaturan →
Perangkat Terhubung** di PC tanpa mengganggu HP lain.

### 4. Setiap membuka aplikasi
Pilih **[LOCAL]** atau **[ONLINE]** sesuai posisi HP saat itu, lalu login
memakai username + PIN yang sama seperti di desktop. Jalur bisa diganti kapan
saja lewat **Menu → Ganti Server**.

### 5. Printer struk (opsional)
1. Pasangkan printer thermal Bluetooth lewat **Pengaturan Bluetooth HP** dulu
   (sekali saja).
2. Di aplikasi: **Menu → Printer & Kertas** → pilih printer → atur lebar kertas
   (58/80 mm) → **Test Print**.

> Catatan: aplikasi hanya menampilkan printer yang **sudah dipasangkan** di HP —
> tidak melakukan pencarian perangkat, jadi tidak perlu izin lokasi.

### Yang bisa dilakukan
Kasir (scan kamera, diskon periodik, tunai/QRIS/kombinasi, tahan transaksi,
cetak struk), riwayat transaksi (cetak ulang, batalkan), master data (barang,
merek, diskon, pelanggan, cek harga), persediaan (opname, item masuk/keluar +
cetak dokumen, pengeluaran), laporan (4 jenis + cetak sebagai struk),
pengaturan toko/struk/printer/tema, hak akses.

---

## Untuk pengembang

### Prasyarat
- Android SDK (platform 36, build-tools) di `%LOCALAPPDATA%\Android\Sdk`
- JDK 17 — JBR bawaan Android Studio (`C:\Program Files\Android\Android Studio\jbr`)
- `local.properties` berisi `sdk.dir` (tidak di-commit)

Versi toolchain di-pin: Gradle 8.14.3 + AGP 8.11.0 + Kotlin 2.1.21.
Jangan dinaikkan tanpa alasan — kombinasi ini yang terbukti jalan.

### Build & test
```bash
./gradlew test            # unit test JVM (RPC, serialisasi, ESC/POS golden, engine diskon)
./gradlew assembleDebug   # APK debug
./gradlew assembleRelease # APK release (minify + shrink)
```

APK debug: `app/build/outputs/apk/debug/app-debug.apk`

### Penandatanganan rilis (sekali saja)

Keystore **tidak boleh masuk repo** dan passwordnya dipegang pemilik aplikasi.
Kalau keystore hilang, update aplikasi tidak bisa dipasang di atas versi lama.

1. Buat keystore (jalankan sendiri, pilih password sendiri):
   ```bash
   keytool -genkeypair -v -keystore %USERPROFILE%\.galaxyas\galaxyas-mobilepos.keystore -alias galaxyas -keyalg RSA -keysize 2048 -validity 10000
   ```
2. Buat `keystore.properties` di root project ini (sudah di-gitignore):
   ```properties
   storeFile=C:/Users/<user>/.galaxyas/galaxyas-mobilepos.keystore
   storePassword=<password toko>
   keyAlias=galaxyas
   keyPassword=<password kunci>
   ```
3. `./gradlew assembleRelease` → `app/build/outputs/apk/release/app-release.apk`

Tanpa `keystore.properties`, `assembleRelease` tetap jalan tapi menghasilkan
APK **unsigned** (tidak bisa dipasang di HP).

### Penyebaran otomatis APK (sejak 1.1.0)

`assembleRelease` diikuti task `sebarApkRilis` yang menyalin APK ke dua tempat
dengan nama `galaxyas-mobilepos-<versionName>.apk` — tidak ada lagi salin manual:

| Tujuan | Guna |
|---|---|
| `galaxyas-mobilepos/dist/` | arsip lokal semua versi (gitignored) |
| `G:\My Drive\aplikasi pos` | Google Drive — Drive for Desktop mengunggah sendiri, APK langsung bisa diunduh dari HP |

Lokasi Drive bisa dipindah tanpa mengubah kode, lewat `local.properties`
(gitignored): `apkDriveDir=G:\\My Drive\\folder lain`. Kalau foldernya tidak
ada (Drive belum jalan, atau build di mesin lain), langkah Drive **dilewati
dengan peringatan** — build rilis sengaja tidak digagalkan karenanya.

### Menerbitkan rilis
1. Naikkan `versionCode` (+1) dan `versionName` di `app/build.gradle.kts`.
2. `./gradlew assembleRelease` — APK otomatis mendarat di `dist/` + Google Drive.
3. Lampirkan APK ke rilis desktop publik yang sedang dibuat CI (tag `vX.Y.Z` di
   `savinsaka/galaxyas-pos-releases`) dengan nama aset **stabil**
   `galaxyas-mobilepos.apk`, plus `mobile-latest.json`:
   ```json
   {
     "version": "X.Y.Z",
     "apk_url": "https://github.com/savinsaka/galaxyas-pos-releases/releases/latest/download/galaxyas-mobilepos.apk",
     "notes": "Ringkasan perubahan."
   }
   ```
   ```bash
   gh release upload vX.Y.Z galaxyas-mobilepos.apk mobile-latest.json -R savinsaka/galaxyas-pos-releases
   ```
   **Setiap** rilis wajib membawa dua aset ini. Kalau satu rilis saja
   melewatkannya, `releases/latest/download/mobile-latest.json` jadi 404 dan
   notifikasi update di semua HP mati diam-diam (kejadian di v1.2.0–v1.3.1).

Aplikasi mengecek berkas itu saat tab **Menu** dibuka dan menampilkan tombol
unduh bila ada versi lebih baru (tanpa auto-update — pemasangan tetap manual).

### Catatan teknis
- **Cleartext HTTP wajib** (`network_security_config.xml`) karena jalur LOCAL
  memakai HTTP polos di IP LAN privat. Jangan dipakai di wifi publik. Jalur
  ONLINE selalu lewat HTTPS ke relay.
- **Kredensial**: token per-perangkat 64 karakter hex hasil `POST /pair`,
  dikirim di header `X-Galaxyas-Token` untuk kedua jalur. Kode pairing 6
  karakter masih diterima di jalur LOCAL (kompatibilitas HP lama) tapi **ditolak
  di jalur ONLINE** — terlalu pendek untuk dijaga dari tebakan lewat internet.
- **Paritas struk**: byte ESC/POS diuji terhadap fixture golden yang dihasilkan
  implementasi desktop (`app/src/test/resources/fixtures`). Kalau format struk
  di desktop berubah, perbarui fixture dan `EscPos.kt` bersamaan.
- **Sinkronisasi tipe** dengan desktop dilakukan manual — lihat aturan di
  PROTOCOL.md sebelum mengubah `Models.kt` atau `ApiClient.kt`.
