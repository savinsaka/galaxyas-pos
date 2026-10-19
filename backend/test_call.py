"""Uji sinyal Voice Call: dering ke semua PC toko tujuan, angkat di satu PC,
SDP/ICE hanya diteruskan ke lawan bicara, tolak/tutup/tak dijawab/offline/sibuk,
socket putus, catatan panggilan di riwayat chat, kredensial TURN.

    pip install -r requirements.txt httpx
    python test_call.py

Memakai SQLite sementara — tidak menyentuh PostgreSQL produksi.
"""

import base64
import hashlib
import hmac
import os
import sys
import tempfile
import time

DB = os.path.join(tempfile.gettempdir(), "gpos_test_call.db")
if os.path.exists(DB):
    os.remove(DB)
os.environ["DATABASE_URL"] = f"sqlite:///{DB}"
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from fastapi.testclient import TestClient  # noqa: E402

from app import admin  # noqa: E402
from app.config import settings  # noqa: E402
from app.main import app  # noqa: E402
from app.routers import chat  # noqa: E402

results = []


def check(name, cond, detail=""):
    results.append((name, cond))
    print(("PASS  " if cond else "FAIL  ") + name + ("   " + str(detail) if detail else ""))


def hdr(code, key):
    return {"X-Chat-Store": code, "X-Chat-Key": key}


def recv_type(ws, want):
    """Ambil pesan berikutnya bertipe `want` (lewati yang lain, mis. message log)."""
    for _ in range(10):
        m = ws.receive_json()
        if m["type"] == want:
            return m
    raise AssertionError(f"tidak menerima {want}")


settings.turn_secret = "rahasia-uji"
settings.turn_urls = "turn:jjapps.net:3478?transport=udp, turn:jjapps.net:3478?transport=tcp"
chat.RING_TIMEOUT_S = 0.5

