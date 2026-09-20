"""GALAXYAS Gudang Bridge — server minimal penghubung GPOS Warehouse (gudang)
ke banyak POS toko. Menggantikan galaxyas-mobile untuk kebutuhan kirim-barang.

Dua sisi, auth pakai API key mesin (tanpa akun/email/password):
- PENGIRIM (gudang):  header `X-Warehouse-Key`
    GET  /api/v1/stores                      -> daftar toko tujuan
    POST /api/v1/order-rows                   -> buat baris kiriman (batch, atomik)
- TOKO (POS pull):    header `X-Store-Api-Key`
    GET  /api/v1/bridge/order-rows            -> baris pending buat toko ini
    POST /api/v1/bridge/order-rows/confirm    -> konfirmasi (boleh koreksi qty)
    POST /api/v1/bridge/order-rows/{id}/reject
  (bentuk endpoint & field DIPERTAHANKAN kompatibel dgn galaxyas-pos/desktop
   src-tauri/src/pull.rs, jadi POS lama nyaris tanpa ubahan — cukup ganti key.)

Admin (lihat/rotasi key), header `X-Admin-Token`:
    GET  /api/v1/admin/keys
"""

import os
import secrets
import uuid

from fastapi import Depends, FastAPI, Header, HTTPException, status
from fastapi.middleware.cors import CORSMiddleware
from pydantic import BaseModel, Field

from app import db

app = FastAPI(title="GALAXYAS Gudang Bridge", version="1.0.0")

# Machine-to-machine; permisif saja (yang jaga = API key, bukan origin).
app.add_middleware(
    CORSMiddleware, allow_origins=["*"], allow_methods=["*"], allow_headers=["*"]
)


@app.on_event("startup")
def _startup() -> None:
    db.init_db()


# ---------------- Auth deps ----------------

def require_warehouse_key(x_warehouse_key: str | None = Header(default=None)) -> None:
    expected = db.get_config("warehouse_key")
    if not x_warehouse_key or not expected or not secrets.compare_digest(x_warehouse_key, expected):
        raise HTTPException(status.HTTP_401_UNAUTHORIZED, "X-Warehouse-Key tidak valid")


def get_store_from_key(x_store_api_key: str | None = Header(default=None)) -> dict:
    if not x_store_api_key:
        raise HTTPException(status.HTTP_401_UNAUTHORIZED, "Header X-Store-Api-Key wajib diisi")
    row = db.get_conn().execute(
        "SELECT * FROM stores WHERE api_key=? AND active=1", (x_store_api_key,)
    ).fetchone()
    if row is None:
        raise HTTPException(status.HTTP_401_UNAUTHORIZED, "API key toko tidak valid")
    return dict(row)


def require_admin(x_admin_token: str | None = Header(default=None)) -> None:
    expected = os.environ.get("ADMIN_TOKEN") or db.get_config("admin_token")
    if not x_admin_token or not expected or not secrets.compare_digest(x_admin_token, expected):
        raise HTTPException(status.HTTP_401_UNAUTHORIZED, "X-Admin-Token tidak valid")


# ---------------- Schemas ----------------

class StoreOut(BaseModel):
    id: str
    code: str
    name: str


class ShipItemIn(BaseModel):
    barcode: str = Field(min_length=1)
    qty: float = Field(gt=0)
    name: str | None = None


class ShipRequest(BaseModel):
    store_id: str
    items: list[ShipItemIn]


class BridgeOrderRowOut(BaseModel):
    id: str
    barcode: str
    name: str | None
    qty_dikirim: float


class ConfirmItem(BaseModel):
    id: str
    qty_confirmed: float = Field(ge=0)


class ConfirmRequest(BaseModel):
    items: list[ConfirmItem]


# ---------------- Health ----------------

@app.get("/")
def root() -> dict:
    return {"service": "galaxyas-gudang-bridge", "ok": True}


@app.get("/api/v1/health")
def health() -> dict:
    return {"ok": True}


# ---------------- Sisi PENGIRIM (gudang) ----------------

@app.get("/api/v1/stores", response_model=list[StoreOut], dependencies=[Depends(require_warehouse_key)])
def list_stores() -> list[dict]:
    rows = db.get_conn().execute(
        "SELECT id, code, name FROM stores WHERE active=1 ORDER BY code"
    ).fetchall()
    return [dict(r) for r in rows]


