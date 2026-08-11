from datetime import datetime, timezone

from sqlalchemy import Boolean, DateTime, Integer, Numeric, String, Text
from sqlalchemy.orm import Mapped, mapped_column

from app.database import Base


def utcnow() -> datetime:
    return datetime.now(timezone.utc)


class Product(Base):
    """Master data barang. Source of truth ada di PostgreSQL ini.

    `id` berupa UUID string yang dipakai bersama oleh server & SQLite lokal,
    sehingga delta sync dapat mencocokkan baris lintas database.
    Hanya kolom master data yang disimpan di sini (stok TIDAK di-sync).
    """

    __tablename__ = "products"

    id: Mapped[str] = mapped_column(String(36), primary_key=True)
    name: Mapped[str] = mapped_column(String(255), nullable=False)
    barcode: Mapped[str | None] = mapped_column(String(64), index=True, nullable=True)
    category: Mapped[str | None] = mapped_column(String(120), nullable=True)
    brand: Mapped[str | None] = mapped_column(String(120), nullable=True)
    unit: Mapped[str | None] = mapped_column(String(40), nullable=True)
    sell_price: Mapped[float] = mapped_column(Numeric(14, 2), default=0, nullable=False)
    cost_price: Mapped[float] = mapped_column(Numeric(14, 2), default=0, nullable=False)
    default_discount: Mapped[float] = mapped_column(Numeric(14, 2), default=0, nullable=False)
    is_active: Mapped[bool] = mapped_column(Boolean, default=True, nullable=False)

    # Tombstone untuk menyebarkan penghapusan via sync (soft delete).
    is_deleted: Mapped[bool] = mapped_column(Boolean, default=False, nullable=False)

    # Dipakai untuk Delta Sync (filter `since`) & Last Write Wins.
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False, index=True
    )


class AdminUser(Base):
    """Akun untuk login panel `/admin`.

    Password disimpan sebagai hash (lihat `app.security`), bukan di `.env`,
    supaya bisa diganti sendiri lewat halaman Profil tanpa SSH ke VPS. Nilai
    `ADMIN_USERNAME`/`ADMIN_PASSWORD` di `.env` hanya dipakai sekali untuk
    membuat akun pertama saat tabel ini masih kosong.
    """

    __tablename__ = "admin_users"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    username: Mapped[str] = mapped_column(String(64), unique=True, index=True, nullable=False)
    full_name: Mapped[str | None] = mapped_column(String(120), nullable=True)
    password_hash: Mapped[str] = mapped_column(String(255), nullable=False)
    last_login_at: Mapped[datetime | None] = mapped_column(DateTime(timezone=True), nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, nullable=False
    )
    updated_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, onupdate=utcnow, nullable=False
    )


class SyncLog(Base):
    """Riwayat sinkronisasi (audit), per toko & arah."""

    __tablename__ = "sync_logs"

    id: Mapped[int] = mapped_column(Integer, primary_key=True, autoincrement=True)
    store_id: Mapped[str] = mapped_column(String(64), index=True, nullable=False)
    direction: Mapped[str] = mapped_column(String(16), nullable=False)  # "upload" | "download"
    record_count: Mapped[int] = mapped_column(Integer, default=0, nullable=False)
    detail: Mapped[str | None] = mapped_column(Text, nullable=True)
    created_at: Mapped[datetime] = mapped_column(
        DateTime(timezone=True), default=utcnow, nullable=False, index=True
    )
