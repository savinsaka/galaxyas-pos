"""Isi data contoh master barang ke PostgreSQL untuk uji coba sinkronisasi.

Jalankan: python seed.py
"""

import uuid
from datetime import datetime, timezone

from app.database import Base, SessionLocal, engine
from app.models import Product

SAMPLE = [
    # (name, barcode, category, brand, unit, sell, cost)
    ("Indomie Goreng", "8992388101010", "Mie Instan", "Indofood", "pcs", 3500, 2800),
    ("Aqua 600ml", "8993675001020", "Minuman", "Aqua", "botol", 4000, 3000),
    ("Teh Botol Sosro", "8992761123031", "Minuman", "Sosro", "botol", 5000, 3800),
    ("Beras Pandan 5kg", "2000000000041", "Sembako", "Lokal", "karung", 68000, 60000),
    ("Minyak Goreng 1L", "8990001112052", "Sembako", "Bimoli", "botol", 18000, 16000),
    ("Sabun Lifebuoy", "8999999563063", "Toiletries", "Unilever", "pcs", 4500, 3500),
]


def main() -> None:
    Base.metadata.create_all(bind=engine)
    now = datetime.now(timezone.utc)
    with SessionLocal() as db:
        for name, barcode, category, brand, unit, sell, cost in SAMPLE:
            exists = db.query(Product).filter(Product.barcode == barcode).first()
            if exists:
                continue
            db.add(
                Product(
                    id=str(uuid.uuid4()),
                    name=name,
                    barcode=barcode,
                    category=category,
                    brand=brand,
                    unit=unit,
                    sell_price=sell,
                    cost_price=cost,
                    default_discount=0,
                    is_active=True,
                    is_deleted=False,
                    updated_at=now,
                )
            )
        db.commit()
    print("Seed selesai.")


if __name__ == "__main__":
    main()
