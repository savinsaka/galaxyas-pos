from pydantic_settings import BaseSettings, SettingsConfigDict


class Settings(BaseSettings):
    """Konfigurasi aplikasi, dibaca dari environment / file .env."""

    model_config = SettingsConfigDict(env_file=".env", env_file_encoding="utf-8", extra="ignore")

    database_url: str = "postgresql+psycopg2://postgres:postgres@localhost:5432/galaxyas"
    cors_origins: str = "http://localhost:1420,http://localhost:5173,tauri://localhost"

    # Panel admin (/admin) untuk lihat & ubah data langsung — ganti nilai ini di .env untuk produksi.
    admin_username: str = "admin"
    admin_password: str = "ganti-password-ini"
    admin_secret_key: str = "ganti-secret-key-ini-untuk-produksi"

    @property
    def cors_origin_list(self) -> list[str]:
        return [o.strip() for o in self.cors_origins.split(",") if o.strip()]


settings = Settings()
