# GALAXYAS POS Relay

Penerus permintaan dari app HP (`galaxyas-mobilepos`) ke PC kasir yang
menjalankan GALAXYAS POS sebagai Server Pusat, supaya HP bisa dipakai dari luar
wifi toko.

```
HP ──https──► relay (VPS) ──WebSocket──► PC kasir ──► SQLite
```

PC kasir yang menyambung keluar ke relay, jadi tidak perlu port forwarding, IP
publik, atau lolos CGNAT.

## Prinsip

1. **Tanpa antrian.** PC kasir tidak terhubung → permintaan langsung ditolak
   503. Tidak ada checkout/opname yang ditahan lalu dieksekusi belakangan.
2. **Tanpa data POS.** Relay tidak menyimpan produk, stok, atau transaksi. Satu-
   satunya yang persisten adalah tabel `stores` di `relay.db`.
3. **Relay tidak memegang kredensial HP.** Yang disimpan hanya hash `agent_key`
   per toko; token per-HP dimiliki PC kasir, relay cuma menerima salinan
   hash-nya untuk menyaring token asing lebih awal.

## Rute

| Rute | Dipakai | Keterangan |
|---|---|---|
| `GET /health` | monitoring | liveness relay + jumlah toko online |
| `WS /agent/ws` | PC kasir | header `X-Store-Id` + `X-Agent-Key` |
| `GET /s/{store_id}/health` | HP (tiap 30 dtk) | 200 = PC kasir online, 503 = mati. Dijawab relay, tidak diteruskan |
| `POST /s/{store_id}/pair` | HP | tukar kode pairing 6 karakter jadi token perangkat |
| `POST /s/{store_id}/rpc/{cmd}` | HP | header `X-Galaxyas-Token: <token perangkat>` |

Balasan `4xx/5xx` selalu `{"error":"…"}` — bentuknya sama dengan Server Pusat
LAN supaya app HP tidak perlu dua jalur penanganan error.

## Envelope WebSocket

Relay → PC kasir:

```json
{"id":"<uuid>","kind":"rpc","cmd":"checkout","args":{…},"auth":"<token perangkat>"}
{"id":"<uuid>","kind":"pair","code":"A1B2C3","device_name":"Redmi Note 12"}
```

PC kasir → relay:

```json
{"id":"<uuid>","status":200,"body":{…}}
{"type":"tokens","hashes":["<sha256>", …]}
{"type":"pong"}
```

`status` diteruskan apa adanya ke HP, jadi 401/400 dari PC kasir sampai ke HP
dengan arti yang sama seperti di jalur LAN.

## Uji

```bash
pip install httpx                 # sekali saja
python test_relay.py              # relay + agent tiruan (15 uji)
python scripts/e2e_desktop.py     # relay + agent desktop Rust SUNGGUHAN
```

`test_relay.py` menguji relay sendirian. `scripts/e2e_desktop.py` menyalakan
relay di port sementara lalu menjalankan uji Rust `relay_e2e` di
`desktop/src-tauri` — ini yang membuktikan handshake WebSocket, auth agent,
envelope bolak-balik, perubahan stok benar-benar tersimpan, dan janji
**tanpa antrian** (agent dimatikan → 503 seketika, dan tidak ada permintaan
tertahan yang menyusul jalan). Database POS-nya di memori, tidak menyentuh data
toko asli.

## Jalankan lokal (untuk uji tanpa VPS)

```bash
pip install -r requirements.txt
python scripts/add_store.py "Toko Uji"
uvicorn app:app --port 9010
```

Lalu di desktop isi URL relay `ws://localhost:9010/agent/ws` beserta store id dan
agent key yang tercetak.

Deploy ke VPS: lihat [DEPLOY.md](DEPLOY.md).
