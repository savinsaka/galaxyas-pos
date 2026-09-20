# GALAXYAS Gudang Bridge

Server minimal penghubung **GPOS Warehouse (gudang)** → banyak **POS toko**.
Menggantikan `galaxyas-mobile` untuk kebutuhan kirim-barang. Auth pakai **API key
mesin** (tanpa akun/email/password). FastAPI + SQLite (stdlib), deploy ringan.

## Endpoint

Pengirim (gudang) — header `X-Warehouse-Key`:
- `GET  /api/v1/stores` → daftar toko tujuan `[{id, code, name}]`
- `POST /api/v1/order-rows` `{store_id, items:[{barcode, qty, name?}]}` → buat baris
  kiriman (satu batch, atomik)

Toko (POS "Pull dari Gudang") — header `X-Store-Api-Key` (kompatibel dgn
`galaxyas-pos/desktop/src-tauri/src/pull.rs`):
- `GET  /api/v1/bridge/order-rows`
- `POST /api/v1/bridge/order-rows/confirm` `{items:[{id, qty_confirmed}]}`
- `POST /api/v1/bridge/order-rows/{id}/reject`

Admin — header `X-Admin-Token`:
- `GET  /api/v1/admin/keys` → lihat warehouse_key + key tiap toko
- `POST /api/v1/admin/stores/{id}/rotate-key`

## Dev lokal

```
python -m venv .venv
./.venv/Scripts/pip install -r requirements.txt   # .venv/bin/pip di Linux
./.venv/Scripts/python seed.py                     # bikin 2 toko + key, cetak
./.venv/Scripts/python -m uvicorn app.main:app --reload --port 8001
```

## Deploy VPS (menggantikan galaxyas-mobile di port 8001 / app.jjapps.net)

Ringkas (lihat catatan rilis warehouse):
1. Backup DB galaxyas-mobile lama, stop & disable `galaxyas-mobile.service`.
2. Taruh folder ini di `~/warehouse-bridge`, buat venv + install requirements.
3. `python seed.py` → catat WAREHOUSE_KEY, ADMIN_TOKEN, dan API key tiap toko.
4. systemd unit baru `galaxyas-gudang.service` jalankan uvicorn di `127.0.0.1:8001`
   (Caddy `app.jjapps.net` sudah menunjuk ke 8001, tak perlu ubah).
5. Isi WAREHOUSE_KEY di GPOS Warehouse; isi API key toko di tiap POS.

`GUDANG_DB_PATH` (env) menentukan lokasi file SQLite (default `./gudang.db`).
`ADMIN_TOKEN` (env) menimpa admin_token dari DB bila diset.
