"""Panel admin (/admin) untuk lihat & ubah data server langsung dari browser.

Sengaja pakai SQLAdmin (nempel di proses FastAPI yang sama, render halaman
per-request lewat Jinja2) — TIDAK ada proses/worker/queue tambahan, TIDAK ada
polling, TIDAK ada build frontend terpisah. Server tetap seringan sebelumnya;
panel ini cuma menjawab request yang datang, sama seperti endpoint sync.
"""

from pathlib import Path

from anyio.to_thread import run_sync
from starlette.requests import Request
from starlette.responses import RedirectResponse, Response
from sqladmin import Admin, BaseView, ModelView, expose
from sqladmin.authentication import AuthenticationBackend

from app.config import settings
from app.database import SessionLocal, engine
from app.models import AdminUser, Product, SyncLog, utcnow
from app.security import MIN_PASSWORD_LENGTH, hash_password, verify_password

_TEMPLATES_DIR = str(Path(__file__).resolve().parent / "templates")

# Kunci session yang menyimpan username admin yang sedang login.
_SESSION_KEY = "admin_user"


def ensure_admin_user() -> None:
    """Buat akun admin pertama dari `.env` kalau tabelnya masih kosong.

    Sesudah akun ada, `.env` TIDAK dipakai lagi untuk login — password diganti
    lewat halaman Profil. Kalau lupa password, pakai `reset_admin.py`.
    """

    with SessionLocal() as db:
        if db.query(AdminUser).first() is not None:
            return
        db.add(
            AdminUser(
                username=settings.admin_username,
                full_name="Administrator",
                password_hash=hash_password(settings.admin_password),
            )
        )
        db.commit()


def _periksa_login(username: str, password: str) -> str | None:
    """Kembalikan username kalau cocok, None kalau tidak. Sekalian catat waktu login."""

    with SessionLocal() as db:
        user = db.query(AdminUser).filter(AdminUser.username == username).first()
        if user is None or not verify_password(password, user.password_hash):
            return None
        user.last_login_at = utcnow()
        db.commit()
        return user.username


def _ambil_profil(username: str) -> dict | None:
    with SessionLocal() as db:
        user = db.query(AdminUser).filter(AdminUser.username == username).first()
        if user is None:
            return None
        return {
            "username": user.username,
            "full_name": user.full_name or "",
            "last_login_at": user.last_login_at,
            "created_at": user.created_at,
        }


def _simpan_identitas(username: str, username_baru: str, full_name: str) -> tuple[str, str | None, str | None]:
    """Simpan username & nama. Kembalikan (username sekarang, pesan sukses, pesan galat)."""

    if len(username_baru) < 3:
        return username, None, "Username minimal 3 karakter."

    with SessionLocal() as db:
        user = db.query(AdminUser).filter(AdminUser.username == username).first()
        if user is None:
            return username, None, "Akun tidak ditemukan."
        if username_baru != username:
            bentrok = db.query(AdminUser).filter(AdminUser.username == username_baru).first()
            if bentrok is not None:
                return username, None, f"Username '{username_baru}' sudah dipakai."
        user.username = username_baru
        user.full_name = full_name or None
        db.commit()
        return username_baru, "Profil tersimpan.", None


def _ganti_password(username: str, lama: str, baru: str, ulang: str) -> tuple[str | None, str | None]:
    """Ganti password. Kembalikan (pesan sukses, pesan galat)."""

    if len(baru) < MIN_PASSWORD_LENGTH:
        return None, f"Password baru minimal {MIN_PASSWORD_LENGTH} karakter."
    if baru != ulang:
        return None, "Konfirmasi password baru tidak sama."

    with SessionLocal() as db:
        user = db.query(AdminUser).filter(AdminUser.username == username).first()
        if user is None:
            return None, "Akun tidak ditemukan."
        if not verify_password(lama, user.password_hash):
            return None, "Password lama salah."
        user.password_hash = hash_password(baru)
        db.commit()
        return "Password berhasil diganti.", None


class AdminAuth(AuthenticationBackend):
    """Login panel: username/password dicocokkan ke tabel `admin_users`.

    Query DB & hashing password dijalankan di thread terpisah (`run_sync`)
    karena keduanya blocking — PBKDF2 sengaja lambat — dan handler SQLAdmin
    ini async, jadi tidak boleh menahan event loop.
    """

    async def login(self, request: Request) -> bool:
        form = await request.form()
        username = str(form.get("username") or "").strip()
        password = str(form.get("password") or "")
        if not username or not password:
            return False
        cocok = await run_sync(_periksa_login, username, password)
        if cocok is None:
            return False
        request.session.update({_SESSION_KEY: cocok})
        return True

    async def logout(self, request: Request) -> bool:
        request.session.clear()
        return True

    async def authenticate(self, request: Request) -> bool:
        username = request.session.get(_SESSION_KEY)
        if not username:
            return False
        # Akun bisa saja sudah dihapus/di-rename sejak cookie dibuat.
        return await run_sync(_ambil_profil, username) is not None


class ProfilView(BaseView):
    """Halaman Profil: ubah identitas & ganti password sendiri, tanpa SSH ke VPS."""

    name = "Profil"
    identity = "profil"
    icon = "fa-solid fa-user"

    @expose("/profil", methods=["GET", "POST"], identity="profil")
    async def profil(self, request: Request) -> Response:
        username = str(request.session.get(_SESSION_KEY) or "")
        pesan: str | None = None
        galat: str | None = None

        if request.method == "POST":
            form = await request.form()
            if form.get("aksi") == "password":
                pesan, galat = await run_sync(
                    _ganti_password,
                    username,
                    str(form.get("password_lama") or ""),
                    str(form.get("password_baru") or ""),
                    str(form.get("password_ulang") or ""),
                )
            else:
                username, pesan, galat = await run_sync(
                    _simpan_identitas,
                    username,
                    str(form.get("username") or "").strip(),
                    str(form.get("full_name") or "").strip(),
                )
                request.session[_SESSION_KEY] = username

        user = await run_sync(_ambil_profil, username)
        if user is None:
            request.session.clear()
            return RedirectResponse(request.url_for("admin:login"), status_code=302)

        return await self.templates.TemplateResponse(
            request,
            "profil.html",
            {
                "title": "Profil",
                "user": user,
                "pesan": pesan,
                "galat": galat,
                "min_password": MIN_PASSWORD_LENGTH,
            },
        )


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
        # Path absolut: service di VPS belum tentu jalan dari folder backend.
        templates_dir=_TEMPLATES_DIR,
        authentication_backend=AdminAuth(secret_key=settings.admin_secret_key),
    )
    admin.add_view(ProductAdmin)
    admin.add_view(SyncLogAdmin)
    admin.add_view(ProfilView)
