"""Dependency login anggota untuk endpoint Sync & SSoT."""

import hashlib
from datetime import datetime, timedelta, timezone

from fastapi import Depends, HTTPException, Request
from sqlalchemy.orm import Session

from app.config import settings
from app.database import get_db
from app.models import Member, MemberToken

# `last_used_at` cukup diperbarui sesekali — sync bisa memanggil beberapa
# endpoint berturut-turut, tidak perlu menulis DB tiap request.
_TOUCH_EVERY = timedelta(minutes=5)


def hash_token(token: str) -> str:
    return hashlib.sha256(token.encode("utf-8")).hexdigest()


def _as_utc(dt: datetime) -> datetime:
    return dt.replace(tzinfo=timezone.utc) if dt.tzinfo is None else dt.astimezone(timezone.utc)


def _unauthorized(detail: str) -> HTTPException:
    return HTTPException(status_code=401, detail=detail, headers={"WWW-Authenticate": "Bearer"})


def bearer_token(request: Request) -> str | None:
    auth = request.headers.get("authorization") or ""
    scheme, _, token = auth.partition(" ")
    if scheme.lower() != "bearer" or not token.strip():
        return None
    return token.strip()


def member_from_token(db: Session, token: str) -> tuple[Member, MemberToken] | None:
    row = db.query(MemberToken).filter(MemberToken.token_hash == hash_token(token)).first()
    if row is None:
        return None
    member = db.get(Member, row.username)
    if member is None or not member.is_active:
        return None
    return member, row


def require_member(request: Request, db: Session = Depends(get_db)) -> Member:
    """Wajib login (dipakai endpoint akun sendiri, mis. logout)."""

    token = bearer_token(request)
    found = member_from_token(db, token) if token else None
    if found is None:
        raise _unauthorized("Belum login atau sesi sudah dicabut.")
    return found[0]


def member_or_legacy(request: Request, db: Session = Depends(get_db)) -> Member | None:
    """Anggota yang login; `None` = app lama tanpa login (hanya sebelum batas).

    Token yang dikirim tapi salah/dicabut selalu ditolak — tidak jatuh ke mode
    lama, supaya PC yang sudah dicabut aksesnya benar-benar berhenti.
    """

    token = bearer_token(request)
    if token is None:
        if datetime.now(timezone.utc) < _as_utc(settings.sync_auth_deadline):
            return None
        raise _unauthorized("Server ini sekarang wajib login. Update aplikasi lalu login di Sync → Akun Server.")

    found = member_from_token(db, token)
    if found is None:
        raise _unauthorized("Sesi server habis atau dicabut. Login ulang di Sync → Akun Server.")
    member, row = found
    now = datetime.now(timezone.utc)
    if row.last_used_at is None or now - _as_utc(row.last_used_at) > _TOUCH_EVERY:
        row.last_used_at = now
        db.commit()
    return member


def store_of(member: Member | None, claimed: str | None) -> str:
    """ID toko untuk log: dari akun bila login, dari isian klien bila mode lama."""

    if member is not None:
        return member.username
    return (claimed or "").strip() or "?"
