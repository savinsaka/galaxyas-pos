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
        # Remote GPOS: ID 9 digit dikunci ke rahasia per-PC saat pertama kali
        # dipakai, supaya PC lain tidak bisa mengaku ID yang sama.
        conn.execute(
            """
            CREATE TABLE IF NOT EXISTS remote_ids (
                remote_id   TEXT PRIMARY KEY,
                secret_hash TEXT NOT NULL,
                created_at  TEXT NOT NULL
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
    return {
        "status": "ok",
        "stores_online": len(AGENTS),
        "remote_hosts": len(REMOTE_HOSTS),
        "remote_sessions": active_remote_sessions(),
    }


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


# ---------- Remote GPOS (eksperimental) ----------
#
# PC yang mau dibantu ("host") menyalakan Remote di GPOS: ia menyambung ke
# /remote/host membawa ID 9 digit, rahasia per-PC, dan hash OTP sesi. PC yang
# membantu ("viewer") menyambung ke /remote/view dengan ID + OTP. Relay hanya
# memasangkan dua socket itu lalu meneruskan pesan apa adanya (JSON input dan
# frame JPEG) — tidak ada yang disimpan, tidak ada yang diantre. Laju frame
# diatur ujung-ke-ujung (host menunggu `ack` viewer sebelum kirim frame
# berikutnya), jadi relay tidak pernah menumpuk frame di RAM.

REMOTE_MAX_SESSIONS = int(os.environ.get("RELAY_REMOTE_MAX_SESSIONS", "2"))
REMOTE_MAX_HOSTS = 50
REMOTE_WAIT_S = 15 * 60
REMOTE_MAX_MSG = 2 * 1024 * 1024


class RemoteHost:
    def __init__(self, remote_id: str, ws: WebSocket, otp_hash: str) -> None:
        self.remote_id = remote_id
        self.ws = ws
        # Dikosongkan begitu viewer masuk — OTP sekali pakai.
        self.otp_hash: str | None = otp_hash
        self.viewer: WebSocket | None = None
        self.done = asyncio.Event()


REMOTE_HOSTS: dict[str, RemoteHost] = {}


def active_remote_sessions() -> int:
    return sum(1 for h in REMOTE_HOSTS.values() if h.viewer is not None)


def _ws_ip(websocket: WebSocket) -> str:
    forwarded = websocket.headers.get("x-forwarded-for", "")
    if forwarded:
        return forwarded.split(",")[0].strip()
    return websocket.client.host if websocket.client else "?"


async def _ws_reject(websocket: WebSocket, code: int, message: str) -> None:
    """Terima dulu lalu kirim alasan, supaya GPOS bisa menampilkan pesan yang
    jelas (close sebelum accept hanya jadi HTTP 403 tanpa keterangan)."""
    try:
        await websocket.accept()
        await websocket.send_text(json.dumps({"type": "error", "message": message}))
        await websocket.close(code=code)
    except Exception:
        pass


def _claim_remote_id(remote_id: str, secret: str) -> bool:
    with closing(db()) as conn:
        row = conn.execute(
            "SELECT secret_hash FROM remote_ids WHERE remote_id = ?", (remote_id,)
        ).fetchone()
        if row is None:
            conn.execute(
                "INSERT INTO remote_ids (remote_id, secret_hash, created_at) VALUES (?, ?, ?)",
                (remote_id, sha256_hex(secret), _now_iso()),
            )
            conn.commit()
            return True
    return secrets.compare_digest(row["secret_hash"], sha256_hex(secret))


async def _pipe(src: WebSocket, dst: WebSocket) -> None:
    """Teruskan pesan src → dst sampai salah satu putus."""
    while True:
        msg = await src.receive()
        if msg["type"] == "websocket.disconnect":
            return
        data_bytes = msg.get("bytes")
        data_text = msg.get("text")
        if data_bytes is not None:
            if len(data_bytes) <= REMOTE_MAX_MSG:
                await dst.send_bytes(data_bytes)
        elif data_text is not None:
            if len(data_text) <= REMOTE_MAX_MSG:
                await dst.send_text(data_text)


async def _close_quietly(ws: WebSocket | None, code: int) -> None:
    if ws is None:
        return
    try:
        await ws.close(code=code)
    except Exception:
        pass


@app.websocket("/remote/host")
async def remote_host_ws(websocket: WebSocket) -> None:
    remote_id = websocket.headers.get("x-remote-id", "")
    secret = websocket.headers.get("x-remote-secret", "")
    otp_hash = websocket.headers.get("x-remote-otp-hash", "").lower()

    if not rate_limit(f"remote-host:{_ws_ip(websocket)}", limit=60, window_s=600):
        await _ws_reject(websocket, 4429, "Terlalu banyak percobaan. Coba lagi nanti.")
        return
    if not (remote_id.isdigit() and len(remote_id) == 9 and len(secret) >= 32 and len(otp_hash) == 64):
        await _ws_reject(websocket, 4400, "Data sesi remote tidak valid.")
        return
    if not _claim_remote_id(remote_id, secret):
        await _ws_reject(websocket, 4401, "ID remote ini sudah dipakai PC lain.")
        return
    if remote_id not in REMOTE_HOSTS and len(REMOTE_HOSTS) >= REMOTE_MAX_HOSTS:
        await _ws_reject(websocket, 4429, "Relay sedang penuh. Coba lagi nanti.")
        return

    await websocket.accept()

    # Satu ID = satu host. Yang baru (mis. switch dimatikan-nyalakan) menendang yang lama.
    previous = REMOTE_HOSTS.get(remote_id)
    if previous is not None:
        previous.done.set()
        await _close_quietly(previous.viewer, 4409)
        await _close_quietly(previous.ws, 4409)

    host = RemoteHost(remote_id, websocket, otp_hash)
    REMOTE_HOSTS[remote_id] = host

    # Satu-satunya pembaca socket host ada di sini (arah host → viewer). Arah
    # sebaliknya dibaca di remote_view_ws. Dua pembaca pada satu socket ASGI
    # akan saling merebut pesan.
    try:
        await websocket.send_text(json.dumps({"type": "ready"}))
        deadline = time.time() + REMOTE_WAIT_S
        while not host.done.is_set():
            if host.viewer is None:
                remaining = deadline - time.time()
                if remaining <= 0:
                    await websocket.send_text(
                        json.dumps({"type": "error", "message": "Sesi kedaluwarsa (15 menit tanpa penyambung)."})
                    )
                    await websocket.close(code=4408)
                    return
                timeout = min(remaining, 5.0)
            else:
                timeout = None
            try:
                msg = await asyncio.wait_for(websocket.receive(), timeout=timeout)
            except asyncio.TimeoutError:
                continue
            if msg["type"] == "websocket.disconnect":
                return
            viewer = host.viewer
            if viewer is None:
                continue  # denyut selama menunggu
            data_bytes = msg.get("bytes")
            data_text = msg.get("text")
            if data_bytes is not None and len(data_bytes) <= REMOTE_MAX_MSG:
                await viewer.send_bytes(data_bytes)
            elif data_text is not None and len(data_text) <= REMOTE_MAX_MSG:
                await viewer.send_text(data_text)
    except Exception:
        pass
    finally:
        if REMOTE_HOSTS.get(remote_id) is host:
            del REMOTE_HOSTS[remote_id]
        host.done.set()
        await _close_quietly(host.viewer, 4410)


@app.websocket("/remote/view")
async def remote_view_ws(websocket: WebSocket) -> None:
    remote_id = websocket.headers.get("x-remote-id", "").replace(" ", "")
    otp = websocket.headers.get("x-remote-otp", "").strip()

    # Ketat: OTP 6 digit hanya aman kalau tebakan dibatasi.
    if not rate_limit(f"remote-view:{_ws_ip(websocket)}", limit=15, window_s=600):
        await _ws_reject(websocket, 4429, "Terlalu banyak percobaan. Coba lagi 10 menit lagi.")
        return
    host = REMOTE_HOSTS.get(remote_id)
    if host is None or host.done.is_set():
        await _ws_reject(websocket, 4404, "PC tujuan tidak sedang membuka remote.")
        return
    if host.viewer is not None or host.otp_hash is None:
        await _ws_reject(websocket, 4409, "PC tujuan sedang diremote orang lain.")
        return
    if not secrets.compare_digest(host.otp_hash, sha256_hex(otp)):
        await _ws_reject(websocket, 4401, "ID atau OTP salah.")
        return
    if active_remote_sessions() >= REMOTE_MAX_SESSIONS:
        await _ws_reject(websocket, 4429, "Relay sedang penuh (batas sesi remote). Coba lagi nanti.")
        return

    await websocket.accept()
    host.viewer = websocket
    host.otp_hash = None

    tasks: list[asyncio.Task] = []
    try:
        await websocket.send_text(json.dumps({"type": "joined"}))
        await host.ws.send_text(json.dumps({"type": "viewer_joined"}))
        tasks = [
            asyncio.create_task(_pipe(websocket, host.ws)),
            asyncio.create_task(host.done.wait()),
        ]
        await asyncio.wait(tasks, return_when=asyncio.FIRST_COMPLETED)
    except Exception:
        pass
    finally:
        for t in tasks:
            t.cancel()
        host.done.set()
        await _close_quietly(websocket, 4410)
        await _close_quietly(host.ws, 4410)
