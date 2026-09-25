"""Chat antar toko + Push Alert.

- Identitas = kode toko (sama dengan `store_id` sync) + Kunci Chat dari panel
  admin. Satu toko = satu identitas: semua PC toko itu tersambung dengan kode
  & kunci yang sama dan menerima pesan yang sama.
- Pesan teks disimpan 7 hari. File **tidak pernah disimpan**: diteruskan
  langsung ke PC toko tujuan yang sedang tersambung; yang dicatat hanya nama
  dan ukurannya.
- Proses uvicorn tunggal (lihat unit `galaxyas-sync`), jadi daftar socket yang
  tersambung cukup disimpan di memori.
"""

from __future__ import annotations

import asyncio
import hashlib
import json
import re
import secrets
import time
from collections import defaultdict, deque
from datetime import datetime, timedelta, timezone
from typing import Any

from anyio.to_thread import run_sync
from fastapi import APIRouter, Request, WebSocket, WebSocketDisconnect
from fastapi.responses import JSONResponse
from sqlalchemy import delete, inspect, or_, select, text, update

from app.database import SessionLocal, engine
from app.models import ChatMessage, ChatStore, utcnow

router = APIRouter(prefix="/api/v1/chat", tags=["chat"])

RETENTION = timedelta(days=7)
MAX_TEXT = 2000
MAX_FILE = 10 * 1024 * 1024
SEND_LIMIT = (30, 60.0)  # 30 pesan per 60 detik per toko
FILE_WAIT_S = 60.0

# Hanya gambar, Word, Excel, PDF. Sengaja TIDAK: svg (bisa berisi skrip),
# docm/xlsm (makro), dan semua yang lain.
ALLOWED_EXT = {
    "jpg", "jpeg", "png", "gif", "webp", "bmp", "heic", "heif", "tif", "tiff",
    "doc", "docx",
    "xls", "xlsx", "csv",
    "pdf",
}

CODE_RE = re.compile(r"^[A-Za-z0-9._-]{1,64}$")
KEY_ALPHABET = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789"  # tanpa 0/O/1/I yang mirip


# ---------- Kunci ----------


def normalize_key(key: str) -> str:
    return re.sub(r"[^A-Za-z0-9]", "", key).upper()


def hash_key(key: str) -> str:
    return hashlib.sha256(normalize_key(key).encode("utf-8")).hexdigest()


def generate_key() -> str:
    raw = "".join(secrets.choice(KEY_ALPHABET) for _ in range(16))
    return "-".join(raw[i : i + 4] for i in range(0, 16, 4))


def file_ext_allowed(name: str) -> bool:
    if "." not in name:
        return False
    return name.rsplit(".", 1)[1].lower() in ALLOWED_EXT


def safe_file_name(name: str) -> str:
    # Buang path & karakter yang tidak boleh di nama file Windows.
    base = name.replace("\\", "/").rsplit("/", 1)[-1]
    base = re.sub(r'[<>:"|?*\x00-\x1f]', "_", base).strip(" .")
    return base[:200] or "file"


# ---------- Akses DB (sinkron, dijalankan di thread) ----------


def ensure_chat_columns() -> None:
    """Migrasi kecil: kolom centang ditambahkan setelah tabel chat_messages
    sudah ada di produksi, dan create_all tidak mengubah tabel lama."""
    cols = {c["name"] for c in inspect(engine).get_columns("chat_messages")}
    kind = "TIMESTAMP WITH TIME ZONE" if engine.dialect.name == "postgresql" else "TIMESTAMP"
    with engine.begin() as conn:
        for col in ("delivered_at", "read_at"):
            if col not in cols:
                conn.execute(text(f"ALTER TABLE chat_messages ADD COLUMN {col} {kind}"))


def _iso(dt: datetime | None) -> str | None:
    if dt is None:
        return None
    if dt.tzinfo is None:
        dt = dt.replace(tzinfo=timezone.utc)
    return dt.isoformat()


