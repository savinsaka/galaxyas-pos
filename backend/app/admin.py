"""Panel admin (/admin) untuk lihat & ubah data server langsung dari browser.

Sengaja pakai SQLAdmin (nempel di proses FastAPI yang sama, render halaman
per-request lewat Jinja2) — TIDAK ada proses/worker/queue tambahan, TIDAK ada
polling, TIDAK ada build frontend terpisah. Server tetap seringan sebelumnya;
panel ini cuma menjawab request yang datang, sama seperti endpoint sync.
"""

from starlette.requests import Request
from sqladmin import Admin, ModelView
from sqladmin.authentication import AuthenticationBackend

from app.config import settings
from app.database import engine
from app.models import Product, SyncLog


class AdminAuth(AuthenticationBackend):
    """Login sederhana (username/password dari .env) — cukup untuk panel internal, tanpa dependensi auth berat."""

    async def login(self, request: Request) -> bool:
        form = await request.form()
        username, password = form.get("username"), form.get("password")
        if username == settings.admin_username and password == settings.admin_password:
            request.session.update({"admin": True})
            return True
        return False

    async def logout(self, request: Request) -> bool:
        request.session.clear()
        return True

    async def authenticate(self, request: Request) -> bool:
        return bool(request.session.get("admin"))


class ProductAdmin(ModelView, model=Product):
    name = "Barang"
    name_plural = "Barang (Master Data)"
    icon = "fa-solid fa-box"

    column_list = [
        Product.id, Product.name, Product.barcode, Product.category, Product.brand,
        Product.sell_price, Product.cost_price, Product.is_active, Product.is_deleted, Product.updated_at,
    ]
    column_searchable_list = [Product.name, Product.barcode, Product.brand]
    column_sortable_list = [Product.name, Product.sell_price, Product.updated_at]
    column_default_sort = [(Product.updated_at, True)]

    can_create = True
    can_edit = True
    can_delete = True
    can_export = False  # matikan export CSV — tidak perlu, sedikit hemat kerja server saat query besar.

    form_columns = [
        Product.name, Product.barcode, Product.category, Product.brand, Product.unit,
        Product.sell_price, Product.cost_price, Product.default_discount,
        Product.is_active, Product.is_deleted,
    ]


class SyncLogAdmin(ModelView, model=SyncLog):
    name = "Log Sync"
    name_plural = "Riwayat Sinkronisasi"
    icon = "fa-solid fa-arrows-rotate"

    column_list = [SyncLog.id, SyncLog.store_id, SyncLog.direction, SyncLog.record_count, SyncLog.created_at, SyncLog.detail]
    column_sortable_list = [SyncLog.created_at, SyncLog.store_id]
    column_default_sort = [(SyncLog.created_at, True)]

    can_create = False
    can_edit = False
    can_delete = True  # untuk buang log lama manual kalau perlu; tabelnya tidak auto-dibersihkan.
    can_export = False


def setup_admin(app) -> None:
    admin = Admin(
        app,
        engine,
        title="GALAXYAS Sync Admin",
        base_url="/admin",
        authentication_backend=AdminAuth(secret_key=settings.admin_secret_key),
    )
    admin.add_view(ProductAdmin)
    admin.add_view(SyncLogAdmin)
