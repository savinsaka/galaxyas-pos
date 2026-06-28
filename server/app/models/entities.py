"""Server-side ORM models (MySQL 8).

These mirror the local SQLite schema. UUIDs are stored as CHAR(36) and all
timestamps are epoch milliseconds (BIGINT) to match the desktop clients and to
make Last-Write-Wins comparisons trivial across stores.
"""

from sqlalchemy import (
    BigInteger,
    Boolean,
    CHAR,
    Float,
    Index,
    String,
    Text,
    UniqueConstraint,
)
from sqlalchemy.orm import Mapped, mapped_column

from app.core.database import Base


class Store(Base):
    __tablename__ = "stores"

    id: Mapped[str] = mapped_column(CHAR(36), primary_key=True)
    name: Mapped[str] = mapped_column(String(200))
    address: Mapped[str | None] = mapped_column(String(255), nullable=True)
    phone: Mapped[str | None] = mapped_column(String(50), nullable=True)
    tax_percent: Mapped[float] = mapped_column(Float, default=0)
    created_at: Mapped[int] = mapped_column(BigInteger)
    updated_at: Mapped[int] = mapped_column(BigInteger)


class User(Base):
    __tablename__ = "users"

    id: Mapped[str] = mapped_column(CHAR(36), primary_key=True)
    store_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    username: Mapped[str] = mapped_column(String(80), unique=True)
    full_name: Mapped[str] = mapped_column(String(120))
    password_hash: Mapped[str] = mapped_column(String(255))
    role: Mapped[str] = mapped_column(String(20))
    is_active: Mapped[bool] = mapped_column(Boolean, default=True)
    created_at: Mapped[int] = mapped_column(BigInteger)
    updated_at: Mapped[int] = mapped_column(BigInteger)


class Item(Base):
    __tablename__ = "items"
    __table_args__ = (
        # Barcode unique per store (operational scope; cross-store conflicts are
        # detected and logged, not blocked, during consolidation).
        UniqueConstraint("store_id", "barcode", name="uq_item_store_barcode"),
        Index("ix_item_barcode", "barcode"),
        Index("ix_item_updated", "updated_at"),
    )

    id: Mapped[str] = mapped_column(CHAR(36), primary_key=True)
    store_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    kode_item: Mapped[str | None] = mapped_column(String(50), nullable=True)
    barcode: Mapped[str | None] = mapped_column(String(50), nullable=True)
    nama_item: Mapped[str] = mapped_column(String(200))
    jenis: Mapped[str | None] = mapped_column(String(100), nullable=True)
    merek: Mapped[str | None] = mapped_column(String(100), nullable=True)
    satuan_dasar: Mapped[str | None] = mapped_column(String(50), nullable=True)
    harga_beli: Mapped[float] = mapped_column(Float, default=0)
    harga_jual: Mapped[float] = mapped_column(Float, default=0)
    diskon_persen: Mapped[float] = mapped_column(Float, default=0)
    stok: Mapped[float] = mapped_column(Float, default=0)
    created_at: Mapped[int] = mapped_column(BigInteger)
    updated_at: Mapped[int] = mapped_column(BigInteger)
    deleted_at: Mapped[int | None] = mapped_column(BigInteger, nullable=True)


class StockTransaction(Base):
    __tablename__ = "stock_transactions"

    id: Mapped[str] = mapped_column(CHAR(36), primary_key=True)
    store_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    item_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    type: Mapped[str] = mapped_column(String(20))
    qty: Mapped[float] = mapped_column(Float)
    qty_before: Mapped[float] = mapped_column(Float)
    qty_after: Mapped[float] = mapped_column(Float)
    ref_doc: Mapped[str | None] = mapped_column(String(100), nullable=True)
    note: Mapped[str | None] = mapped_column(String(255), nullable=True)
    user_id: Mapped[str] = mapped_column(CHAR(36))
    created_at: Mapped[int] = mapped_column(BigInteger)
    updated_at: Mapped[int] = mapped_column(BigInteger)


