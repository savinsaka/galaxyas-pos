"""Uji login anggota Sync/SSoT: login, anti tebak, cabut sesi, batas app lama,
dokumentasi API tertutup, dan kebijakan wajib update.

    pip install -r requirements.txt httpx
    python test_auth.py

Memakai SQLite sementara — tidak menyentuh PostgreSQL produksi.
"""

import os
import sys
import tempfile
from datetime import datetime, timedelta, timezone

DB = os.path.join(tempfile.gettempdir(), "gpos_test_auth.db")
if os.path.exists(DB):
    os.remove(DB)
os.environ["DATABASE_URL"] = f"sqlite:///{DB}"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from fastapi.testclient import TestClient  # noqa: E402

from app.config import settings  # noqa: E402
from app.database import SessionLocal  # noqa: E402
from app.main import app  # noqa: E402
from app.models import Member, MemberToken, SyncLog  # noqa: E402
from app.routers import auth  # noqa: E402
from app.security import hash_password  # noqa: E402

results = []


def check(name, cond, detail=""):
    results.append((name, cond))
    print(("PASS  " if cond else "FAIL  ") + name + ("   " + str(detail) if detail else ""))


def bearer(token):
    return {"Authorization": f"Bearer {token}"}


SEBELUM = datetime.now(timezone.utc) + timedelta(hours=1)
SESUDAH = datetime.now(timezone.utc) - timedelta(hours=1)

with TestClient(app) as c:
    with SessionLocal() as db:
        db.add(Member(username="toko-001", name="Toko Depan", password_hash=hash_password("rahasia-123")))
        db.add(Member(username="toko-002", name="Toko Belakang", password_hash=hash_password("rahasia-456")))
        db.commit()

    for path in ("/docs", "/redoc", "/openapi.json"):
        check(f"{path} tertutup", c.get(path).status_code == 404)

    r = c.post("/api/v1/auth/login", json={"username": "toko-001", "password": "salah"})
    check("password salah ditolak", r.status_code == 401, r.text)
    r = c.post("/api/v1/auth/login", json={"username": "tidak-ada", "password": "x"})
    check("username tak dikenal ditolak dengan pesan sama", r.status_code == 401 and "salah" in r.json()["detail"])

    r = c.post("/api/v1/auth/login", json={"username": "toko-001", "password": "rahasia-123", "device": "KASIR-1"})
    check("login benar", r.status_code == 200 and r.json()["name"] == "Toko Depan", r.text)
    tok = r.json()["token"]
    with SessionLocal() as db:
        row = db.query(MemberToken).first()
        check("token disimpan sebagai hash saja", row.token_hash != tok and len(row.token_hash) == 64)

    check("me dengan token", c.get("/api/v1/auth/me", headers=bearer(tok)).json()["username"] == "toko-001")
    check("me tanpa token 401", c.get("/api/v1/auth/me").status_code == 401)

    settings.sync_auth_deadline = SEBELUM
    check("sebelum batas: app lama tanpa login masih boleh", c.get("/api/v1/sync/products/pull?store_id=lama").status_code == 200)
    check("sebelum batas: token palsu tetap ditolak", c.get("/api/v1/ssot/products", headers=bearer("palsu")).status_code == 401)

    settings.sync_auth_deadline = SESUDAH
    r = c.get("/api/v1/sync/products/pull?store_id=lama")
    check("sesudah batas: tanpa login ditolak", r.status_code == 401 and "wajib login" in r.json()["detail"], r.text)
    check("sesudah batas: ssot tanpa login ditolak", c.get("/api/v1/ssot/products").status_code == 401)
    check("sesudah batas: login tetap jalan", c.get("/api/v1/ssot/products", headers=bearer(tok)).status_code == 200)

    r = c.post(
        "/api/v1/sync/products/push",
        headers=bearer(tok),
        json={"store_id": "ngaku-toko-lain", "products": []},
    )
    with SessionLocal() as db:
        last = db.query(SyncLog).order_by(SyncLog.id.desc()).first()
    check("store_id log diambil dari akun, bukan isian klien", r.status_code == 200 and last.store_id == "toko-001", last.store_id)

    # Nonaktifkan anggota → token lamanya langsung mati.
    with SessionLocal() as db:
        db.get(Member, "toko-001").is_active = False
        db.commit()
    check("anggota nonaktif: token ditolak", c.get("/api/v1/ssot/products", headers=bearer(tok)).status_code == 401)
    r = c.post("/api/v1/auth/login", json={"username": "toko-001", "password": "rahasia-123"})
    check("anggota nonaktif: tidak bisa login", r.status_code == 403)

    r = c.post("/api/v1/auth/login", json={"username": "toko-002", "password": "rahasia-456"})
    tok2 = r.json()["token"]
    check("logout", c.post("/api/v1/auth/logout", headers=bearer(tok2)).status_code == 200)
    check("token sesudah logout ditolak", c.get("/api/v1/ssot/products", headers=bearer(tok2)).status_code == 401)

    auth._fails.clear()
    codes = [c.post("/api/v1/auth/login", json={"username": "toko-002", "password": "x"}).status_code for _ in range(11)]
    check("10 gagal lalu dikunci 429", codes[:10] == [401] * 10 and codes[10] == 429, codes)
    r = c.post("/api/v1/auth/login", json={"username": "toko-002", "password": "rahasia-456"})
    check("password benar pun ditahan selama terkunci", r.status_code == 429)
    auth._fails.clear()

    r = c.get("/api/v1/app/policy?app=pos")
    check("policy kosong = tidak wajib update", r.status_code == 200 and r.json()["min_version"] == "", r.text)
    settings.pos_min_version = "1.8.0"
    settings.app_update_deadline = SESUDAH
    r = c.get("/api/v1/app/policy?app=pos").json()
    check("policy POS terisi", r["min_version"] == "1.8.0" and r["deadline"], r)
    check("policy warehouse terpisah", c.get("/api/v1/app/policy?app=warehouse").json()["min_version"] == "")
    check("policy app tak dikenal 422", c.get("/api/v1/app/policy?app=lain").status_code == 422)

failed = [n for n, ok in results if not ok]
print(f"\n{len(results) - len(failed)}/{len(results)} lulus")
sys.exit(1 if failed else 0)
