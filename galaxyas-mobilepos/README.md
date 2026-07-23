# GALAXYAS Mobile POS

Aplikasi kasir Android (Kotlin + Jetpack Compose) yang menjadi **kasir tambahan**
untuk GALAXYAS POS desktop. HP **tidak menyimpan database sendiri** — semua data
(barang, stok, transaksi) ada di PC yang menjalankan **Server Pusat**, dan HP
mengaksesnya lewat jaringan wifi yang sama.

Kontrak protokol dengan desktop: lihat [PROTOCOL.md](PROTOCOL.md).

## Untuk pengguna (kasir)

### 1. Pasang aplikasi
Unduh `galaxyas-mobilepos-X.Y.Z.apk` dari halaman rilis, buka file-nya di HP,
lalu izinkan pemasangan dari "sumber tidak dikenal" bila diminta.

### 2. Nyalakan Server Pusat di PC
Di GALAXYAS POS desktop: **Pengaturan → Server Pusat** → centang
"Jadikan PC ini Server Pusat". Catat **IP**, **Port** (biasanya 8899), dan
**Kode Pairing** yang tampil.

### 3. Hubungkan HP
Pastikan HP dan PC berada di **wifi yang sama**, buka aplikasi, isi Nama
Server, IP, Port, dan Kode Pairing → **Hubungkan**. Lalu login memakai
username + PIN yang sama seperti di desktop.

### 4. Printer struk (opsional)
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

### Menerbitkan rilis
1. Naikkan `versionCode` (+1) dan `versionName` di `app/build.gradle.kts`.
2. Build & tandatangani APK, ganti nama jadi `galaxyas-mobilepos-X.Y.Z.apk`.
3. Buat rilis bertag `mobile-vX.Y.Z` di repo publik
   `savinsaka/galaxyas-pos-releases`, lampirkan APK + `mobile-latest.json`:
   ```json
   {
     "version": "X.Y.Z",
     "apk_url": "https://github.com/savinsaka/galaxyas-pos-releases/releases/download/mobile-vX.Y.Z/galaxyas-mobilepos-X.Y.Z.apk",
     "notes": "Ringkasan perubahan."
   }
   ```

Aplikasi mengecek berkas itu saat tab **Menu** dibuka dan menampilkan tombol
unduh bila ada versi lebih baru (tanpa auto-update — pemasangan tetap manual).

### Catatan teknis
- **Cleartext HTTP wajib** (`network_security_config.xml`) karena Server Pusat
  memakai HTTP polos di IP LAN privat. Jangan dipakai di wifi publik.
- **Paritas struk**: byte ESC/POS diuji terhadap fixture golden yang dihasilkan
  implementasi desktop (`app/src/test/resources/fixtures`). Kalau format struk
  di desktop berubah, perbarui fixture dan `EscPos.kt` bersamaan.
- **Sinkronisasi tipe** dengan desktop dilakukan manual — lihat aturan di
  PROTOCOL.md sebelum mengubah `Models.kt` atau `ApiClient.kt`.
