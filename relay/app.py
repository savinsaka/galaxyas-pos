"""Relay GALAXYAS POS — penerus permintaan HP kasir ke PC kasir.

Kenapa ada: app mobile dulu hanya bisa memanggil Server Pusat lewat IP lokal,
jadi HP wajib satu wifi dengan PC kasir. PC kasir tidak bisa dijangkau langsung
dari internet (CGNAT / tanpa IP publik), maka **PC kasir yang menelepon keluar**
ke relay ini lewat WebSocket, dan relay meneruskan permintaan HP ke socket itu.

Aturan utama: **relay TIDAK PERNAH mengantre.** Kalau PC kasir sedang tidak
terhubung, permintaan langsung ditolak 503. Tidak boleh ada checkout/opname yang
tersimpan di sini lalu dieksekusi belakangan — stok harus selalu mencerminkan
keadaan sekarang. Relay juga tidak menyimpan data POS apa pun; satu-satunya yang
dipersistenkan adalah daftar toko (`relay.db`).
"""

from __future__ import annotations

import asyncio
import hashlib
import json
import os
import secrets
import sqlite3
import time
import uuid
from collections import defaultdict, deque
from contextlib import asynccontextmanager, closing
from datetime import datetime, timezone
from typing import Any

from fastapi import FastAPI, Request, WebSocket, WebSocketDisconnect
from fastapi.middleware.gzip import GZipMiddleware
from fastapi.responses import JSONResponse

DB_PATH = os.environ.get("RELAY_DB", os.path.join(os.path.dirname(__file__), "relay.db"))

# Kunci untuk rute /admin/* (dipakai app GALAXYAS Relay Admin di PC pemilik).
# Kosong = rute admin MATI TOTAL, bukan sekadar tertutup — supaya relay yang
# dipasang tanpa sengaja tidak pernah bisa didaftari toko oleh orang luar.
ADMIN_KEY = os.environ.get("RELAY_ADMIN_KEY", "")

# Batas waktu menunggu jawaban PC kasir. Sengaja lebih pendek dari timeout baca
# di app HP (30 detik) supaya HP menerima pesan 504 yang jelas, bukan timeout
# mentah tanpa keterangan.
REQUEST_TIMEOUT_S = 25.0

AGENT_OFFLINE_MSG = "PC kasir sedang mati atau tidak terhubung internet."
AGENT_TIMEOUT_MSG = "PC kasir tidak menjawab — coba lagi sebentar lagi."

DEVICE_HEADER = "x-galaxyas-token"


# ---------- Penyimpanan daftar toko ----------


def db() -> sqlite3.Connection:
    conn = sqlite3.connect(DB_PATH)
    conn.row_factory = sqlite3.Row
    return conn