def _mark_delivered(to_code: str) -> dict[str, Any]:
    """Semua pesan untuk toko ini yang belum 'diterima' → diterima sekarang.
    Balik {"at": waktu, "by_sender": {kode pengirim: [id]}} untuk dikabarkan
    ke pengirimnya."""
    now = utcnow()
    with SessionLocal() as db:
        rows = db.execute(
            select(ChatMessage.id, ChatMessage.from_code)
            .where(ChatMessage.to_code == to_code, ChatMessage.delivered_at.is_(None))
        ).all()
        if not rows:
            return {}
        db.execute(
            update(ChatMessage)
            .where(ChatMessage.id.in_([r.id for r in rows]))
            .values(delivered_at=now)
        )
        db.commit()
    out: dict[str, list[int]] = defaultdict(list)
    for r in rows:
        out[r.from_code].append(r.id)
    return {"at": _iso(now), "by_sender": dict(out)}


def _mark_read(me: str, peer: str, up_to: int) -> tuple[list[int], str]:
    """Pesan dari `peer` ke `me` sampai id `up_to` → dibaca."""
    now = utcnow()
    with SessionLocal() as db:
        ids = list(
            db.scalars(
                select(ChatMessage.id).where(
                    ChatMessage.from_code == peer,
                    ChatMessage.to_code == me,
                    ChatMessage.id <= up_to,
                    ChatMessage.read_at.is_(None),
                )
            ).all()
        )
        if ids:
            db.execute(update(ChatMessage).where(ChatMessage.id.in_(ids)).values(read_at=now))
            db.execute(
                update(ChatMessage)
                .where(ChatMessage.id.in_(ids), ChatMessage.delivered_at.is_(None))
                .values(delivered_at=now)
            )
            db.commit()
    return ids, _iso(now) or ""


def _verify(code: str, key: str) -> ChatStore | None:
    if not CODE_RE.match(code) or not key:
        return None
    with SessionLocal() as db:
        store = db.get(ChatStore, code)
        if store is None or not store.key_hash:
            return None
        if not secrets.compare_digest(store.key_hash, hash_key(key)):
            return None
        db.expunge(store)
        return store


def _store_name(code: str) -> str | None:
    with SessionLocal() as db:
        store = db.get(ChatStore, code)
        return store.name if store else None


_last_cleanup = 0.0


def _save_message(msg: ChatMessage) -> dict[str, Any]:
    global _last_cleanup
    with SessionLocal() as db:
        db.add(msg)
        # Pembersihan 7 hari dilakukan sambil lalu (paling sering sejam
        # sekali) — tidak perlu proses/timer terpisah.
        now = time.time()
        if now - _last_cleanup > 3600:
            _last_cleanup = now
            db.execute(delete(ChatMessage).where(ChatMessage.created_at < utcnow() - RETENTION))
        db.commit()
        db.refresh(msg)
        return message_dict(msg)


def _history(code: str, since: datetime | None) -> list[dict[str, Any]]:
    cutoff = utcnow() - RETENTION
    if since is not None and since > cutoff:
        cutoff = since
    with SessionLocal() as db:
        rows = db.scalars(
            select(ChatMessage)
            .where(or_(ChatMessage.from_code == code, ChatMessage.to_code == code))
            .where(ChatMessage.created_at > cutoff)
            .order_by(ChatMessage.id)
            .limit(3000)
        ).all()
        return [message_dict(m) for m in rows]


def message_dict(m: ChatMessage) -> dict[str, Any]:
    created = m.created_at
    if created.tzinfo is None:
        created = created.replace(tzinfo=timezone.utc)
    return {
        "id": m.id,
        "from": m.from_code,
        "to": m.to_code,
        "kind": m.kind,
        "body": m.body,
        "file_name": m.file_name,
        "file_size": m.file_size,
        "push": m.push,
        "created_at": created.isoformat(),
        "delivered_at": _iso(m.delivered_at),
        "read_at": _iso(m.read_at),
    }


# ---------- Socket yang tersambung ----------


class Conn:
    def __init__(self, code: str, ws: WebSocket) -> None:
        self.code = code
        self.ws = ws
        # Pasangan teks+biner (file) harus terkirim berurutan tanpa disela.
        self.lock = asyncio.Lock()

    async def send_json(self, payload: dict[str, Any], data: bytes | None = None) -> None:
        try:
            async with self.lock:
                await self.ws.send_text(json.dumps(payload))
                if data is not None:
                    await self.ws.send_bytes(data)
        except Exception:
            pass


CONNS: dict[str, set[Conn]] = defaultdict(set)
_send_hits: dict[str, deque[float]] = defaultdict(deque)
_lookup_hits: dict[str, deque[float]] = defaultdict(deque)