@app.post("/api/v1/order-rows", dependencies=[Depends(require_warehouse_key)])
def create_order_rows(payload: ShipRequest) -> dict:
    conn = db.get_conn()
    store = conn.execute("SELECT id FROM stores WHERE id=? AND active=1", (payload.store_id,)).fetchone()
    if store is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "Toko tujuan tidak ditemukan / nonaktif")
    if not payload.items:
        raise HTTPException(status.HTTP_400_BAD_REQUEST, "Tidak ada item")

    ts = db.now_iso()
    created = []
    for it in payload.items:
        rid = uuid.uuid4().hex
        conn.execute(
            "INSERT INTO order_rows(id, store_id, barcode, name, qty_dikirim, status, created_at, updated_at)"
            " VALUES(?,?,?,?,?, 'pending', ?, ?)",
            (rid, payload.store_id, it.barcode.strip(), it.name, it.qty, ts, ts),
        )
        created.append(rid)
    conn.commit()  # atomik: semua baris satu batch, tidak ada dobel-separuh
    return {"created": len(created), "ids": created}


# ---------------- Sisi TOKO (POS pull) ----------------

@app.get("/api/v1/bridge/order-rows", response_model=list[BridgeOrderRowOut])
def bridge_list(store: dict = Depends(get_store_from_key)) -> list[dict]:
    rows = db.get_conn().execute(
        "SELECT id, barcode, name, qty_dikirim FROM order_rows"
        " WHERE store_id=? AND status='pending' AND barcode IS NOT NULL AND qty_dikirim > 0"
        " ORDER BY updated_at ASC",
        (store["id"],),
    ).fetchall()
    return [dict(r) for r in rows]


@app.post("/api/v1/bridge/order-rows/confirm", response_model=list[BridgeOrderRowOut])
def bridge_confirm(payload: ConfirmRequest, store: dict = Depends(get_store_from_key)) -> list[dict]:
    conn = db.get_conn()
    out = []
    for item in payload.items:
        row = conn.execute("SELECT * FROM order_rows WHERE id=?", (item.id,)).fetchone()
        if row is None or row["store_id"] != store["id"]:
            raise HTTPException(status.HTTP_404_NOT_FOUND, f"Baris {item.id} tidak ditemukan")
        if row["status"] != "pending":
            raise HTTPException(status.HTTP_409_CONFLICT, f"Baris {item.id} sudah '{row['status']}'")
        conn.execute(
            "UPDATE order_rows SET qty_dikirim=?, status='confirmed', updated_at=? WHERE id=?",
            (item.qty_confirmed, db.now_iso(), item.id),
        )
        out.append({"id": row["id"], "barcode": row["barcode"], "name": row["name"], "qty_dikirim": item.qty_confirmed})
    conn.commit()
    return out


@app.post("/api/v1/bridge/order-rows/{row_id}/reject", response_model=BridgeOrderRowOut)
def bridge_reject(row_id: str, store: dict = Depends(get_store_from_key)) -> dict:
    conn = db.get_conn()
    row = conn.execute("SELECT * FROM order_rows WHERE id=?", (row_id,)).fetchone()
    if row is None or row["store_id"] != store["id"]:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "Baris tidak ditemukan")
    if row["status"] != "pending":
        raise HTTPException(status.HTTP_409_CONFLICT, f"Baris sudah '{row['status']}'")
    conn.execute("UPDATE order_rows SET status='rejected', updated_at=? WHERE id=?", (db.now_iso(), row_id))
    conn.commit()
    return {"id": row["id"], "barcode": row["barcode"], "name": row["name"], "qty_dikirim": row["qty_dikirim"]}


# ---------------- Admin (lihat/rotasi key) ----------------

@app.get("/api/v1/admin/keys", dependencies=[Depends(require_admin)])
def admin_keys() -> dict:
    stores = db.get_conn().execute("SELECT code, name, api_key, active FROM stores ORDER BY code").fetchall()
    return {
        "warehouse_key": db.get_config("warehouse_key"),
        "stores": [dict(s) for s in stores],
    }


@app.post("/api/v1/admin/stores/{store_id}/rotate-key", dependencies=[Depends(require_admin)])
def rotate_store_key(store_id: str) -> dict:
    conn = db.get_conn()
    row = conn.execute("SELECT id FROM stores WHERE id=?", (store_id,)).fetchone()
    if row is None:
        raise HTTPException(status.HTTP_404_NOT_FOUND, "Toko tidak ditemukan")
    new_key = secrets.token_urlsafe(32)
    conn.execute("UPDATE stores SET api_key=? WHERE id=?", (new_key, store_id))
    conn.commit()
    return {"store_id": store_id, "api_key": new_key}
