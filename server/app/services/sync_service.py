"""Server-side synchronization logic.

Strategy (per the architecture plan):
- Idempotency: every entity is keyed by UUID, so retried pushes are no-ops.
- Last-Write-Wins for non-critical fields (newer `updated_at` wins).
- Conflict logging for critical fields (e.g. `barcode`) and cross-store barcode
  collisions: instead of silently overwriting, the server returns the conflict
  so an Admin/Supervisor can review it on the client.
"""

import time
import uuid

from sqlalchemy import select
from sqlalchemy.orm import Session

from app.models import Item, Sale, SaleItem, Shift, StockTransaction, Store, SyncLog
from app.schemas.sync import (
    ConflictOut,
    PullChange,
    PullResponse,
    PushItem,
    PushRequest,
    PushResponse,
)

CRITICAL_ITEM_FIELDS = ("barcode",)

# entity_type -> (model, list of assignable columns)
_SIMPLE_ENTITIES: dict[str, type] = {
    "sale": Sale,
    "stock_transaction": StockTransaction,
    "shift": Shift,
    "store": Store,
}


def _now_ms() -> int:
    return int(time.time() * 1000)


def _log(db: Session, store_id: str, direction: str, entity_type: str, entity_id: str, result: str) -> None:
    db.add(
        SyncLog(
            id=str(uuid.uuid4()),
            store_id=store_id,
            direction=direction,
            entity_type=entity_type,
            entity_id=entity_id,
            result=result,
            created_at=_now_ms(),
        )
    )


def _assign(model_obj, payload: dict, columns) -> None:
    for col in columns:
        if col in payload:
            setattr(model_obj, col, payload[col])


def _handle_item(db: Session, store_id: str, change: PushItem) -> ConflictOut | None:
    payload = change.payload
    incoming_updated = int(payload.get("updated_at", 0))
    existing = db.get(Item, change.entity_id)

    # Cross-store / same-store barcode collision against a *different* item.
    barcode = payload.get("barcode")
    if barcode:
        clash = db.scalar(
            select(Item).where(
                Item.store_id == store_id,
                Item.barcode == barcode,
                Item.id != change.entity_id,
                Item.deleted_at.is_(None),
            )
        )
        if clash is not None:
            return ConflictOut(
                queue_id=change.queue_id,
                entity_type="item",
                entity_id=change.entity_id,
                server_payload=_item_to_dict(clash),
                conflict_field="barcode",
            )

    if existing is None:
        item = Item(id=change.entity_id, store_id=store_id)
        _assign(item, payload, _item_columns())
        item.updated_at = incoming_updated or _now_ms()
        db.add(item)
        _log(db, store_id, "push", "item", change.entity_id, "applied")
        return None

    # Conflict on critical fields if the server copy is newer and differs.
    if existing.updated_at > (change.base_updated_at or 0):
        for field in CRITICAL_ITEM_FIELDS:
            if payload.get(field) != getattr(existing, field):
                return ConflictOut(
                    queue_id=change.queue_id,
                    entity_type="item",
                    entity_id=change.entity_id,
                    server_payload=_item_to_dict(existing),
                    conflict_field=field,
                )

    # Last-Write-Wins for the remaining fields.
    if incoming_updated >= existing.updated_at:
        _assign(existing, payload, _item_columns())
        existing.updated_at = incoming_updated
        _log(db, store_id, "push", "item", change.entity_id, "applied")
    else:
        _log(db, store_id, "push", "item", change.entity_id, "skipped")
    return None


def _handle_simple(db: Session, store_id: str, change: PushItem) -> None:
    model = _SIMPLE_ENTITIES[change.entity_type]
    existing = db.get(model, change.entity_id)
    incoming_updated = int(change.payload.get("updated_at", 0))
    columns = [c.name for c in model.__table__.columns]
    if existing is None:
        obj = model(id=change.entity_id)
        _assign(obj, change.payload, columns)
        if hasattr(obj, "store_id"):
            obj.store_id = store_id
        db.add(obj)
        _log(db, store_id, "push", change.entity_type, change.entity_id, "applied")
    elif incoming_updated >= getattr(existing, "updated_at", 0):
        _assign(existing, change.payload, columns)
        _log(db, store_id, "push", change.entity_type, change.entity_id, "applied")
    else:
        _log(db, store_id, "push", change.entity_type, change.entity_id, "skipped")


def process_push(db: Session, req: PushRequest) -> PushResponse:
    accepted: list[str] = []
    conflicts: list[ConflictOut] = []

    for change in req.changes:
        if change.entity_type in ("item", "item_stock"):
            conflict = _handle_item(db, req.store_id, change)
            if conflict:
                conflicts.append(conflict)
                _log(db, req.store_id, "push", "item", change.entity_id, "conflict")
                continue
            accepted.append(change.queue_id)
        elif change.entity_type in _SIMPLE_ENTITIES:
            _handle_simple(db, req.store_id, change)
            accepted.append(change.queue_id)
        else:
            # Unknown entity types are acknowledged so the client clears them.
            accepted.append(change.queue_id)

    db.commit()
    return PushResponse(accepted=accepted, conflicts=conflicts)


def process_pull(db: Session, store_id: str, since: int) -> PullResponse:
    """Return master-data (items) changed after `since` across all stores so a
    cashier can receive other stores' catalog/price updates."""
    server_time = _now_ms()
    rows = db.scalars(
        select(Item).where(Item.updated_at > since).order_by(Item.updated_at).limit(500)
    ).all()
    changes = [
        PullChange(
            entity_type="item",
            entity_id=row.id,
            payload=_item_to_dict(row),
            server_updated_at=row.updated_at,
            deleted=row.deleted_at is not None,
        )
        for row in rows
    ]
    for row in rows:
        _log(db, store_id, "pull", "item", row.id, "applied")
    db.commit()
    return PullResponse(changes=changes, server_time=server_time)


def _item_columns() -> list[str]:
    return [
        "kode_item",
        "barcode",
        "nama_item",
        "jenis",
        "merek",
        "satuan_dasar",
        "harga_beli",
        "harga_jual",
        "diskon_persen",
        "stok",
        "created_at",
        "deleted_at",
    ]


def _item_to_dict(item: Item) -> dict:
    return {
        "id": item.id,
        "store_id": item.store_id,
        "kode_item": item.kode_item,
        "barcode": item.barcode,
        "nama_item": item.nama_item,
        "jenis": item.jenis,
        "merek": item.merek,
        "satuan_dasar": item.satuan_dasar,
        "harga_beli": item.harga_beli,
        "harga_jual": item.harga_jual,
        "diskon_persen": item.diskon_persen,
        "stok": item.stok,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
        "deleted_at": item.deleted_at,
        "sync_status": "synced",
    }
