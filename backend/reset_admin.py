"""Reset password akun panel admin dari terminal — untuk kasus lupa password.

Password panel disimpan sebagai hash di tabel `admin_users`, jadi mengubah
ADMIN_PASSWORD di `.env` tidak lagi berpengaruh setelah akun terbuat. Skrip ini
jalan langsung di server (butuh akses ke DATABASE_URL yang sama).

Jalankan dari folder backend:

    python reset_admin.py            # lihat daftar akun
    python reset_admin.py admin      # reset password akun 'admin'

Password diketik lewat prompt (tidak tampil, tidak masuk history shell).
"""

import getpass
import sys

from app.database import Base, SessionLocal, engine
from app.models import AdminUser
from app.security import MIN_PASSWORD_LENGTH, hash_password


def _minta_password(label: str) -> str:
    """Ketikan password tanpa gema. Kalau stdin bukan terminal, baca apa adanya
    supaya skrip ini tetap bisa dipakai dari pipe/automasi."""

    if sys.stdin.isatty():
        return getpass.getpass(label)
    print(label, end="", flush=True)
    return sys.stdin.readline().rstrip("\r\n")


def daftar_akun() -> None:
    with SessionLocal() as db:
        users = db.query(AdminUser).order_by(AdminUser.username).all()
        if not users:
            print("Belum ada akun admin. Jalankan server sekali agar akun dari .env dibuat.")
            return
        print("Akun admin:")
        for user in users:
            print(f"  - {user.username}")
    print("\nReset password: python reset_admin.py <username>")


def reset(username: str) -> int:
    with SessionLocal() as db:
        user = db.query(AdminUser).filter(AdminUser.username == username).first()
        if user is None:
            print(f"Akun '{username}' tidak ada.")
            return 1

        baru = _minta_password("Password baru: ")
        if len(baru) < MIN_PASSWORD_LENGTH:
            print(f"Password minimal {MIN_PASSWORD_LENGTH} karakter.")
            return 1
        if baru != _minta_password("Ulangi password baru: "):
            print("Konfirmasi tidak sama.")
            return 1

        user.password_hash = hash_password(baru)
        db.commit()

    print(f"Password akun '{username}' sudah diganti.")
    return 0


def main() -> int:
    Base.metadata.create_all(bind=engine)
    if len(sys.argv) < 2:
        daftar_akun()
        return 0
    return reset(sys.argv[1])


if __name__ == "__main__":
    raise SystemExit(main())
