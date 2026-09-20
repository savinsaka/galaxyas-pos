"""Seed awal bridge Gudang: bikin warehouse_key + admin_token (kalau belum ada)
dan 2 toko default (kalau belum ada toko sama sekali), lalu cetak semua key.

Jalankan sekali di server:  python seed.py
Aman diulang: tidak menimpa key yang sudah ada, tidak menggandakan toko.
Nama toko bisa di-override:  TOKO1_NAME="Galaxyas Pusat" TOKO2_NAME="Galaxyas Cabang" python seed.py
"""

import os
import secrets
import uuid

from app import db


def _ensure_config(key: str, gen) -> str:
    val = db.get_config(key)
    if not val:
        val = gen()
        db.set_config(key, val)
    return val


def _ensure_store(code: str, name: str) -> dict:
    conn = db.get_conn()
    row = conn.execute("SELECT id, code, name, api_key FROM stores WHERE code=?", (code,)).fetchone()
    if row:
        return dict(row)
    sid = uuid.uuid4().hex
    key = secrets.token_urlsafe(32)
    conn.execute(
        "INSERT INTO stores(id, code, name, api_key, active, created_at) VALUES(?,?,?,?,1,?)",
        (sid, code, name, key, db.now_iso()),
    )
    conn.commit()
    return {"id": sid, "code": code, "name": name, "api_key": key}


def main() -> None:
    db.init_db()

    warehouse_key = _ensure_config("warehouse_key", lambda: secrets.token_urlsafe(32))
    admin_token = _ensure_config("admin_token", lambda: secrets.token_urlsafe(24))

    existing = db.get_conn().execute("SELECT COUNT(*) c FROM stores").fetchone()["c"]
    stores = []
    if existing == 0:
        stores.append(_ensure_store("toko-1", os.environ.get("TOKO1_NAME", "Toko 1")))
        stores.append(_ensure_store("toko-2", os.environ.get("TOKO2_NAME", "Toko 2")))
    else:
        stores = [dict(r) for r in db.get_conn().execute("SELECT code, name, api_key FROM stores ORDER BY code").fetchall()]

    print("=" * 60)
    print("GALAXYAS Gudang Bridge — kredensial")
    print("=" * 60)
    print(f"WAREHOUSE_KEY (untuk GPOS Warehouse) : {warehouse_key}")
    print(f"ADMIN_TOKEN   (untuk /api/v1/admin)  : {admin_token}")
    print("-" * 60)
    for s in stores:
        print(f"Toko {s['code']:<8} {s['name']:<20} API key (untuk POS toko): {s['api_key']}")
    print("=" * 60)
    print("Simpan baik-baik. WAREHOUSE_KEY diisi di Warehouse; API key toko diisi di POS masing-masing.")


if __name__ == "__main__":
    main()