def _rate_ok(bucket: deque[float], limit: int, window: float) -> bool:
    now = time.time()
    while bucket and now - bucket[0] > window:
        bucket.popleft()
    if len(bucket) >= limit:
        return False
    bucket.append(now)
    return True


def online(code: str) -> bool:
    return bool(CONNS.get(code))


async def kick_store(code: str) -> None:
    """Putus semua PC toko ini (dipakai saat kunci diganti/toko dihapus)."""
    for conn in list(CONNS.get(code, ())):
        try:
            await conn.ws.close(code=4401)
        except Exception:
            pass
    CONNS.pop(code, None)


async def _deliver(message: dict[str, Any], sender: Conn, data: bytes | None = None) -> None:
    """Kirim ke semua PC toko tujuan (+ isi file bila ada) dan ke PC lain toko
    pengirim (tanpa isi file), supaya riwayat di semua PC kasir sama."""
    payload = {"type": "message", "message": message}
    for conn in list(CONNS.get(message["to"], ())):
        await conn.send_json(payload, data)
    for conn in list(CONNS.get(message["from"], ())):
        await conn.send_json(payload)


def _auth_headers(request_or_ws) -> tuple[str, str]:
    return (
        request_or_ws.headers.get("x-chat-store", "").strip(),
        request_or_ws.headers.get("x-chat-key", "").strip(),
    )


# ---------- WebSocket ----------


@router.websocket("/ws")
async def chat_ws(websocket: WebSocket) -> None:
    code, key = _auth_headers(websocket)
    store = await run_sync(_verify, code, key)
    await websocket.accept()
    if store is None:
        await websocket.send_text(
            json.dumps({"type": "error", "fatal": True, "message": "Kode toko atau Kunci Chat salah."})
        )
        await websocket.close(code=4401)
        return

    me = Conn(code, websocket)
    CONNS[code].add(me)
    await me.send_json({"type": "hello", "code": code, "name": store.name})
    # PC toko ini baru tersambung: pesan yang menunggu kini "diterima" (✓✓).
    delivered = await run_sync(_mark_delivered, code)
    for sender, ids in (delivered.get("by_sender") or {}).items():
        await _receipt(sender, ids, delivered_at=delivered["at"])

    try:
        while True:
            raw = await websocket.receive()
            if raw["type"] == "websocket.disconnect":
                break
            text = raw.get("text")
            if text is None:
                continue  # biner tanpa header: abaikan
            try:
                msg = json.loads(text)
            except json.JSONDecodeError:
                continue
            kind = msg.get("type")
            if kind == "ping":
                await me.send_json({"type": "pong"})
            elif kind == "send":
                await _handle_text(me, msg)
            elif kind == "file":
                await _handle_file(me, msg, websocket)
            elif kind == "read":
                await _handle_read(me, msg)
    except WebSocketDisconnect:
        pass
    except Exception:
        pass
    finally:
        CONNS.get(code, set()).discard(me)
        if code in CONNS and not CONNS[code]:
            del CONNS[code]


async def _receipt(code: str, ids: list[int], delivered_at: str | None = None, read_at: str | None = None) -> None:
    """Kabari semua PC toko `code` bahwa status centang pesan-pesan ini berubah."""
    if not ids:
        return
    payload = {"type": "receipt", "ids": ids, "delivered_at": delivered_at, "read_at": read_at}
    for conn in list(CONNS.get(code, ())):
        await conn.send_json(payload)


async def _handle_read(me: Conn, msg: dict[str, Any]) -> None:
    peer = str(msg.get("peer", "")).strip()
    try:
        up_to = int(msg.get("up_to", 0))
    except (TypeError, ValueError):
        return
    if not CODE_RE.match(peer) or up_to <= 0:
        return
    ids, at = await run_sync(_mark_read, me.code, peer, up_to)
    # Pengirim melihat ✓✓ biru; PC lain toko ini ikut menghapus angka belum dibaca.
    await _receipt(peer, ids, delivered_at=at, read_at=at)
    await _receipt(me.code, ids, delivered_at=at, read_at=at)


async def _reject(me: Conn, client_id: Any, message: str) -> None:
    await me.send_json({"type": "error", "client_id": client_id, "message": message})


