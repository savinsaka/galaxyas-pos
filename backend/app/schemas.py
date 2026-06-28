from datetime import datetime

from pydantic import BaseModel, ConfigDict, Field


class ProductBase(BaseModel):
    name: str
    barcode: str | None = None
    category: str | None = None
    brand: str | None = None
    unit: str | None = None
    sell_price: float = 0
    cost_price: float = 0
    default_discount: float = 0
    is_active: bool = True
    is_deleted: bool = False


class ProductIn(ProductBase):
    """Produk yang dikirim klien saat upload (push)."""

    id: str
    updated_at: datetime


class ProductOut(ProductBase):
    """Produk yang dikembalikan server saat download (pull)."""

    model_config = ConfigDict(from_attributes=True)

    id: str
    updated_at: datetime


class PullResponse(BaseModel):
    server_time: datetime
    count: int
    products: list[ProductOut]


class PushRequest(BaseModel):
    store_id: str = Field(..., description="Identitas toko pengirim")
    products: list[ProductIn]


class PushResultItem(BaseModel):
    id: str
    status: str  # "applied" | "skipped_stale" | "created"


class PushResponse(BaseModel):
    server_time: datetime
    received: int
    applied: int
    skipped: int
    results: list[PushResultItem]


class SyncLogOut(BaseModel):
    model_config = ConfigDict(from_attributes=True)

    id: int
    store_id: str
    direction: str
    record_count: int
    detail: str | None
    created_at: datetime
