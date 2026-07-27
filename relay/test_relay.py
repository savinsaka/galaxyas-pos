"""Uji relay dengan agent tiruan: passthrough, 503 saat agent mati, anti-antrian.

    pip install httpx
    python test_relay.py

Tidak butuh PC kasir sungguhan — agent tiruan di file ini yang membalas.
Yang paling penting dijaga di sini: **relay tidak boleh mengantre apa pun**
(uji no. 2, 8, dan 9).
"""

import asyncio
import json
import os
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import httpx  # noqa: E402
import uvicorn  # noqa: E402
import websockets  # noqa: E402

os.environ["RELAY_DB"] = os.path.join(os.path.dirname(os.path.abspath(__file__)), "test_relay.db")
if os.path.exists(os.environ["RELAY_DB"]):
    os.remove(os.environ["RELAY_DB"])

import app as relay  # noqa: E402

PORT = 9117
BASE = f"http://127.0.0.1:{PORT}"

# Daftarkan toko uji.
relay.init_schema()
STORE_ID = "teststore123"
AGENT_KEY = "agentkey456"
with relay.db() as conn:
    conn.execute(
        "INSERT OR REPLACE INTO stores (id, name, agent_key_hash, created_at) VALUES (?,?,?,?)",
        (STORE_ID, "Toko Uji", relay.sha256_hex(AGENT_KEY), "2026-07-27T00:00:00Z"),
    )
    conn.commit()

server = uvicorn.Server(uvicorn.Config(relay.app, host="127.0.0.1", port=PORT, log_level="error"))
threading.Thread(target=server.run, daemon=True).start()
time.sleep(1.5)

results = []


def check(name, cond, detail=""):
    results.append((name, cond, detail))
    print(("PASS  " if cond else "FAIL  ") + name + ("   " + detail if detail else ""))


async def fake_agent(stop_after_n=None, delay=0.0):
    """Agent tiruan: balas setiap rpc dengan echo, seperti lan::dispatch."""
    url = f"ws://127.0.0.1:{PORT}/agent/ws"
    async with websockets.connect(
        url, additional_headers={"X-Store-Id": STORE_ID, "X-Agent-Key": AGENT_KEY}
    ) as ws:
        await ws.send(json.dumps({"type": "tokens", "hashes": [relay.sha256_hex("tokenhp1")]}))
        handled = 0
        while True:
            msg = json.loads(await ws.recv())
            if delay:
                await asyncio.sleep(delay)
            if msg.get("kind") == "pair":
                body = {"device_id": "d1", "device_token": "tokenhp1", "store_name": "Toko Uji"}
                await ws.send(json.dumps({"id": msg["id"], "status": 200, "body": body}))
            else:
                await ws.send(
                    json.dumps(
                        {
                            "id": msg["id"],
                            "status": 200,
                            "body": {"cmd": msg["cmd"], "args": msg["args"], "auth": msg["auth"]},
                        }
                    )
                )
            handled += 1
            if stop_after_n and handled >= stop_after_n:
                return