def init_schema() -> None:
    with closing(db()) as conn:
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS stores (
                id             TEXT PRIMARY KEY,
                name           TEXT NOT NULL,
                agent_key_hash TEXT NOT NULL,
                created_at     TEXT NOT NULL
            )
            """
        )
        conn.commit()


def sha256_hex(value: str) -> str:
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def _now_iso() -> str:
    return datetime.now(timezone.utc).isoformat()


def verify_store(store_id: str, agent_key: str) -> sqlite3.Row | None:
    with closing(db()) as conn:
        row = conn.execute("SELECT * FROM stores WHERE id = ?", (store_id,)).fetchone()
    if row is None:
        return None
    # compare_digest: hindari membocorkan panjang kecocokan lewat waktu.
    if not secrets.compare_digest(row["agent_key_hash"], sha256_hex(agent_key)):
        return None
    return row


# ---------- Registry agent yang sedang terhubung ----------


class Agent:
    """Satu PC kasir yang sedang online, plus permintaan yang menunggu jawabannya."""

    def __init__(self, store_id: str, ws: WebSocket) -> None:
        self.store_id = store_id
        self.ws = ws
        self.pending: dict[str, asyncio.Future] = {}
        self.send_lock = asyncio.Lock()
        # Hash token perangkat yang sah, dikirim PC kasir saat connect. Selama
        # masih None relay meneruskan apa adanya dan PC kasir yang memvalidasi.
        self.token_hashes: set[str] | None = None
        self.connected_at = time.time()

    async def send(self, payload: dict[str, Any]) -> None:
        async with self.send_lock:
            await self.ws.send_text(json.dumps(payload))

    def fail_all(self, message: str) -> None:
        """Putus = semua yang menunggu langsung gagal. Tidak ada yang diantre."""
        for fut in list(self.pending.values()):
            if not fut.done():
                fut.set_exception(RuntimeError(message))
        self.pending.clear()


AGENTS: dict[str, Agent] = {}


async def forward(store_id: str, payload: dict[str, Any]) -> JSONResponse:
    """Teruskan satu permintaan ke PC kasir dan tunggu jawabannya."""
    agent = AGENTS.get(store_id)
    if agent is None:
        return JSONResponse({"error": AGENT_OFFLINE_MSG}, status_code=503)

    request_id = uuid.uuid4().hex
    future: asyncio.Future = asyncio.get_running_loop().create_future()
    agent.pending[request_id] = future
    try:
        await agent.send({"id": request_id, **payload})
        reply = await asyncio.wait_for(future, timeout=REQUEST_TIMEOUT_S)
    except asyncio.TimeoutError:
        return JSONResponse({"error": AGENT_TIMEOUT_MSG}, status_code=504)
    except Exception:
        # Socket putus di tengah jalan — perlakukan sama dengan PC kasir mati.
        return JSONResponse({"error": AGENT_OFFLINE_MSG}, status_code=503)
    finally:
        agent.pending.pop(request_id, None)

    status = int(reply.get("status", 200))
    body = reply.get("body")
    return JSONResponse(body, status_code=status)


# ---------- Rate limit sederhana (in-memory, per IP) ----------

_hits: dict[str, deque[float]] = defaultdict(deque)


def rate_limit(key: str, limit: int, window_s: float) -> bool:
    """True bila masih dalam batas. Cukup untuk satu proses uvicorn."""
    now = time.time()
    bucket = _hits[key]
    while bucket and now - bucket[0] > window_s:
        bucket.popleft()
    if len(bucket) >= limit:
        return False
    bucket.append(now)
    return True


def client_ip(request: Request) -> str:
    # Selalu di belakang reverse proxy (lihat DEPLOY.md) — pakai hop pertama.
    forwarded = request.headers.get("x-forwarded-for", "")
    if forwarded:
        return forwarded.split(",")[0].strip()
    return request.client.host if request.client else "?"


# ---------- App ----------

@asynccontextmanager
async def lifespan(_app: FastAPI):
    init_schema()
    yield


app = FastAPI(title="GALAXYAS POS Relay", version="1.0.0", lifespan=lifespan)
# Laporan & master data bisa ratusan KB; HP sering di kuota seluler.
app.add_middleware(GZipMiddleware, minimum_size=1024)


@app.get("/health")
def health() -> dict[str, Any]:
    """Liveness relay itu sendiri (bukan status PC kasir)."""
    return {"status": "ok", "stores_online": len(AGENTS)}


@app.websocket("/agent/ws")
async def agent_ws(websocket: WebSocket) -> None:
    """PC kasir menyambung ke sini dan menunggu perintah."""
    store_id = websocket.headers.get("x-store-id", "")
    agent_key = websocket.headers.get("x-agent-key", "")
    store = verify_store(store_id, agent_key) if store_id and agent_key else None
    if store is None:
        await websocket.close(code=4401, reason="store id atau agent key salah")
        return

    await websocket.accept()

    # Satu toko = satu socket. Koneksi baru (mis. app di-restart) menendang yang
    # lama, supaya tidak ada dua PC mengaku toko yang sama.
    previous = AGENTS.get(store_id)
    if previous is not None:
        previous.fail_all("koneksi digantikan koneksi baru")
        try:
            await previous.ws.close(code=4409, reason="digantikan koneksi baru")
        except Exception:
            pass

    agent = Agent(store_id, websocket)
    AGENTS[store_id] = agent

    try:
        while True:
            raw = await websocket.receive_text()
            try:
                message = json.loads(raw)
            except json.JSONDecodeError:
                continue

            kind = message.get("type")
            if kind == "tokens":
                # Daftar hash token perangkat yang sah — dipakai menolak token
                # asing tanpa perlu membangunkan PC kasir.
                agent.token_hashes = set(message.get("hashes") or [])
            elif kind == "pong":
                continue
            else:
                future = agent.pending.get(message.get("id", ""))
                if future is not None and not future.done():
                    future.set_result(message)
    except WebSocketDisconnect:
        pass
    except Exception:
        pass
    finally:
        if AGENTS.get(store_id) is agent:
            del AGENTS[store_id]
        agent.fail_all(AGENT_OFFLINE_MSG)


def device_token_known(store_id: str, token: str) -> bool:
    """Saring token asing sebelum mengganggu PC kasir. Selama PC kasir belum
    mengirim daftar hash-nya, semua token dilewatkan dan PC kasir yang menolak."""
    agent = AGENTS.get(store_id)
    if agent is None or agent.token_hashes is None:
        return True
    return sha256_hex(token) in agent.token_hashes


@app.get("/s/{store_id}/health")
async def store_health(store_id: str) -> JSONResponse:
    """Dipakai ConnectionWatcher di HP tiap 30 detik. Dijawab relay sendiri —
    tidak diteruskan ke PC kasir supaya hemat kuota dan tidak membebani kasir."""
    if store_id in AGENTS:
        return JSONResponse({"ok": True})
    return JSONResponse({"error": AGENT_OFFLINE_MSG}, status_code=503)


@app.post("/s/{store_id}/pair")
async def store_pair(store_id: str, request: Request) -> JSONResponse:
    """Tukar kode pairing 6 karakter dengan token perangkat. Dibatasi ketat
    karena inilah satu-satunya rute yang menerima kode pendek."""
    if not rate_limit(f"pair:{client_ip(request)}", limit=10, window_s=3600):
        return JSONResponse(
            {"error": "Terlalu banyak percobaan pairing. Coba lagi satu jam lagi."},
            status_code=429,
        )
    body = await request.json()
    return await forward(
        store_id,
        {
            "kind": "pair",
            "code": str(body.get("code", "")),
            "device_name": str(body.get("device_name", "")),
        },
    )


# ---------- Rute admin (app GALAXYAS Relay Admin) ----------


def admin_guard(request: Request) -> JSONResponse | None:
    """Balik JSONResponse kalau ditolak, None kalau boleh lanjut."""
    if not ADMIN_KEY:
        # 404, bukan 403: jangan bocorkan bahwa rute admin memang ada.
        return JSONResponse({"error": "not found"}, status_code=404)
    if not rate_limit(f"admin:{client_ip(request)}", limit=60, window_s=60):
        return JSONResponse({"error": "Terlalu banyak permintaan."}, status_code=429)
    provided = request.headers.get("x-admin-key", "")
    if not secrets.compare_digest(provided, ADMIN_KEY):
        return JSONResponse({"error": "Kunci admin salah."}, status_code=401)
    return None


@app.get("/admin/stores")
async def admin_list_stores(request: Request) -> JSONResponse:
    if (denied := admin_guard(request)) is not None:
        return denied
    with closing(db()) as conn:
        rows = conn.execute("SELECT id, name, created_at FROM stores ORDER BY created_at").fetchall()
    return JSONResponse(
        [
            {
                "id": r["id"],
                "name": r["name"],
                "created_at": r["created_at"],
                "online": r["id"] in AGENTS,
            }
            for r in rows
        ]
    )


@app.post("/admin/stores")
async def admin_create_store(request: Request) -> JSONResponse:
    if (denied := admin_guard(request)) is not None:
        return denied
    body = await request.json()
    name = str(body.get("name", "")).strip()
    if not name:
        return JSONResponse({"error": "Nama toko wajib diisi."}, status_code=400)

    store_id = secrets.token_hex(16)
    agent_key = secrets.token_hex(32)
    with closing(db()) as conn:
        conn.execute(
            "INSERT INTO stores (id, name, agent_key_hash, created_at) VALUES (?, ?, ?, ?)",
            (store_id, name, sha256_hex(agent_key), _now_iso()),
        )
        conn.commit()
    # agent_key hanya ada di respons ini — yang tersimpan cuma hash-nya.
    return JSONResponse({"id": store_id, "name": name, "agent_key": agent_key})


@app.delete("/admin/stores/{store_id}")
async def admin_delete_store(store_id: str, request: Request) -> JSONResponse:
    if (denied := admin_guard(request)) is not None:
        return denied
    with closing(db()) as conn:
        cur = conn.execute("DELETE FROM stores WHERE id = ?", (store_id,))
        conn.commit()
        removed = cur.rowcount
    # Putuskan agent yang mungkin masih tersambung dengan kredensial lama.
    agent = AGENTS.pop(store_id, None)
    if agent is not None:
        agent.fail_all("toko dihapus")
        try:
            await agent.ws.close(code=4403, reason="toko dihapus")
        except Exception:
            pass
    if not removed:
        return JSONResponse({"error": "Toko tidak ditemukan."}, status_code=404)
    return JSONResponse({"ok": True})


@app.post("/s/{store_id}/rpc/{command}")
async def store_rpc(store_id: str, command: str, request: Request) -> JSONResponse:
    if not rate_limit(f"rpc:{client_ip(request)}", limit=600, window_s=60):
        return JSONResponse({"error": "Terlalu banyak permintaan."}, status_code=429)

    token = request.headers.get(DEVICE_HEADER, "")
    if not token or not device_token_known(store_id, token):
        return JSONResponse({"error": "unauthorized"}, status_code=401)

    raw = await request.body()
    try:
        args = json.loads(raw) if raw.strip() else None
    except json.JSONDecodeError as exc:
        return JSONResponse({"error": f"JSON tidak valid: {exc}"}, status_code=400)

    return await forward(store_id, {"kind": "rpc", "cmd": command, "args": args, "auth": token})
