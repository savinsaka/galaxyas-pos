from contextlib import asynccontextmanager

from fastapi import FastAPI
from fastapi.middleware.cors import CORSMiddleware

from app.admin import setup_admin
from app.config import settings
from app.database import Base, engine
from app.routers import sync


@asynccontextmanager
async def lifespan(app: FastAPI):
    # Untuk produksi sebaiknya pakai migrasi (Alembic). Untuk scaffold: auto-create.
    Base.metadata.create_all(bind=engine)
    yield


app = FastAPI(
    title="GALAXYAS POS - Sync Server",
    description="Backend hanya untuk sinkronisasi master data (Delta Sync, Last Write Wins).",
    version="0.1.0",
    lifespan=lifespan,
)

app.add_middleware(
    CORSMiddleware,
    allow_origins=settings.cors_origin_list,
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.include_router(sync.router)
setup_admin(app)


@app.get("/health", tags=["meta"])
def health() -> dict[str, str]:
    return {"status": "ok", "service": "galaxyas-sync"}
