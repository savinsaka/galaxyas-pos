# Cara rilis update GALAXYAS POS

Panduan singkat: sekali setup, lalu setiap ada update tinggal 4 langkah (bump versi → commit → tag → push tag). CI yang build & publish otomatis.

## Kenapa ada 2 repo?

- **`savinsaka/galaxyas-pos`** (repo ini) — **private**. Berisi seluruh source code.
- **`savinsaka/galaxyas-pos-releases`** — **public**, sengaja dibuat kosong dari source code, isinya cuma installer hasil build (`.exe`/`.msi`) + `latest.json`.

Kenapa dipisah: aplikasi yang sudah terpasang di komputer user perlu bisa cek update tanpa login/token (GitHub Release di repo private butuh autentikasi untuk diakses, sementara aplikasi user tidak punya kredensial itu). Dengan repo release terpisah yang public, source code tetap rahasia tapi endpoint update tetap bisa diakses bebas — masih gratis, tidak perlu server sendiri.

## Setup sekali saja

- [x] Signing key sudah digenerate, public key sudah ditempel di `desktop/src-tauri/tauri.conf.json` (`plugins.updater.pubkey`).
- [x] Workflow CI sudah ada di `.github/workflows/release.yml`, target publish-nya sudah diarahkan ke `savinsaka/galaxyas-pos-releases`.
- [x] Secret `TAURI_SIGNING_PRIVATE_KEY` dan `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` sudah tersimpan di repo private ini.
- [x] Repo `savinsaka/galaxyas-pos-releases` sudah dibuat (public, kosong).
- [x] Secret `RELEASE_REPO_TOKEN` sudah tersimpan. Setup lengkap — langsung lanjut ke bagian "Setiap kali mau rilis update" di bawah.

### Membuat token `RELEASE_REPO_TOKEN`

1. Buka **https://github.com/settings/personal-access-tokens/new** (Fine-grained token).
2. **Resource owner**: pilih akun `savinsaka`. **Repository access**: pilih "Only select repositories" → pilih `galaxyas-pos-releases` saja (jangan pilih repo private-nya).
3. **Permissions → Repository permissions → Contents**: set ke **Read and write**.
4. **Expiration**: pilih sebebasnya (mis. "No expiration" atau 1 tahun, tinggal perpanjang nanti kalau expired dan build mulai gagal).
5. Klik **Generate token**, salin nilainya (`github_pat_...`).
6. Beri tahu saya nilainya (atau paste sendiri ke repo private → **Settings → Secrets and variables → Actions → New repository secret**, nama: `RELEASE_REPO_TOKEN`).

Token ini sengaja **dibatasi hanya ke repo release** (bukan repo source code) — jadi walau suatu saat bocor, paling buruk cuma bisa upload/hapus file release, tidak bisa baca source code private.

### Menambahkan secret ke GitHub (kalau perlu setup ulang / pindah komputer)

1. Buka repo private ini di GitHub → **Settings → Secrets and variables → Actions → New repository secret**.
2. Buat 3 secret:
   - `TAURI_SIGNING_PRIVATE_KEY` — isi dengan seluruh isi file private key (lokasi lokal: `C:\Users\savin\.tauri\galaxyas-updater.key`, satu baris teks base64).
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` — password key tersebut (simpan di password manager).
   - `RELEASE_REPO_TOKEN` — fine-grained PAT dari langkah di atas.
3. Selesai — CI otomatis pakai ketiga secret ini setiap kali build.

**Penting:** simpan file `galaxyas-updater.key` (dan passwordnya) baik-baik — kalau hilang, Anda tidak bisa lagi menandatangani update baru dan harus generate key baru (yang berarti user lama harus install ulang manual sekali, karena aplikasi lama tidak akan percaya key baru).

## Setiap kali mau rilis update

1. **Samakan nomor versi** di 3 file (harus identik):
   - `desktop/package.json` → `"version"`
   - `desktop/src-tauri/Cargo.toml` → `version = "..."`
   - `desktop/src-tauri/tauri.conf.json` → `"version"`

2. **Commit** perubahan seperti biasa.

3. **Tag** versi (harus diawali `v`):
   ```
   git tag v0.2.0
   ```

4. **Push tag-nya** (ini yang memicu build otomatis):
   ```
   git push origin v0.2.0
   ```

5. Tunggu ~5–10 menit. Buka tab **Actions** di repo **private** ini untuk lihat progress build. Kalau sukses, otomatis muncul **draft release** baru di tab **Releases** repo **`galaxyas-pos-releases`** (yang public) — berisi installer `.exe`/`.msi` + file `latest.json` (manifest yang dibaca aplikasi untuk cek update).

6. **Review lalu klik "Publish release"** di halaman draft tersebut (di repo `galaxyas-pos-releases`). Selama masih draft, endpoint update belum melihatnya — jadi user lama belum akan dapat notifikasi. Ini kesengajaan, supaya Anda bisa cek dulu sebelum disebar ke semua orang.

7. Setelah dipublish: user lama yang membuka aplikasi akan melihat banner biru di atas "🔔 Versi X.Y.Z tersedia" → klik **Update Sekarang** → aplikasi otomatis download, install, dan restart sendiri ke versi baru.

## Kalau build gagal / rilis salah

- Build gagal: cek log di tab Actions, biasanya karena versi di 3 file tidak sinkron atau ada error kompilasi. Hapus tag yang gagal (`git tag -d v0.2.0 && git push origin :refs/tags/v0.2.0`), perbaiki, lalu ulangi dari langkah 3 dengan versi baru.
- Rilis sudah dipublish tapi ternyata bermasalah: hapus/unpublish release tersebut di GitHub secepatnya (jadikan draft lagi atau delete) supaya endpoint `latest` tidak lagi mengarah ke situ, lalu segera rilis versi perbaikan.
