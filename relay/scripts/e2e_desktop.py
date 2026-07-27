"""Uji rangkaian penuh: agent desktop (Rust) ↔ relay ini ↔ klien HTTP sebagai HP.

    python scripts/e2e_desktop.py

Menyalakan relay di port sementara, mendaftarkan toko uji, lalu menjalankan
`cargo test --lib relay_e2e -- --ignored` di `desktop/src-tauri` dengan env var
yang menunjuk ke relay tadi. Database relay uji dibuat & dihapus sendiri;
database POS yang dipakai uji ada di memori (tidak menyentuh data toko asli).

Uji ini yang membuktikan bagian yang tidak terlihat oleh uji per-komponen:
handshake WebSocket, header auth agent, envelope bolak-balik, dan janji
"tanpa antrian" saat PC kasir mati.
"""

from __future__ import annotations

import os
import secrets
import socket
import subprocess
import sys
import threading
import time
from datetime import datetime, timezone

sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

HERE = os.path.dirname(os.path.abspath(__file__))
RELAY_DIR = os.path.dirname(HERE)
REPO = os.path.dirname(RELAY_DIR)
SRC_TAURI = os.path.join(REPO, "desktop", "src-tauri")

TEST_DB = os.path.join(HERE, "e2e_relay.db")
if os.path.exists(TEST_DB):
    os.remove(TEST_DB)
os.environ["RELAY_DB"] = TEST_DB

import uvicorn  # noqa: E402

import app as relay  # noqa: E402


def free_port() -> int:
    with socket.socket() as s:
        s.bind(("127.0.0.1", 0))
        return s.getsockname()[1]


def main() -> int:
    port = free_port()
    store_id = secrets.token_hex(8)
    agent_key = secrets.token_hex(16)

    relay.init_schema()
    with relay.db() as conn:
        conn.execute(
            "INSERT INTO stores (id, name, agent_key_hash, created_at) VALUES (?,?,?,?)",
            (store_id, "Toko E2E", relay.sha256_hex(agent_key), datetime.now(timezone.utc).isoformat()),
        )
        conn.commit()

    server = uvicorn.Server(
        uvicorn.Config(relay.app, host="127.0.0.1", port=port, log_level="warning")
    )
    threading.Thread(target=server.run, daemon=True).start()
    for _ in range(50):
        if server.started:
            break
        time.sleep(0.1)
    print(f"relay uji jalan di http://127.0.0.1:{port} (store {store_id})\n")

    env = {
        **os.environ,
        "GALAXYAS_RELAY_HTTP": f"http://127.0.0.1:{port}",
        "GALAXYAS_RELAY_WS": f"ws://127.0.0.1:{port}",
        "GALAXYAS_RELAY_STORE": store_id,
        "GALAXYAS_RELAY_KEY": agent_key,
    }
    result = subprocess.run(
        ["cargo", "test", "--lib", "relay_e2e", "--", "--ignored", "--nocapture"],
        cwd=SRC_TAURI,
        env=env,
    )

    server.should_exit = True
    time.sleep(0.5)
    # Windows menahan file selama koneksi sqlite thread relay belum benar-benar
    # tutup; sisa file uji tidak masalah (sudah di-gitignore).
    try:
        os.remove(TEST_DB)
    except OSError:
        pass
    return result.returncode


if __name__ == "__main__":
    raise SystemExit(main())
