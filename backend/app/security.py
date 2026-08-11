"""Hash & verifikasi password akun panel admin.

Sengaja hanya memakai stdlib (PBKDF2-HMAC-SHA256 dari `hashlib`) — tidak
menambah bcrypt/passlib. Yang disimpan di sini cuma password panel internal,
dan menambah dependensi berarti menambah kerjaan saat pasang ulang di VPS.

Format simpanan: `pbkdf2_sha256$<iterasi>$<salt hex>$<hash hex>`. Iterasi ikut
disimpan supaya hash lama tetap bisa diverifikasi kalau nilainya dinaikkan.
"""

import hashlib
import hmac
import secrets

_ALGO = "pbkdf2_sha256"
_ITERATIONS = 600_000
_SALT_BYTES = 16

# Panjang minimal password baru — dipakai di halaman Profil & skrip reset.
MIN_PASSWORD_LENGTH = 8


def _derive(password: str, salt: str, iterations: int) -> str:
    return hashlib.pbkdf2_hmac(
        "sha256", password.encode("utf-8"), salt.encode("utf-8"), iterations
    ).hex()


def hash_password(password: str) -> str:
    salt = secrets.token_hex(_SALT_BYTES)
    return f"{_ALGO}${_ITERATIONS}${salt}${_derive(password, salt, _ITERATIONS)}"


def verify_password(password: str, stored: str | None) -> bool:
    """True kalau `password` cocok dengan hash `stored`. Hash rusak/kosong = False."""

    if not stored:
        return False
    try:
        algo, iterations, salt, digest = stored.split("$")
        rounds = int(iterations)
    except ValueError:
        return False
    if algo != _ALGO:
        return False
    return hmac.compare_digest(_derive(password, salt, rounds), digest)
