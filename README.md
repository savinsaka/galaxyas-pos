# GALAXYAS POS

Aplikasi **Point of Sale desktop** untuk bisnis retail multi-cabang.

> **Prinsip utama:** *Local Heavy + Offline First*. Semua operasional harian
> (penjualan, stok, transaksi) berjalan 100% di lokal tanpa bergantung internet.
> Server hanya dipakai untuk **sinkronisasi master data** secara **manual**
> (bukan otomatis, tanpa queue). Konflik diselesaikan dengan **Last Write Wins**
> berdasarkan kolom `updated_at`.

## Arsitektur

| Komponen        | Teknologi              | Keterangan                          |
| --------------- | ---------------------- | ----------------------------------- |
| Desktop App     | Tauri v2 + Svelte      | Aplikasi `.exe` per toko            |
| Backend Server  | FastAPI (Python)       | Hanya untuk sync master data        |
| Database Server | PostgreSQL             | Source of truth master data         |
| Database Lokal  | SQLite                 | Per toko (offline)                  |
| Sync Method     | REST API + Delta Sync  | Manual trigger                      |

```
galaxyas-pos/
├── backend/          # FastAPI sync server (PostgreSQL)
│   └── app/
│       ├── main.py
│       ├── database.py
│       ├── models.py
│       ├── schemas.py
│       └── routers/sync.py
└── desktop/          # Tauri v2 + Svelte (SQLite lokal)
    ├── src/                # Frontend Svelte (UI POS)
    └── src-tauri/src/      # Rust: SQLite, commands, sync client
```

## Yang di-sync vs tidak di-sync

**Di-sync (master data, manual):** nama barang, barcode, kategori, merek, satuan,
harga jual & pokok, default discount, status aktif/non-aktif, member pelanggan,
diskon periode.

**TIDAK di-sync:** stok, transaksi penjualan, data kasir lokal.

## Cara Sinkronisasi

1. Operator klik **"Kirim Data ke Server"** (Upload) — kirim perubahan master data lokal.
2. Operator klik **"Ambil Update dari Server"** (Download) — tarik perubahan terbaru.
3. Menggunakan **Delta Sync** berdasarkan `updated_at`.
4. Konflik diselesaikan dengan **Last Write Wins**.

## Menjalankan

### Backend (FastAPI + PostgreSQL)

```bash
cd backend
python -m venv .venv
.venv\Scripts\activate          # Windows
pip install -r requirements.txt
copy .env.example .env          # lalu sesuaikan DATABASE_URL
uvicorn app.main:app --reload
```

API docs: http://localhost:8000/docs

### Desktop (Tauri v2 + Svelte)

```bash
cd desktop
npm install
npm run tauri dev
```

Database lokal SQLite dibuat otomatis di app data dir saat pertama kali jalan.