async def _check_target(me: Conn, client_id: Any, to: str) -> bool:
    if to == me.code:
        await _reject(me, client_id, "Tidak bisa mengirim ke toko sendiri.")
        return False
    if not CODE_RE.match(to) or await run_sync(_store_name, to) is None:
        await _reject(me, client_id, f"Kode toko '{to}' tidak terdaftar untuk chat.")
        return False
    if not _rate_ok(_send_hits[me.code], *SEND_LIMIT):
        await _reject(me, client_id, "Terlalu banyak pesan. Tunggu sebentar.")
        return False
    return True


async def _handle_text(me: Conn, msg: dict[str, Any]) -> None:
    client_id = msg.get("client_id")
    to = str(msg.get("to", "")).strip()
    body = str(msg.get("body", "")).strip()
    if not body:
        return
    if len(body) > MAX_TEXT:
        await _reject(me, client_id, f"Pesan terlalu panjang (maks {MAX_TEXT} karakter).")
        return
    if not await _check_target(me, client_id, to):
        return
    row = ChatMessage(
        from_code=me.code,
        to_code=to,
        kind="text",
        body=body,
        push=bool(msg.get("push")),
        delivered_at=utcnow() if online(to) else None,
    )
    saved = await run_sync(_save_message, row)
    saved["client_id"] = client_id
    await _deliver(saved, me)


async def _handle_file(me: Conn, msg: dict[str, Any], websocket: WebSocket) -> None:
    """Header JSON, lalu satu frame biner berisi file. Frame biner selalu
    dibaca dulu (walau nanti ditolak) supaya urutan protokol tidak kacau."""
    client_id = msg.get("client_id")
    to = str(msg.get("to", "")).strip()
    name = safe_file_name(str(msg.get("name", "")))
    try:
        raw = await asyncio.wait_for(websocket.receive(), timeout=FILE_WAIT_S)
    except asyncio.TimeoutError:
        await _reject(me, client_id, "Isi file tidak diterima.")
        return
    if raw["type"] == "websocket.disconnect":
        raise WebSocketDisconnect()
    data = raw.get("bytes")
    if data is None:
        await _reject(me, client_id, "Isi file tidak diterima.")
        return
    if not file_ext_allowed(name):
        await _reject(me, client_id, "Jenis file tidak diizinkan. Hanya gambar, Word, Excel, dan PDF.")
        return
    if len(data) > MAX_FILE:
        await _reject(me, client_id, "File terlalu besar (maks 10 MB).")
        return
    if not await _check_target(me, client_id, to):
        return
    if not online(to):
        await _reject(me, client_id, "Toko tujuan sedang offline, file tidak terkirim.")
        return
    row = ChatMessage(
        from_code=me.code,
        to_code=to,
        kind="file",
        body=str(msg.get("body", "")).strip()[:MAX_TEXT],
        file_name=name,
        file_size=len(data),
        push=bool(msg.get("push")),
        delivered_at=utcnow(),  # file hanya dikirim kalau tujuan online
    )
    saved = await run_sync(_save_message, row)
    saved["client_id"] = client_id
    await _deliver(saved, me, data)


# ---------- REST ----------


async def _auth(request: Request) -> ChatStore | None:
    code, key = _auth_headers(request)
    return await run_sync(_verify, code, key)


def _unauthorized() -> JSONResponse:
    return JSONResponse({"error": "Kode toko atau Kunci Chat salah."}, status_code=401)


@router.get("/messages")
async def chat_messages(request: Request, since: datetime | None = None) -> JSONResponse:
    store = await _auth(request)
    if store is None:
        return _unauthorized()
    if since is not None and since.tzinfo is None:
        since = since.replace(tzinfo=timezone.utc)
    rows = await run_sync(_history, store.code, since)
    return JSONResponse({"me": store.code, "name": store.name, "messages": rows})


@router.get("/lookup")
async def chat_lookup(request: Request, code: str) -> JSONResponse:
    store = await _auth(request)
    if store is None:
        return _unauthorized()
    if not _rate_ok(_lookup_hits[store.code], 30, 60.0):
        return JSONResponse({"error": "Terlalu banyak pencarian. Tunggu sebentar."}, status_code=429)
    code = code.strip()
    name = await run_sync(_store_name, code) if CODE_RE.match(code) else None
    if name is None:
        return JSONResponse({"error": f"Kode toko '{code}' tidak terdaftar untuk chat."}, status_code=404)
    return JSONResponse({"code": code, "name": name, "online": online(code)})
