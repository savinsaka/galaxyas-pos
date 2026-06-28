"""Endpoint Delta Sync master data.

Strategi:
- Delta Sync berbasis kolom `updated_at`.
- PULL: klien kirim `since` (waktu sync terakhir), server balas baris yang berubah sesudahnya.
- PUSH: klien kirim perubahan lokal; server terapkan **Last Write Wins** —
  baris hanya ditimpa bila `updated_at` yang masuk LEBIH BARU dari yang ada.
"""

from datetime import datetime, timezone

from fastapi import APIRouter, Depends, Query
from sqlalchemy import select
from sqlalchemy.orm import Session

from app.database import get_db
from app.models import Product, SyncLog
from app.schemas import (
    ProductOut,
    PullResponse,
    PushRequest,
    PushResponse,
    PushResultItem,
    SyncLogOut,
)

router = APIRouter(prefix="/api/v1/sync", tags=["sync"])


def _as_utc(dt: datetime) -> datetime:
    """Pastikan datetime memiliki tzinfo UTC agar perbandingan aman."""
    if dt.tzinfo is None:
        return dt.replace(tzinfo=timezone.utc)
    return dt.astimezone(timezone.utc)


@router.get("/products/pull", response_model=PullResponse)
def pull_products(
    db: Session = Depends(get_db),
    store_id: str = Query(..., description="Identitas toko"),
    since: datetime | None = Query(
        None, description="Ambil produk yang berubah setelah waktu ini (ISO 8601). Kosong = full sync."
    ),
) -> PullResponse:
    stmt = select(Product)
    if since is not None:
        stmt = stmt.where(Product.updated_at > _as_utc(since))
    stmt = stmt.order_by(Product.updated_at.asc())

    products = list(db.execute(stmt).scalars().all())
    server_time = datetime.now(timezone.utc)

    db.add(
        SyncLog(
            store_id=store_id,
            direction="download",
            record_count=len(products),
            detail=f"pull since={since.isoformat() if since else 'full'}",
        )
    )
    db.commit()

    return PullResponse(
        server_time=server_time,
        count=len(products),
        products=[ProductOut.model_validate(p) for p in products],
    )


@router.post("/products/push", response_model=PushResponse)
def push_products(payload: PushRequest, db: Session = Depends(get_db)) -> PushResponse:
    results: list[PushResultItem] = []
    applied = 0
    skipped = 0

    for incoming in payload.products:
        incoming_ts = _as_utc(incoming.updated_at)
        existing = db.get(Product, incoming.id)

        if existing is None:
            db.add(
                Product(
                    id=incoming.id,
                    name=incoming.name,
                    barcode=incoming.barcode,
                    category=incoming.category,
                    brand=incoming.brand,
                    unit=incoming.unit,
                    sell_price=incoming.sell_price,
                    cost_price=incoming.cost_price,
                    default_discount=incoming.default_discount,
                    is_active=incoming.is_active,
                    is_deleted=incoming.is_deleted,
                    updated_at=incoming_ts,
                )
            )
            applied += 1
            results.append(PushResultItem(id=incoming.id, status="created"))
            continue

        # Last Write Wins: hanya timpa bila data masuk lebih baru.
        if incoming_ts > _as_utc(existing.updated_at):
            existing.name = incoming.name
            existing.barcode = incoming.barcode
            existing.category = incoming.category
            existing.brand = incoming.brand
            existing.unit = incoming.unit
            existing.sell_price = incoming.sell_price
            existing.cost_price = incoming.cost_price
            existing.default_discount = incoming.default_discount
            existing.is_active = incoming.is_active
            existing.is_deleted = incoming.is_deleted
            existing.updated_at = incoming_ts
            applied += 1
            results.append(PushResultItem(id=incoming.id, status="applied"))
        else:
            skipped += 1
            results.append(PushResultItem(id=incoming.id, status="skipped_stale"))

    db.add(
        SyncLog(
            store_id=payload.store_id,
            direction="upload",
            record_count=applied,
            detail=f"push received={len(payload.products)} applied={applied} skipped={skipped}",
        )
    )
    db.commit()

    return PushResponse(
        server_time=datetime.now(timezone.utc),
        received=len(payload.products),
        applied=applied,
        skipped=skipped,
        results=results,
    )


@router.get("/logs", response_model=list[SyncLogOut])
def list_logs(
    db: Session = Depends(get_db),
    store_id: str | None = Query(None),
    limit: int = Query(50, le=500),
) -> list[SyncLog]:
    stmt = select(SyncLog)
    if store_id:
        stmt = stmt.where(SyncLog.store_id == store_id)
    stmt = stmt.order_by(SyncLog.created_at.desc()).limit(limit)
    return list(db.execute(stmt).scalars().all())
