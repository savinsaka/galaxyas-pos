"""Seed the central database with a default store and admin user.

Usage (from the server/ directory, with .env configured):
    python -m scripts.seed
"""

import time
import uuid

from app.core.database import SessionLocal
from app.core.security import hash_password
from app.models import Store, User

DEFAULT_STORE_ID = "store-001"


def main() -> None:
    db = SessionLocal()
    now = int(time.time() * 1000)
    try:
        if db.get(Store, DEFAULT_STORE_ID) is None:
            db.add(
                Store(
                    id=DEFAULT_STORE_ID,
                    name="GalaxyAS Toko Pusat",
                    address="Jl. Merdeka No. 1",
                    phone="021-0000000",
                    tax_percent=11,
                    created_at=now,
                    updated_at=now,
                )
            )

        from sqlalchemy import select

        if db.scalar(select(User).where(User.username == "admin")) is None:
            db.add(
                User(
                    id=str(uuid.uuid4()),
                    store_id=DEFAULT_STORE_ID,
                    username="admin",
                    full_name="Administrator",
                    password_hash=hash_password("admin123"),
                    role="admin",
                    is_active=True,
                    created_at=now,
                    updated_at=now,
                )
            )
        db.commit()
        print("Seed complete. Login: admin / admin123")
    finally:
        db.close()


if __name__ == "__main__":
    main()
