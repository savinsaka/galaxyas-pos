"""Uji Chat antar toko: auth, teks & push, file (diteruskan, tidak disimpan),
ekstensi terlarang, toko offline, riwayat & retensi 7 hari, ganti kunci.

    pip install -r requirements.txt httpx
    python test_chat.py

Memakai SQLite sementara — tidak menyentuh PostgreSQL produksi.
"""

import os
import sys
import tempfile
from datetime import timedelta

DB = os.path.join(tempfile.gettempdir(), "gpos_test_chat.db")
if os.path.exists(DB):
    os.remove(DB)
os.environ["DATABASE_URL"] = f"sqlite:///{DB}"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from fastapi.testclient import TestClient  # noqa: E402
from starlette.websockets import WebSocketDisconnect  # noqa: E402

from app import admin  # noqa: E402
from app.database import SessionLocal, engine  # noqa: E402
from app.main import app  # noqa: E402
from app.models import ChatMessage, utcnow  # noqa: E402
from app.routers import chat  # noqa: E402

results = []


def check(name, cond, detail=""):
    results.append((name, cond))
    print(("PASS  " if cond else "FAIL  ") + name + ("   " + str(detail) if detail else ""))


def hdr(code, key):
    return {"X-Chat-Store": code, "X-Chat-Key": key}


