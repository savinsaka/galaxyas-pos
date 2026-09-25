"""Uji Remote GPOS di relay: pairing ID+OTP, OTP salah/sekali pakai, klaim ID,
penerusan dua arah (teks + biner), dan putus di satu sisi menutup sisi lain.

    python test_remote.py
"""

import asyncio
import hashlib
import json
import os
import sys
import threading
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import uvicorn  # noqa: E402
import websockets  # noqa: E402

os.environ["RELAY_DB"] = os.path.join(os.path.dirname(os.path.abspath(__file__)), "test_remote.db")
if os.path.exists(os.environ["RELAY_DB"]):
    os.remove(os.environ["RELAY_DB"])

import app as relay  # noqa: E402

PORT = 9118
WS = f"ws://127.0.0.1:{PORT}"

relay.init_schema()
server = uvicorn.Server(uvicorn.Config(relay.app, host="127.0.0.1", port=PORT, log_level="error"))
threading.Thread(target=server.run, daemon=True).start()
time.sleep(1.5)

results = []


def check(name, cond, detail=""):
    results.append((name, cond))
    print(("PASS  " if cond else "FAIL  ") + name + ("   " + detail if detail else ""))


def h(s):
    return hashlib.sha256(s.encode()).hexdigest()


RID = "123456789"
SECRET = "s" * 40


async def host(otp, secret=SECRET, rid=RID):
    return await websockets.connect(
        f"{WS}/remote/host",
        additional_headers={"X-Remote-Id": rid, "X-Remote-Secret": secret, "X-Remote-Otp-Hash": h(otp)},
    )


async def viewer(otp, rid=RID):
    return await websockets.connect(
        f"{WS}/remote/view", additional_headers={"X-Remote-Id": rid, "X-Remote-Otp": otp}
    )


async def first(ws):
    return json.loads(await asyncio.wait_for(ws.recv(), 3))


async def closed(ws):
    try:
        while True:
            await asyncio.wait_for(ws.recv(), 3)
    except websockets.ConnectionClosed:
        return True
    except asyncio.TimeoutError:
        return False


async def main():
    # 1. Tidak ada host -> viewer ditolak dengan pesan.
    v = await viewer("111111")
    m = await first(v)
    check("viewer tanpa host ditolak", m["type"] == "error" and "tidak sedang" in m["message"], m.get("message", ""))

    # 2. Host siap.
    hs = await host("222222")
    check("host menerima ready", (await first(hs))["type"] == "ready")

    # 3. Klaim ID: rahasia lain untuk ID yang sama ditolak.
    other = await host("333333", secret="x" * 40)
    m = await first(other)
    check("ID dikunci ke rahasia PC pertama", m["type"] == "error" and "dipakai PC lain" in m["message"])

    # 4. OTP salah.
    v = await viewer("000000")
    m = await first(v)
    check("OTP salah ditolak", m["type"] == "error" and "salah" in m["message"])

    # 5. OTP benar -> pasangan terbentuk.
    v = await viewer("222222")
    check("viewer menerima joined", (await first(v))["type"] == "joined")
    check("host diberi tahu viewer_joined", (await first(hs))["type"] == "viewer_joined")

    # 6. Penerusan dua arah.
    await hs.send(b"\xff\xd8JPEGDATA")
    got = await asyncio.wait_for(v.recv(), 3)
    check("biner host -> viewer", got == b"\xff\xd8JPEGDATA")
    await v.send(json.dumps({"type": "mouse", "x": 10}))
    got = json.loads(await asyncio.wait_for(hs.recv(), 3))
    check("teks viewer -> host", got == {"type": "mouse", "x": 10})

    # 7. OTP sekali pakai: viewer kedua dengan OTP sama ditolak.
    v2 = await viewer("222222")
    m = await first(v2)
    check("OTP tidak bisa dipakai dua kali", m["type"] == "error", m.get("message", ""))

    # 8. Viewer putus -> host ikut ditutup (sesi selesai).
    await v.close()
    check("viewer putus menutup host", await closed(hs))
    await asyncio.sleep(0.2)
    check("registry host bersih", RID not in relay.REMOTE_HOSTS)

    # 9. Sesi baru dengan OTP baru; host putus -> viewer ditutup.
    hs = await host("444444")
    await first(hs)
    v = await viewer("444444")
    await first(v)
    await first(hs)
    await hs.close()
    check("host putus menutup viewer", await closed(v))

    # 10. Host baru untuk ID sama menendang host lama.
    h1 = await host("555555")
    await first(h1)
    h2 = await host("666666")
    await first(h2)
    check("host lama ditendang host baru", await closed(h1))
    v = await viewer("555555")
    m = await first(v)
    check("OTP host lama tidak berlaku", m["type"] == "error")
    v = await viewer("666666")
    check("OTP host baru berlaku", (await first(v))["type"] == "joined")
    await v.close()
    await closed(h2)

    # 11. Batas sesi bersamaan.
    relay.REMOTE_MAX_SESSIONS = 1
    ha = await host("777777", rid="111111111", secret="a" * 40)
    await first(ha)
    va = await viewer("777777", rid="111111111")
    await first(va)
    hb = await host("888888", rid="222222222", secret="b" * 40)
    await first(hb)
    vb = await viewer("888888", rid="222222222")
    m = await first(vb)
    check("batas sesi bersamaan ditegakkan", m["type"] == "error" and "penuh" in m["message"])
    for ws in (va, ha, hb):
        await ws.close()

    # 12. Rate limit viewer (15 percobaan / 10 menit per IP) — sudah terpakai
    # beberapa di atas; habiskan sisanya.
    last = None
    for _ in range(20):
        ws = await viewer("999999", rid="555555555")
        last = await first(ws)
    check("rate limit percobaan OTP", "Terlalu banyak" in last["message"], last["message"])


asyncio.run(main())
failed = [n for n, ok in results if not ok]
print(f"\n{len(results) - len(failed)}/{len(results)} lulus")
server.should_exit = True
sys.exit(1 if failed else 0)