with TestClient(app) as client:
    for code, name in (("toko-001", "Toko Pusat"), ("toko-002", "Toko Depan"), ("toko-003", "Toko Belakang")):
        admin._simpan_toko_chat(code, name)
    k1 = admin._kunci_baru("toko-001")
    k2 = admin._kunci_baru("toko-002")
    k3 = admin._kunci_baru("toko-003")

    with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-001", k1)) as a, \
         client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-002", k2)) as b1, \
         client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-002", k2)) as b2:
        for ws in (a, b1, b2):
            ws.receive_json()  # hello

        # 1. Toko tujuan offline.
        a.send_json({"type": "call_invite", "call_id": "call-offline-1", "to": "toko-003"})
        m = a.receive_json()
        check("tujuan offline langsung berakhir", m == {"type": "call_end", "call_id": "call-offline-1", "reason": "offline"})

        # 2. Kode tidak terdaftar / diri sendiri.
        a.send_json({"type": "call_invite", "call_id": "call-unknown-1", "to": "toko-404"})
        check("kode tak terdaftar ditolak", a.receive_json()["reason"] == "unknown")
        a.send_json({"type": "call_invite", "call_id": "call-self-001", "to": "toko-001"})
        check("menelepon diri sendiri ditolak", a.receive_json()["reason"] == "unknown")

        # 3. Berdering di SEMUA PC toko tujuan + kredensial TURN.
        a.send_json({"type": "call_invite", "call_id": "call-answer-1", "to": "toko-002"})
        r1 = b1.receive_json()
        r2 = b2.receive_json()
        ringing = a.receive_json()
        check("dering di PC 1 toko tujuan", r1["type"] == "call_ring" and r1["from"] == "toko-001" and r1["from_name"] == "Toko Pusat")
        check("dering di PC 2 toko tujuan", r2["type"] == "call_ring")
        check("penelepon dapat call_ringing", ringing["type"] == "call_ringing")
        turn = ringing["ice"][0]
        exp, who = turn["username"].split(":")
        expected = base64.b64encode(hmac.new(b"rahasia-uji", turn["username"].encode(), hashlib.sha1).digest()).decode()
        check("kredensial TURN valid", turn["credential"] == expected and who == "toko-001" and int(exp) > time.time() + 3600, turn)
        check("URL TURN dari setting", turn["urls"] == ["turn:jjapps.net:3478?transport=udp", "turn:jjapps.net:3478?transport=tcp"])
        check("STUN cadangan ada", any("stun:" in u for s in ringing["ice"] for u in s["urls"]))

        # 4. Toko yang sedang berdering dianggap sibuk.
        with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-003", k3)) as c:
            c.receive_json()
            c.send_json({"type": "call_invite", "call_id": "call-busy-01", "to": "toko-002"})
            check("tujuan sibuk ditolak", c.receive_json()["reason"] == "busy")

        # 5. Angkat di PC 2 → PC 1 berhenti berdering.
        b2.send_json({"type": "call_accept", "call_id": "call-answer-1"})
        check("penelepon dapat call_accepted", a.receive_json() == {"type": "call_accepted", "call_id": "call-answer-1"})
        check("PC lain berhenti berdering", b1.receive_json() == {"type": "call_end", "call_id": "call-answer-1", "reason": "answered_elsewhere"})

        b1.send_json({"type": "call_accept", "call_id": "call-answer-1"})  # telat: diabaikan

        # 6. SDP/ICE hanya ke lawan bicara.
        a.send_json({"type": "call_signal", "call_id": "call-answer-1", "data": {"sdp": {"type": "offer", "sdp": "v=0"}}})
        got = b2.receive_json()
        check("offer sampai ke PC yang mengangkat", got["type"] == "call_signal" and got["data"]["sdp"]["type"] == "offer")
        b2.send_json({"type": "call_signal", "call_id": "call-answer-1", "data": {"candidate": {"candidate": "c"}}})
        check("candidate balik ke penelepon", a.receive_json()["data"]["candidate"]["candidate"] == "c")
        b1.send_json({"type": "call_signal", "call_id": "call-answer-1", "data": {"sdp": {"type": "answer"}}})  # bukan peserta

        # 7. Tutup → lawan dapat call_end + riwayat "Panggilan suara".
        time.sleep(1.1)
        a.send_json({"type": "call_hangup", "call_id": "call-answer-1"})
        end = b2.receive_json()
        check("lawan dapat call_end hangup", end == {"type": "call_end", "call_id": "call-answer-1", "reason": "hangup"})
        log_b = recv_type(b2, "message")["message"]
        check("riwayat panggilan berdurasi", log_b["kind"] == "call" and int(log_b["body"]) >= 1, log_b)
        check("panggilan terjawab tidak jadi belum-dibaca", log_b["read_at"] is not None)
        recv_type(b1, "message")
        recv_type(a, "message")

        # 8. Ditolak.
        a.send_json({"type": "call_invite", "call_id": "call-reject-1", "to": "toko-002"})
        b1.receive_json(); b2.receive_json(); a.receive_json()
        b1.send_json({"type": "call_reject", "call_id": "call-reject-1"})
        check("penelepon dapat rejected", a.receive_json()["reason"] == "rejected")
        check("PC lain berhenti berdering saat ditolak", b2.receive_json()["reason"] == "rejected")
        check("riwayat ditolak", recv_type(a, "message")["message"]["body"] == "rejected")
        recv_type(b1, "message"); recv_type(b2, "message")

        # 9. Tidak dijawab (timeout dering).
        a.send_json({"type": "call_invite", "call_id": "call-noans-1", "to": "toko-002"})
        b1.receive_json(); b2.receive_json(); a.receive_json()
        check("timeout → no_answer ke penelepon", a.receive_json()["reason"] == "no_answer")
        check("timeout → dering berhenti", b1.receive_json()["reason"] == "no_answer")
        b2.receive_json()
        missed = recv_type(b1, "message")["message"]
        check("riwayat tak terjawab & belum dibaca", missed["body"] == "missed" and missed["read_at"] is None)
        recv_type(b2, "message"); recv_type(a, "message")

        # 10. Penelepon membatalkan saat berdering.
        a.send_json({"type": "call_invite", "call_id": "call-cancel-1", "to": "toko-002"})
        b1.receive_json(); b2.receive_json(); a.receive_json()
        a.send_json({"type": "call_hangup", "call_id": "call-cancel-1"})
        check("batal → dering berhenti", b1.receive_json()["reason"] == "cancelled")
        b2.receive_json()
        check("batal → tak terjawab", recv_type(b1, "message")["message"]["body"] == "missed")
        recv_type(b2, "message"); recv_type(a, "message")

        # 11. Socket penelepon putus saat bicara.
        with client.websocket_connect("/api/v1/chat/ws", headers=hdr("toko-003", k3)) as c:
            c.receive_json()
            c.send_json({"type": "call_invite", "call_id": "call-drop-01", "to": "toko-002"})
            b1.receive_json(); b2.receive_json(); c.receive_json()
            b1.send_json({"type": "call_accept", "call_id": "call-drop-01"})
            c.receive_json(); b2.receive_json()
        check("socket putus → lawan dapat disconnected", b1.receive_json()["reason"] == "disconnected")
        check("panggilan dibersihkan", not chat.CALLS)

failed = [n for n, ok in results if not ok]
print(f"\n{len(results) - len(failed)}/{len(results)} lulus")
sys.exit(1 if failed else 0)