async def main():
    async with httpx.AsyncClient(base_url=BASE, timeout=40) as http:
        # 1. Relay hidup, belum ada agent.
        r = await http.get("/health")
        check("GET /health ok", r.status_code == 200 and r.json()["stores_online"] == 0, r.text)

        # 2. PC kasir mati -> 503 SEKETIKA, tidak menggantung.
        t0 = time.time()
        r = await http.post(
            f"/s/{STORE_ID}/rpc/list_products",
            headers={"X-Galaxyas-Token": "tokenhp1"},
            json={"limit": 5},
        )
        elapsed = time.time() - t0
        check("agent mati -> 503", r.status_code == 503, r.text)
        check("agent mati -> gagal <1 dtk", elapsed < 1.0, f"{elapsed:.3f}s")

        r = await http.get(f"/s/{STORE_ID}/health")
        check("store health 503 saat agent mati", r.status_code == 503, r.text)

        # 3. Agent tersambung -> passthrough.
        agent = asyncio.create_task(fake_agent())
        await asyncio.sleep(0.8)

        r = await http.get(f"/s/{STORE_ID}/health")
        check("store health 200 saat agent hidup", r.status_code == 200, r.text)

        r = await http.post(
            f"/s/{STORE_ID}/rpc/list_products",
            headers={"X-Galaxyas-Token": "tokenhp1"},
            json={"limit": 5, "search": "aqua"},
        )
        body = r.json()
        check(
            "rpc diteruskan utuh",
            r.status_code == 200
            and body["cmd"] == "list_products"
            and body["args"] == {"limit": 5, "search": "aqua"}
            and body["auth"] == "tokenhp1",
            r.text,
        )

        # 4. Token asing ditolak relay tanpa mengganggu PC kasir.
        r = await http.post(
            f"/s/{STORE_ID}/rpc/list_products",
            headers={"X-Galaxyas-Token": "token-palsu"},
            json={},
        )
        check("token asing -> 401", r.status_code == 401, r.text)

        # 5. Tanpa token -> 401.
        r = await http.post(f"/s/{STORE_ID}/rpc/list_products", json={})
        check("tanpa token -> 401", r.status_code == 401, r.text)

        # 6. Pairing diteruskan.
        r = await http.post(f"/s/{STORE_ID}/pair", json={"code": "A1B2C3", "device_name": "Redmi"})
        check(
            "pair diteruskan",
            r.status_code == 200 and r.json()["device_token"] == "tokenhp1",
            r.text,
        )

        # 7. Toko tak dikenal -> 503 (bukan bocor info keberadaan toko).
        r = await http.post(
            "/s/tokolain/rpc/list_products", headers={"X-Galaxyas-Token": "tokenhp1"}, json={}
        )
        check("store id salah -> 503", r.status_code == 503, r.text)

        # 8. Agent putus di tengah permintaan -> gagal seketika, TIDAK diantre.
        agent.cancel()
        try:
            await agent
        except asyncio.CancelledError:
            pass
        await asyncio.sleep(0.5)

        t0 = time.time()
        r = await http.post(
            f"/s/{STORE_ID}/rpc/checkout", headers={"X-Galaxyas-Token": "tokenhp1"}, json={"sale": {}}
        )
        elapsed = time.time() - t0
        check("setelah agent putus -> 503 cepat", r.status_code == 503 and elapsed < 1.0, r.text)

        # 9. Agent nyambung lagi: TIDAK boleh ada permintaan lama yang tereksekusi.
        seen = []

        async def counting_agent():
            url = f"ws://127.0.0.1:{PORT}/agent/ws"
            async with websockets.connect(
                url, additional_headers={"X-Store-Id": STORE_ID, "X-Agent-Key": AGENT_KEY}
            ) as ws:
                await ws.send(json.dumps({"type": "tokens", "hashes": [relay.sha256_hex("tokenhp1")]}))
                while True:
                    msg = json.loads(await ws.recv())
                    seen.append(msg.get("cmd"))
                    await ws.send(json.dumps({"id": msg["id"], "status": 200, "body": {"ok": True}}))

        agent2 = asyncio.create_task(counting_agent())
        await asyncio.sleep(1.5)
        check("tidak ada permintaan tertahan yang tereksekusi ulang", seen == [], f"seen={seen}")

        # 10. Agent kedua menendang yang pertama (satu toko satu socket).
        agent3 = asyncio.create_task(counting_agent())
        await asyncio.sleep(0.8)
        r = await http.post(
            f"/s/{STORE_ID}/rpc/ping", headers={"X-Galaxyas-Token": "tokenhp1"}, json={}
        )
        check("socket pengganti tetap melayani", r.status_code == 200, r.text)

        # 11. Agent key salah ditolak.
        try:
            async with websockets.connect(
                f"ws://127.0.0.1:{PORT}/agent/ws",
                additional_headers={"X-Store-Id": STORE_ID, "X-Agent-Key": "salah"},
            ):
                check("agent key salah ditolak", False, "koneksi malah diterima")
        except Exception as exc:
            check("agent key salah ditolak", True, type(exc).__name__)

        # 12. Rute admin: tanpa kunci di server = 404 (dianggap tidak ada).
        r = await http.get("/admin/stores")
        check("admin mati saat RELAY_ADMIN_KEY kosong", r.status_code == 404, r.text)
        r = await http.post("/admin/stores", json={"name": "Toko Nakal"})
        check("admin buat toko juga 404 saat kunci kosong", r.status_code == 404, r.text)

        # 13. Dengan kunci diset: salah kunci ditolak, benar kunci jalan.
        relay.ADMIN_KEY = "kunci-admin-uji"
        r = await http.get("/admin/stores", headers={"X-Admin-Key": "salah"})
        check("admin kunci salah -> 401", r.status_code == 401, r.text)
        r = await http.get("/admin/stores")
        check("admin tanpa header -> 401", r.status_code == 401, r.text)

        # Agent uji masih tersambung di titik ini, jadi flag online harus True —
        # itu justru yang ingin dibuktikan: statusnya nyata, bukan tebakan.
        r = await http.get("/admin/stores", headers={"X-Admin-Key": "kunci-admin-uji"})
        rows = r.json()
        check(
            "admin daftar toko + status online sesuai kenyataan",
            r.status_code == 200
            and any(x["id"] == STORE_ID and x["online"] is True for x in rows),
            r.text,
        )
        check(
            "daftar toko tidak membocorkan agent key",
            all("agent_key" not in x and "agent_key_hash" not in x for x in rows),
            r.text,
        )

        r = await http.post(
            "/admin/stores",
            headers={"X-Admin-Key": "kunci-admin-uji"},
            json={"name": "Toko Kedua"},
        )
        baru = r.json()
        check(
            "admin buat toko -> id 32 hex + agent key 64 hex",
            r.status_code == 200 and len(baru["id"]) == 32 and len(baru["agent_key"]) == 64,
            r.text,
        )

        # Toko baru harus benar-benar bisa dipakai agent.
        try:
            async with websockets.connect(
                f"ws://127.0.0.1:{PORT}/agent/ws",
                additional_headers={"X-Store-Id": baru["id"], "X-Agent-Key": baru["agent_key"]},
            ):
                check("toko baru langsung bisa dipakai agent", True)
        except Exception as exc:
            check("toko baru langsung bisa dipakai agent", False, repr(exc))

        r = await http.post(
            "/admin/stores", headers={"X-Admin-Key": "kunci-admin-uji"}, json={"name": "  "}
        )
        check("nama toko kosong ditolak", r.status_code == 400, r.text)

        r = await http.request(
            "DELETE", f"/admin/stores/{baru['id']}", headers={"X-Admin-Key": "kunci-admin-uji"}
        )
        check("admin hapus toko", r.status_code == 200, r.text)
        r = await http.request(
            "DELETE", f"/admin/stores/{baru['id']}", headers={"X-Admin-Key": "kunci-admin-uji"}
        )
        check("hapus toko yang tidak ada -> 404", r.status_code == 404, r.text)

        # Kredensial toko yang dihapus harus mati.
        try:
            async with websockets.connect(
                f"ws://127.0.0.1:{PORT}/agent/ws",
                additional_headers={"X-Store-Id": baru["id"], "X-Agent-Key": baru["agent_key"]},
            ):
                check("kredensial toko terhapus ditolak", False, "malah diterima")
        except Exception:
            check("kredensial toko terhapus ditolak", True)

        relay.ADMIN_KEY = ""

        # 14. Timeout: agent lambat -> 504.
        agent2.cancel()
        agent3.cancel()
        await asyncio.sleep(0.3)
        relay.REQUEST_TIMEOUT_S = 1.0
        slow = asyncio.create_task(fake_agent(delay=3.0))
        await asyncio.sleep(0.8)
        r = await http.post(
            f"/s/{STORE_ID}/rpc/slow", headers={"X-Galaxyas-Token": "tokenhp1"}, json={}
        )
        check("agent lambat -> 504", r.status_code == 504, r.text)
        slow.cancel()

    print()
    failed = [n for n, ok, _ in results if not ok]
    print(f"{len(results) - len(failed)}/{len(results)} lulus")
    if failed:
        print("GAGAL:", failed)
    return 1 if failed else 0


code = asyncio.run(main())
server.should_exit = True
time.sleep(0.5)
sys.exit(code)