with TestClient(app) as client:
    admin._simpan_toko_chat("toko-001", "Toko Pusat")
    admin._simpan_toko_chat("toko-002", "Toko Depan")
    admin._simpan_toko_chat("toko-003", "Toko Belakang")
    k1 = admin._kunci_baru("toko-001")
    k2 = admin._kunci_baru("toko-002")
    k3 = admin._kunci_baru("toko-003")

    # 1. Kunci salah ditolak.
    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", "SALAH")) as ws:
        m = ws.receive_json()
        check("kunci salah ditolak", m["type"] == "error" and m.get("fatal"))
    r = client.get("/api/v1/chat/messages", headers=hdr("toko-001", "SALAH"))
    check("REST tanpa kunci benar 401", r.status_code == 401)

    # 2. Kunci boleh diketik tanpa strip / huruf kecil.
    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", k1.replace("-", "").lower())) as ws:
        check("kunci dinormalisasi", ws.receive_json()["type"] == "hello")

    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", k1)) as a, \
         client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", k1)) as a2, \
         client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-002", k2)) as b:
        hello = a.receive_json()
        a2.receive_json()
        b.receive_json()
        check("hello berisi nama toko", hello["name"] == "Toko Pusat")

        # 3. Teks biasa sampai ke toko tujuan + PC lain toko pengirim.
        a.send_json({"type": "send", "to": "toko-002", "body": "halo", "client_id": "c1"})
        got_b = b.receive_json()
        got_a = a.receive_json()
        got_a2 = a2.receive_json()
        check("teks sampai ke toko tujuan", got_b["message"]["body"] == "halo" and not got_b["message"]["push"])
        check("pengirim dapat konfirmasi + client_id", got_a["message"]["client_id"] == "c1")
        check("PC lain toko pengirim ikut menerima", got_a2["message"]["body"] == "halo")

        # 4. Push alert.
        a.send_json({"type": "send", "to": "toko-002", "body": "stok habis!", "push": True})
        check("alert ditandai push", b.receive_json()["message"]["push"] is True)
        a.receive_json(); a2.receive_json()

        # 5. Kode tujuan tidak terdaftar.
        a.send_json({"type": "send", "to": "toko-999", "body": "x", "client_id": "c2"})
        m = a.receive_json()
        check("tujuan tak terdaftar ditolak", m["type"] == "error" and m["client_id"] == "c2")

        # 6. File: diteruskan dengan isi ke tujuan, tanpa isi ke PC pengirim.
        a.send_json({"type": "file", "to": "toko-002", "name": "laporan.pdf", "client_id": "f1"})
        a.send_bytes(b"%PDF-1.4 isi")
        head = b.receive_json()
        data = b.receive_bytes()
        check("file sampai ke tujuan", head["message"]["file_name"] == "laporan.pdf" and data == b"%PDF-1.4 isi")
        check("pengirim dapat konfirmasi file", a.receive_json()["message"]["kind"] == "file")
        a2.receive_json()

        # 7. Ekstensi terlarang.
        for bad in ("virus.exe", "gambar.svg", "makro.xlsm"):
            a.send_json({"type": "file", "to": "toko-002", "name": bad, "client_id": bad})
            a.send_bytes(b"xx")
            m = a.receive_json()
            check(f"ekstensi {bad} ditolak", m["type"] == "error" and "tidak diizinkan" in m["message"])

        # 8. File ke toko yang tidak ada PC online.
        a.send_json({"type": "file", "to": "toko-003", "name": "foto.jpg", "client_id": "f3"})
        a.send_bytes(b"\xff\xd8")
        m = a.receive_json()
        check("file ke toko offline ditolak", m["type"] == "error" and "offline" in m["message"])

        # 9. Teks ke toko offline tetap tersimpan (dibaca saat buka menu).
        a.send_json({"type": "send", "to": "toko-003", "body": "nanti dibaca"})
        a.receive_json(); a2.receive_json()

        # 10. Lookup kontak.
        r = client.get("/api/v1/chat/lookup", params={"code": "toko-002"}, headers=hdr("toko-001", k1))
        check("lookup kode toko", r.json() == {"code": "toko-002", "name": "Toko Depan", "online": True})
        r = client.get("/api/v1/chat/lookup", params={"code": "toko-404"}, headers=hdr("toko-001", k1))
        check("lookup kode asing 404", r.status_code == 404)

    # 11. Riwayat: file tercatat tanpa isi; toko-003 melihat pesannya.
    r = client.get("/api/v1/chat/messages", headers=hdr("toko-003", k3)).json()
    check("riwayat toko offline berisi pesan", [m["body"] for m in r["messages"]] == ["nanti dibaca"])
    with SessionLocal() as db:
        files = db.query(ChatMessage).filter(ChatMessage.kind == "file").all()
        check("file hanya tercatat nama & ukuran", len(files) == 1 and files[0].file_size == 12)

    # 12. Retensi 7 hari.
    with SessionLocal() as db:
        old = db.query(ChatMessage).first()
        old.created_at = utcnow() - timedelta(days=8)
        db.commit()
    r = client.get("/api/v1/chat/messages", headers=hdr("toko-001", k1)).json()
    check("pesan > 7 hari tidak tampil", all(m["body"] != "halo" for m in r["messages"]))
    chat._last_cleanup = 0
    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", k1)) as a:
        a.receive_json()
        a.send_json({"type": "send", "to": "toko-003", "body": "pemicu bersih"})
        a.receive_json()
    with SessionLocal() as db:
        check("pesan > 7 hari dihapus", db.query(ChatMessage).filter(ChatMessage.body == "halo").count() == 0)

    # 13. Ganti kunci: kunci lama tidak berlaku lagi.
    k1b = admin._kunci_baru("toko-001")
    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", k1)) as ws:
        check("kunci lama ditolak setelah diganti", ws.receive_json()["type"] == "error")
    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", k1b)) as ws:
        check("kunci baru berlaku", ws.receive_json()["type"] == "hello")

    # 14. Rate limit kirim.
    chat._send_hits.clear()
    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-002", k2)) as b:
        b.receive_json()
        last = None
        for i in range(35):
            b.send_json({"type": "send", "to": "toko-003", "body": f"spam {i}"})
            last = b.receive_json()
        check("rate limit 30 pesan/menit", last["type"] == "error" and "Terlalu banyak" in last["message"])

    # 15. Centang ala WA: ✓ terkirim → ✓✓ diterima → ✓✓ biru dibaca.
    chat._send_hits.clear()
    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", k1b)) as a:
        a.receive_json()
        a.send_json({"type": "send", "to": "toko-003", "body": "cek centang"})
        sent = a.receive_json()["message"]
        check("✓ terkirim: tujuan offline belum diterima", sent["delivered_at"] is None and sent["read_at"] is None)

        with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-003", k3)) as c:
            c.receive_json()
            r = a.receive_json()
            check("✓✓ diterima saat tujuan tersambung", r["type"] == "receipt" and sent["id"] in r["ids"]
                  and r["delivered_at"] and not r["read_at"])

            c.send_json({"type": "read", "peer": "toko-001", "up_to": sent["id"]})
            r = a.receive_json()
            check("✓✓ biru dibaca dikabarkan ke pengirim", sent["id"] in r["ids"] and r["read_at"])
            r2 = c.receive_json()
            check("PC toko pembaca ikut dikabari (angka belum dibaca hilang)", sent["id"] in r2["ids"])

            # Tujuan online saat dikirim → langsung ✓✓.
            a.send_json({"type": "send", "to": "toko-003", "body": "langsung diterima"})
            c.receive_json()
            m = a.receive_json()["message"]
            check("tujuan online → langsung diterima", m["delivered_at"] is not None and m["read_at"] is None)

    hist = client.get("/api/v1/chat/messages", headers=hdr("toko-001", k1b)).json()["messages"]
    cek = next(m for m in hist if m["body"] == "cek centang")
    check("riwayat memuat status dibaca", cek["read_at"] is not None)

    # 16. Voice note: diteruskan seperti file, kind "voice", body = durasi.
    chat._send_hits.clear()
    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", k1b)) as a, \
         client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-002", k2)) as b:
        a.receive_json(); b.receive_json()
        a.send_json({"type": "file", "voice": True, "to": "toko-002", "name": "vn.webm", "duration": 12.4})
        a.send_bytes(b"OPUS" * 100)
        head = b.receive_json()["message"]
        data = b.receive_bytes()
        check("voice note sampai", head["kind"] == "voice" and head["body"] == "12" and len(data) == 400)
        a.receive_json()

        a.send_json({"type": "file", "to": "toko-002", "name": "diam-diam.webm", "client_id": "w1"})
        a.send_bytes(b"x")
        m = a.receive_json()
        check("webm sebagai file biasa ditolak", m["type"] == "error" and m["client_id"] == "w1")

        a.send_json({"type": "file", "voice": True, "to": "toko-002", "name": "vn.exe", "client_id": "w2"})
        a.send_bytes(b"x")
        check("voice note bukan webm/ogg ditolak", a.receive_json()["type"] == "error")

        a.send_json({"type": "file", "voice": True, "to": "toko-002", "name": "vn.webm", "client_id": "w3"})
        a.send_bytes(b"x" * (chat.MAX_VOICE + 1))
        m = a.receive_json()
        check("voice note terlalu besar ditolak", m["type"] == "error" and "terlalu panjang" in m["message"])

    # 17. Migrasi kolom aman dijalankan berulang.
    chat.ensure_chat_columns()
    check("migrasi kolom idempoten", True)

    # 18. Halaman admin butuh login.
    r = client.get("/admin/toko-chat", follow_redirects=False)
    check("halaman Toko Chat butuh login", r.status_code in (302, 303, 307))

    # 19. Bug nyata 2026-09-25: refresh halaman sesudah "Ganti Kunci" mengirim
    # ulang formulir → kunci diganti lagi → PC toko terus terputus.
    from app.config import settings
    from app.models import ChatStore

    client.post("/admin/login", data={"username": settings.admin_username, "password": settings.admin_password})
    r = client.post("/admin/toko-chat", data={"aksi": "kunci", "code": "toko-002"}, follow_redirects=False)
    check("aksi admin dibalas redirect 303 (bukan halaman)", r.status_code == 303)
    page = client.get(r.headers["location"]).text
    import re as _re
    shown = _re.search(r"[A-Z2-9]{4}-[A-Z2-9]{4}-[A-Z2-9]{4}-[A-Z2-9]{4}", page)
    check("kunci baru tampil sekali setelah redirect", shown is not None)
    with SessionLocal() as db:
        hash_before = db.get(ChatStore, "toko-002").key_hash
    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-002", shown.group(0))) as b:
        check("PC tersambung dengan kunci baru", b.receive_json()["type"] == "hello")
        page2 = client.get("/admin/toko-chat").text  # = refresh browser
        with SessionLocal() as db:
            hash_after = db.get(ChatStore, "toko-002").key_hash
        check("refresh halaman TIDAK mengganti kunci", hash_before == hash_after)
        check("kunci tidak tampil lagi saat refresh", shown.group(0) not in page2)
        check("PC tetap online sesudah refresh", chat.online("toko-002"))

failed = [n for n, ok in results if not ok]
print(f"\n{len(results) - len(failed)}/{len(results)} lulus")
engine.dispose()
if os.path.exists(DB):
    os.remove(DB)
sys.exit(1 if failed else 0)
