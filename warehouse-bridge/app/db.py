"""SQLite minimal untuk bridge Gudang→Toko. Sengaja pakai sqlite3 stdlib (tanpa
ORM) supaya deploy ringan: cuma butuh fastapi + uvicorn."""

import os
import sqlite3
import threading
from datetime import datetime, timezone

DB_PATH = os.environ.get("GUDANG_DB_PATH", os.path.join(os.path.dirname(os.path.dirname(__file__)), "gudang.db"))

_local = threading.local()


def now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def get_conn() -> sqlite3.Connection:
    """Satu koneksi per-thread (uvicorn workers/threadpool aman)."""
    conn = getattr(_local, "conn", None)
    if conn is None:
        conn = sqlite3.connect(DB_PATH, check_same_thread=False)
        conn.row_factory = sqlite3.Row
        conn.execute("PRAGMA journal_mode=WAL")
        conn.execute("PRAGMA foreign_keys=ON")
        _local.conn = conn
    return conn


SCHEMA = """
CREATE TABLE IF NOT EXISTS stores (
    id         TEXT PRIMARY KEY,
    code       TEXT UNIQUE NOT NULL,
    name       TEXT NOT NULL,
    api_key    TEXT UNIQUE NOT NULL,
    active     INTEGER NOT NULL DEFAULT 1,
    created_at TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS config (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS order_rows (
    id          TEXT PRIMARY KEY,
    store_id    TEXT NOT NULL REFERENCES stores(id),
    barcode     TEXT NOT NULL,
    name        TEXT,
    qty_dikirim REAL NOT NULL,
    status      TEXT NOT NULL DEFAULT 'pending',  -- pending | confirmed | rejected
    created_at  TEXT NOT NULL,
    updated_at  TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_order_rows_store_status ON order_rows(store_id, status);
"""


def init_db() -> None:
    conn = get_conn()
    conn.executescript(SCHEMA)
    conn.commit()


def get_config(key: str) -> str | None:
    row = get_conn().execute("SELECT value FROM config WHERE key=?", (key,)).fetchone()
    return row["value"] if row else None


def set_config(key: str, value: str) -> None:
    conn = get_conn()
    conn.execute(
        "INSERT INTO config(key, value) VALUES(?, ?) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        (key, value),
    )
    conn.commit()
