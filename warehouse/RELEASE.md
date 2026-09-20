# Cara rilis update GPOS Warehouse

Sama seperti GALAXYAS POS, tapi app-nya folder `warehouse/`, repo rilisnya
`savinsaka/gpos-warehouse-releases`, dan **tag-nya diawali `wh-v`** (bukan `v`)
supaya tidak memicu build POS.

## Repo & signing

- **`savinsaka/galaxyas-pos`** (repo ini, private) — source code.
- **`savinsaka/gpos-warehouse-releases`** (public) — installer `.exe`/`.msi` +
  `latest.json` yang dibaca updater aplikasi.
- Signing key **sama** dengan POS (pubkey di `warehouse/src-tauri/tauri.conf.json`
  identik), jadi memakai secret yang sama: `TAURI_SIGNING_PRIVATE_KEY` &
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- Publish ke repo rilis pakai `RELEASE_REPO_TOKEN`. **Token ini harus punya akses
  tulis (Contents: Read and write) ke `gpos-warehouse-releases` juga** — buka
  token fine-grained di GitHub Settings → tambahkan repo ini ke daftar
  "Repository access". (Kalau tidak, langkah publish di CI akan gagal 403.)

## Setiap kali rilis update

1. Samakan versi di 3 file: `warehouse/package.json`, `warehouse/src-tauri/Cargo.toml`,
   `warehouse/src-tauri/tauri.conf.json` (juga `Cargo.lock` entry `desktop`).
2. Commit.
3. Tag diawali `wh-v`:
   ```
   git tag wh-v1.0.0
   git push origin wh-v1.0.0
   ```
4. Tunggu CI (tab Actions). Sukses → muncul **draft release** di
   `gpos-warehouse-releases`.
5. Review lalu **Publish release**. Setelah dipublish, app lama (yang sudah
   ber-updater) akan lihat banner update.

## Catatan

- Warehouse & POS bisa dipasang berdampingan (identifier beda:
  `com.galaxyas.warehouse` vs `com.galaxyas.pos`, DB lokal terpisah).
