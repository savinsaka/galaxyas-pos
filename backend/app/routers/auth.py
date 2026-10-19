"""Login anggota (toko/gudang) dari app, plus kebijakan wajib update.

Tidak ada endpoint daftar: akun dibuat admin di `/admin` → Kelola Anggota.
"""

import secrets
import time
from collections import defaultdict, deque

from fastapi import APIRouter, Depends, HTTPException, Query, Request
from pydantic import BaseModel
from sqlalchemy.orm import Session

from app.config import settings
from app.database import get_db
from app.deps import bearer_token, hash_token, require_member
from app.models import Member, MemberToken, utcnow
from app.security import hash_password, verify_password

router = APIRouter(prefix="/api/v1", tags=["auth"])

# Anti tebak password: maksimal 10 gagal per (username, IP) dalam 15 menit.
_MAX_FAILS = 10
_WINDOW_S = 15 * 60
_fails: dict[tuple[str, str], deque[float]] = defaultdict(deque)

# Dipakai untuk menyamakan waktu respons saat username tidak ada (PBKDF2 sengaja
# lambat), supaya tidak bisa menebak username mana yang terdaftar.
_DUMMY_HASH = hash_password(secrets.token_hex(8))


def _client_ip(request: Request) -> str:
    # Di belakang Caddy: entri TERAKHIR X-Forwarded-For ditulis Caddy sendiri.
    fwd = request.headers.get("x-forwarded-for")
    if fwd:
        return fwd.split(",")[-1].strip()
    return request.client.host if request.client else "?"


def _recent_fails(key: tuple[str, str]) -> deque[float]:
    q = _fails[key]
    cutoff = time.monotonic() - _WINDOW_S
    while q and q[0] < cutoff:
        q.popleft()
    return q


class LoginRequest(BaseModel):
    username: str
    password: str
    device: str | None = None


class LoginResponse(BaseModel):
    token: str
    username: str
    name: str


class MeResponse(BaseModel):
    username: str
    name: str


class AppPolicy(BaseModel):
    min_version: str
    deadline: str | None
    message: str


@router.post("/auth/login", response_model=LoginResponse)
def login(payload: LoginRequest, request: Request, db: Session = Depends(get_db)) -> LoginResponse:
    username = payload.username.strip()
    key = (username.lower(), _client_ip(request))
    fails = _recent_fails(key)
    if len(fails) >= _MAX_FAILS:
        raise HTTPException(status_code=429, detail="Terlalu banyak percobaan gagal. Coba lagi 15 menit lagi.")

    member = db.query(Member).filter(Member.username == username).first() if username else None
    ok = verify_password(payload.password, member.password_hash if member else _DUMMY_HASH)
    if member is None or not ok:
        fails.append(time.monotonic())
        raise HTTPException(status_code=401, detail="ID Toko atau password salah.")
    if not member.is_active:
        raise HTTPException(status_code=403, detail="Akun ini dinonaktifkan admin.")
    _fails.pop(key, None)

    token = secrets.token_urlsafe(32)
    db.add(
        MemberToken(
            token_hash=hash_token(token),
            username=member.username,
            device=(payload.device or "").strip()[:120] or None,
            last_used_at=utcnow(),
        )
    )
    db.commit()
    return LoginResponse(token=token, username=member.username, name=member.name)


@router.post("/auth/logout")
def logout(request: Request, member: Member = Depends(require_member), db: Session = Depends(get_db)) -> dict:
    token = bearer_token(request) or ""
    db.query(MemberToken).filter(MemberToken.token_hash == hash_token(token)).delete()
    db.commit()
    return {"ok": True}


@router.get("/auth/me", response_model=MeResponse)
def me(member: Member = Depends(require_member)) -> MeResponse:
    return MeResponse(username=member.username, name=member.name)


@router.get("/app/policy", response_model=AppPolicy)
def app_policy(app: str = Query("pos", pattern="^(pos|warehouse)$")) -> AppPolicy:
    """Publik (dibaca sebelum login): versi minimum app & batas waktunya."""

    min_version = settings.warehouse_min_version if app == "warehouse" else settings.pos_min_version
    deadline = settings.app_update_deadline
    return AppPolicy(
        min_version=min_version.strip(),
        deadline=deadline.isoformat() if deadline else None,
        message=settings.app_update_message,
    )
