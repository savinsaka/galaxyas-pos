"""Daftarkan satu toko baru di relay.

    python scripts/add_store.py "Toko Pusat"

Mencetak `store_id` dan `agent_key` SEKALI — agent key hanya disimpan hash-nya,
jadi kalau hilang harus buat ulang (`reset_key.py` atau daftarkan toko baru).
Kedua nilai itu dimasukkan ke desktop: Pengaturan → Server Pusat → Akses Online.
"""

from __future__ import annotations

import os
import secrets
import sys
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from app import db, init_schema, sha256_hex  # noqa: E402


def main() -> None:
    if len(sys.argv) < 2:
        print('Pemakaian: python scripts/add_store.py "Nama Toko"')
        raise SystemExit(1)

    name = sys.argv[1]
    # 32 hex = 128 bit, tidak bisa ditebak. store_id ikut jadi rahasia ringan
    # karena muncul di URL yang dipakai HP.
    store_id = secrets.token_hex(16)
    agent_key = secrets.token_hex(32)

    init_schema()
    with db() as conn:
        conn.execute(
            "INSERT INTO stores (id, name, agent_key_hash, created_at) VALUES (?, ?, ?, ?)",
            (store_id, name, sha256_hex(agent_key), datetime.now(timezone.utc).isoformat()),
        )
        conn.commit()

    print(f"Toko    : {name}")
    print(f"store_id: {store_id}")
    print(f"agent_key: {agent_key}")
    print()
    print("Simpan agent_key sekarang juga — tidak bisa ditampilkan lagi.")


if __name__ == "__main__":
    main()
