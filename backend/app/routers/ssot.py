"""Lihat & ubah isi SSoT langsung dari app desktop (layar Sync → SSoT).

Beda dengan `/sync/products/pull`: daftar di sini tidak mencatat SyncLog
(dibuka berkali-kali untuk menjelajah, bukan sinkronisasi). Simpan satu
barang memberi `updated_at` = waktu server, jadi toko lain ikut menariknya
lewat Sync In biasa.
"""

import uuid
from datetime import datetime, timezone

from fastapi import APIRouter, Depends, HTTPException, Query
from sqlalchemy import func, select
from sqlalchemy.orm import Session

from app.database import get_db
from app.models import Product, SyncLog
from app.schemas import ProductOut, SsotSaveRequest

router = APIRouter(prefix="/api/v1/ssot", tags=["ssot"])


@router.get("/products", response_model=list[ProductOut])
def list_products(
    db: Session = Depends(get_db),
    include_deleted: bool = Query(False, description="Ikut tampilkan produk yang sudah dihapus (tombstone)"),
) -> list[Product]:
    stmt = select(Product)
    if not include_deleted:
        stmt = stmt.where(Product.is_deleted.is_(False))
    stmt = stmt.order_by(func.lower(Product.name).asc())
    return list(db.execute(stmt).scalars().all())


@router.post("/products", response_model=ProductOut)
def save_product(payload: SsotSaveRequest, db: Session = Depends(get_db)) -> Product:
    p = payload.product
    name = p.name.strip()
    if not name:
        raise HTTPException(status_code=422, detail="Nama barang wajib diisi.")
    barcode = (p.barcode or "").strip() or None

    existing = db.get(Product, p.id) if p.id else None
    if p.id and existing is None:
        raise HTTPException(status_code=404, detail="Barang tidak ditemukan di SSoT.")

    if barcode:
        stmt = select(Product).where(Product.barcode == barcode, Product.is_deleted.is_(False))
        if existing is not None:
            stmt = stmt.where(Product.id != existing.id)
        clash = db.execute(stmt.limit(1)).scalars().first()
        if clash is not None:
            raise HTTPException(
                status_code=409, detail=f'Barcode "{barcode}" sudah dipakai oleh "{clash.name}" di SSoT.'
            )

    now = datetime.now(timezone.utc)
    if existing is None:
        existing = Product(id=str(uuid.uuid4()), is_deleted=False)
        db.add(existing)
        action = "baru"
    else:
        action = "ubah"

    existing.name = name
    existing.barcode = barcode
    existing.category = (p.category or "").strip() or None
    existing.brand = (p.brand or "").strip() or None
    existing.unit = (p.unit or "").strip() or None
    existing.sell_price = p.sell_price
    existing.cost_price = p.cost_price
    existing.default_discount = p.default_discount
    existing.is_active = p.is_active
    existing.updated_at = now

    db.add(
        SyncLog(
            store_id=payload.store_id,
            direction="upload",
            record_count=1,
            detail=f"SSoT {action}: {name}",
        )
    )
    db.commit()
    db.refresh(existing)
    return existing
