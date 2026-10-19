from datetime import datetime, timezone

from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Konfigurasi aplikasi, dibaca dari environment / file .env."""

    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    database_url: str = "postgresql+psycopg2://postgres:postgres@localhost:5432/galaxyas"
    cors_origins: str = "http://localhost:1420,http://localhost:5173,tauri://localhost"

    # Panel admin (/admin). Username & password di bawah hanya dipakai untuk
    # membuat akun pertama (lihat app.admin.ensure_admin_user); selanjutnya
    # password dikelola dari halaman Profil dan disimpan sebagai hash di DB.
    admin_username: str = "admin"
    admin_password: str = "ganti-password-ini"
    admin_secret_key: str = "ganti-secret-key-ini-untuk-produksi"

    # Sesudah waktu ini, request Sync/SSoT tanpa login ditolak (app lama yang
    # belum update). Sebelumnya masih diterima supaya ada waktu untuk update.
    # Default 19 Okt 2026 23:00 WIB.
    sync_auth_deadline: datetime = datetime(2026, 10, 19, 16, 0, tzinfo=timezone.utc)

    # Gerbang "Wajib Update" (app v1.7.0+ menanyakan ini saat dibuka). Kosong =
    # tidak ada kewajiban. Versi app < min_version: pita merah sebelum
    # `app_update_deadline`, layar terkunci sesudahnya.
    pos_min_version: str = ""
    warehouse_min_version: str = ""
    app_update_deadline: datetime | None = None
    app_update_message: str = ""

    @property
    def cors_origin_list(self) -> list[str]:
        return [o.strip() for o in self.cors_origins.split(",") if o.strip()]


settings = Settings()
