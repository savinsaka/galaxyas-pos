"""Uji layar SSoT & Hard Push mode "hanya merek terpilih".

    pip install -r requirements.txt httpx
    python test_ssot.py

Memakai SQLite sementara — tidak menyentuh PostgreSQL produksi.
"""

import os
import sys
import tempfile

DB = os.path.join(tempfile.gettempdir(), "gpos_test_ssot.db")
if os.path.exists(DB):
    os.remove(DB)
os.environ["DATABASE_URL"] = f"sqlite:///{DB}"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from fastapi.testclient import TestClient  # noqa: E402

from app.database import SessionLocal  # noqa: E402
from app.main import app  # noqa: E402
from app.models import SyncLog  # noqa: E402

results = []


def check(name, cond, detail=""):
    results.append((name, cond))
    print(("PASS  " if cond else "FAIL  ") + name + ("   " + str(detail) if detail else ""))


def produk(**kw):
    base = {"name": "Barang", "barcode": None, "category": None, "brand": "OMG", "unit": "pcs",
            "sell_price": 10000, "cost_price": 8000, "default_discount": 0, "is_active": True}
    base.update(kw)
    return base


with TestClient(app) as c:
    r = c.post("/api/v1/ssot/products", json={"store_id": "T1", "product": produk(name="Kopi", barcode="111")})
    check("tambah barang baru", r.status_code == 200, r.text)
    kopi = r.json()
    check("id baru dibuat server", len(kopi["id"]) == 36)

    r = c.post("/api/v1/ssot/products", json={"store_id": "T1", "product": produk(name="Teh", barcode="111")})
    check("barcode dobel ditolak 409", r.status_code == 409 and "Kopi" in r.json()["detail"], r.text)

    r = c.post("/api/v1/ssot/products", json={"store_id": "T1", "product": produk(name="  ", barcode="222")})
    check("nama kosong ditolak", r.status_code == 422)

    r = c.post(
        "/api/v1/ssot/products",
        json={"store_id": "T1", "product": produk(id=kopi["id"], name="Kopi Susu", barcode="111", sell_price=12000)},
    )
    check("ubah barang (barcode sendiri boleh)", r.status_code == 200 and r.json()["sell_price"] == 12000, r.text)
    check("updated_at maju", r.json()["updated_at"] > kopi["updated_at"])

    r = c.post("/api/v1/ssot/products", json={"store_id": "T1", "product": produk(id="tidak-ada", name="X")})
    check("id tak dikenal 404", r.status_code == 404)

    c.post("/api/v1/ssot/products", json={"store_id": "T1", "product": produk(name="Aqua", brand="LAIN", barcode="333")})
    n_log = SessionLocal().query(SyncLog).count()
    r = c.get("/api/v1/ssot/products")
    names = [p["name"] for p in r.json()]
    check("daftar urut nama", names == ["Aqua", "Kopi Susu"], names)
    check("daftar tidak menulis SyncLog", SessionLocal().query(SyncLog).count() == n_log)

    aqua = next(p for p in r.json() if p["name"] == "Aqua")
    kopi = next(p for p in r.json() if p["name"] == "Kopi Susu")
    body = {
        "store_id": "T1",
        "exclude_brands": [],
        "include_brands": ["omg"],
        "products": [
            {**kopi, "sell_price": 15000},
            {**aqua, "brand": "OMG", "sell_price": 15000},  # SSoT masih LAIN → jangan ditimpa
        ],
    }
    r = c.post("/api/v1/sync/products/hard-push", json=body)
    st = {x["id"]: x["status"] for x in r.json()["results"]}
    check("hard push include: merek terpilih ditimpa", st[kopi["id"]] == "applied", st)
    check("hard push include: merek SSoT lain dilewati", st[aqua["id"]] == "skipped_brand", st)

    body.pop("include_brands")
    r = c.post("/api/v1/sync/products/hard-push", json=body)
    check("hard push tanpa include tetap seperti dulu", all(x["status"] == "applied" for x in r.json()["results"]))

failed = [n for n, ok in results if not ok]
print(f"\n{len(results) - len(failed)}/{len(results)} lulus")
sys.exit(1 if failed else 0)