class Sale(Base):
    __tablename__ = "sales"
    __table_args__ = (Index("ix_sale_updated", "updated_at"),)

    id: Mapped[str] = mapped_column(CHAR(36), primary_key=True)
    store_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    invoice_no: Mapped[str | None] = mapped_column(String(50), nullable=True)
    user_id: Mapped[str] = mapped_column(CHAR(36))
    shift_id: Mapped[str | None] = mapped_column(CHAR(36), nullable=True)
    subtotal: Mapped[float] = mapped_column(Float, default=0)
    diskon: Mapped[float] = mapped_column(Float, default=0)
    pajak: Mapped[float] = mapped_column(Float, default=0)
    total: Mapped[float] = mapped_column(Float, default=0)
    bayar: Mapped[float] = mapped_column(Float, default=0)
    kembali: Mapped[float] = mapped_column(Float, default=0)
    status: Mapped[str] = mapped_column(String(20))
    created_at: Mapped[int] = mapped_column(BigInteger)
    updated_at: Mapped[int] = mapped_column(BigInteger)
    deleted_at: Mapped[int | None] = mapped_column(BigInteger, nullable=True)


class SaleItem(Base):
    __tablename__ = "sale_items"

    id: Mapped[str] = mapped_column(CHAR(36), primary_key=True)
    sale_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    item_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    nama_item: Mapped[str] = mapped_column(String(200))
    qty: Mapped[float] = mapped_column(Float)
    harga: Mapped[float] = mapped_column(Float)
    diskon: Mapped[float] = mapped_column(Float, default=0)
    subtotal: Mapped[float] = mapped_column(Float)
    created_at: Mapped[int] = mapped_column(BigInteger)
    updated_at: Mapped[int] = mapped_column(BigInteger)


class Shift(Base):
    __tablename__ = "shifts"

    id: Mapped[str] = mapped_column(CHAR(36), primary_key=True)
    store_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    user_id: Mapped[str] = mapped_column(CHAR(36))
    opening_cash: Mapped[float] = mapped_column(Float, default=0)
    closing_cash: Mapped[float | None] = mapped_column(Float, nullable=True)
    expected_cash: Mapped[float | None] = mapped_column(Float, nullable=True)
    total_sales: Mapped[float] = mapped_column(Float, default=0)
    opened_at: Mapped[int] = mapped_column(BigInteger)
    closed_at: Mapped[int | None] = mapped_column(BigInteger, nullable=True)
    status: Mapped[str] = mapped_column(String(20))
    created_at: Mapped[int] = mapped_column(BigInteger)
    updated_at: Mapped[int] = mapped_column(BigInteger)


class AuditLog(Base):
    __tablename__ = "audit_logs"

    id: Mapped[str] = mapped_column(CHAR(36), primary_key=True)
    store_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    user_id: Mapped[str] = mapped_column(CHAR(36))
    action: Mapped[str] = mapped_column(String(80))
    entity_type: Mapped[str | None] = mapped_column(String(50), nullable=True)
    entity_id: Mapped[str | None] = mapped_column(CHAR(36), nullable=True)
    detail: Mapped[str | None] = mapped_column(Text, nullable=True)
    created_at: Mapped[int] = mapped_column(BigInteger)


class SyncLog(Base):
    """Server-only audit of every push/pull operation per store."""

    __tablename__ = "sync_log"

    id: Mapped[str] = mapped_column(CHAR(36), primary_key=True)
    store_id: Mapped[str] = mapped_column(CHAR(36), index=True)
    direction: Mapped[str] = mapped_column(String(10))  # 'push' | 'pull'
    entity_type: Mapped[str] = mapped_column(String(50))
    entity_id: Mapped[str] = mapped_column(CHAR(36))
    result: Mapped[str] = mapped_column(String(20))  # 'applied' | 'conflict' | 'skipped'
    created_at: Mapped[int] = mapped_column(BigInteger)
