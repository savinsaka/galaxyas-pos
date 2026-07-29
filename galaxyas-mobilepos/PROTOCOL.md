# Kontrak Protokol Server Pusat ↔ Mobile POS

App ini adalah **client murni** dari GALAXYAS POS desktop yang berjalan sebagai
*Server Pusat*. Kontrak wire didefinisikan oleh desktop dan **dijaga sinkron
secara manual** — tidak ada codegen.

> ⚠ **Aturan sinkronisasi:** setiap perubahan pada
> `desktop/src-tauri/src/lan.rs` (tabel `dispatch`), `desktop/src-tauri/src/relay.rs`,
> `desktop/src-tauri/src/models.rs`, atau `desktop/src/lib/types.ts` WAJIB
> direplikasi ke `app/src/main/java/com/galaxyas/mobilepos/data/model/Models.kt` +
> `data/network/ApiClient.kt` + tabel di file ini.
> Models.kt memakai field snake_case 1:1 supaya diff terhadap types.ts mekanis.

## Dua jalur, satu kontrak

| Mode | Base URL | Dilayani |
|---|---|---|
| `LOCAL` | `http://<ip>:8899` | `lan.rs` (tiny_http) di PC kasir |
| `ONLINE` | `https://<relay>/s/<store_id>` | relay FastAPI di VPS → `relay.rs` di PC kasir |

Rute di bawah base URL **identik** untuk kedua jalur, jadi `RpcClient` hanya
mengganti `baseUrl` dan tidak punya dua cabang logika:

- `GET <base>/health` — liveness. Jalur LOCAL: dengan header token = validasi
  kredensial (200 sah, 401 salah). Jalur ONLINE: dijawab relay sendiri
  (200 = PC kasir online, 503 = mati) tanpa membangunkan PC kasir.
- `POST <base>/pair` — body `{"code":"<6 karakter>","device_name":"<merk HP>"}`
  → `{"device_id","device_token","store_name"}`. Satu-satunya saat token mentah
  dikirim; PC kasir hanya menyimpan hash SHA-256-nya.
- `POST <base>/rpc/<command>` — panggil command. Body = JSON args (key
  **snake_case**, persis nama parameter command Rust). Sukses = JSON hasil
  (body kosong = null untuk command void). Gagal = `{"error":"..."}` + status 4xx/5xx.

Header wajib untuk `/rpc/*` dan `/health`: `X-Galaxyas-Token`. Isinya **token
perangkat 64 karakter hex** hasil `/pair`. Kode pairing 6 karakter masih
diterima di jalur LOCAL (HP versi lama), tapi **ditolak di jalur ONLINE** —
lihat `lan.rs::authenticate(allow_pairing_code)`.

Port LOCAL default 8899, HTTP polos (LAN privat) — `network_security_config.xml`
mengizinkan cleartext. Jalur ONLINE selalu HTTPS.

### Status khas jalur ONLINE

| Status | Arti | Perilaku app |
|---|---|---|
| 503 | PC kasir tidak terhubung ke relay | tampilkan pesan dari body apa adanya |
| 504 | PC kasir tidak menjawab dalam 25 detik | idem |
| 429 | rate limit relay | idem |

**Relay tidak pernah mengantre.** Permintaan yang gagal karena PC kasir mati
benar-benar hilang — tidak ada yang dieksekusi menyusul saat PC hidup lagi.
Ini disengaja supaya stok/opname selalu mencerminkan keadaan sekarang.

Pesan error koneksi di `RpcClient.CONNECT_ERR_MSG` disalin VERBATIM dari lan.rs
supaya UX konsisten dengan client desktop; mode ONLINE memakai varian
`CONNECT_ERR_MSG_ONLINE` karena menyuruh user "periksa wifi" menyesatkan saat
HP sedang di kuota seluler.

### QR pairing

`store_id` relay 32 karakter hex mustahil diketik benar di HP, jadi desktop
menampilkan QR berisi seluruh isian sekaligus (Pengaturan → Server Pusat →
Tampilkan QR). Isinya JSON dari `lan.rs::PairingPayload`:

```json
{"v":1,"name":"GALAXYAS Toko 1","host":"192.168.18.11","port":8899,
 "relay":"relay.jjapps.net","store_id":"ed2a…","code":"A1B2C3"}
```

- `relay` & `store_id` **kosong bila Akses Online mati di PC** — jangan
  menitipkan alamat yang tidak akan pernah menjawab.
- `host` kosong bila IP LAN tidak terdeteksi; QR tetap dibuat.
- `v` dinaikkan kalau formatnya berubah tak-kompatibel; app HP yang lebih tua
  menampilkan "update dulu" alih-alih salah menafsirkan.

Bentuk JSON-nya **dikunci uji di kedua sisi** karena tidak ada codegen:
`lan.rs::tests::bentuk_json_qr_dikunci_karena_diparse_app_hp` dan
`PairingQrTest.kt`. Mengubah field berarti mengubah `PairingPayload` di
`Models.kt` dan fixture di kedua uji itu dalam satu commit.

Parser di HP (`data/PairingQr.kt`) dipanggil untuk **setiap** kode yang terbaca
kamera — kamera yang sama juga dipakai memindai barcode barang — jadi ia
mengembalikan `NotPairingQr` alih-alih melempar exception.

### Envelope WebSocket relay ↔ PC kasir

Bagian ini hanya menyangkut desktop & relay (app HP tidak melihatnya), tapi
dicatat di sini supaya kontraknya satu tempat — lihat `relay/README.md`.

```
relay → PC : {"id":"<uuid>","kind":"rpc","cmd":"…","args":…,"auth":"<token>"}
relay → PC : {"id":"<uuid>","kind":"pair","code":"…","device_name":"…"}
PC → relay : {"id":"<uuid>","status":200,"body":…}
PC → relay : {"type":"tokens","hashes":[…]}   // penyaring token di relay
```

## 49 command (lan.rs::dispatch @ desktop v1.3.2)

| Command | Args (wire) | Hasil |
|---|---|---|
| list_products | search?, include_inactive?, limit? | ProductWithStock[] |
| list_products_page | search?, include_inactive?, brand?, sort_by?, sort_dir?, limit, offset | ProductPage |
| save_product | input: ProductInput | Product |
| toggle_product_active | id, active | void |
| delete_product | id | void |
| dedupe_products | – | DedupeResult |
| find_by_barcode | barcode | ProductWithStock? |
| adjust_stock | product_id, delta | number (stok baru) |
| set_stock | product_id, qty | number (stok baru) |
| checkout | sale: SaleInput | TransactionDetail |
| list_transactions | from?, to?, search?, limit, offset | TransactionPage |
| get_transaction | id | TransactionDetail? |
| delete_transaction | id | void |
| update_transaction | id, input: SaleInput | TransactionDetail |
| login | username, pin | User? (null = salah) |
| list_users | – | User[] |
| save_user | input: UserInput | User |
| delete_user | id | void |
| create_stock_movement | input: StockMovementInput | StockMovement |
| list_stock_movements | kind?, from?, to?, limit | StockMovement[] |
| delete_stock_movement | id (i64) | void |
| create_opname_special | input: OpnameSpecialInput | OpnameSpecialResult |
| create_stock_movement_batch | input: StockMovementBatchInput | StockMovementBatchDetail |
| list_stock_movement_batches | kind?, from?, to?, limit, offset | StockMovementBatchPage |
| get_stock_movement_batch | id | StockMovementBatchDetail? |
| update_stock_movement_batch | id, items: StockMovementBatchItemInput[], note? | StockMovementBatchDetail |
| delete_stock_movement_batch | id | void |
| list_discounts | – | DiscountPeriod[] |
| save_discount | input: DiscountPeriodInput | DiscountPeriod |
| delete_discount | id | void |
| list_brands | – | Brand[] |
| save_brand | input: BrandInput | Brand |
| delete_brand | id | void |
| list_customers | search?, include_inactive? | Customer[] |
| save_customer | input: CustomerInput | Customer |
| delete_customer | id | void |
| list_expenses | from?, to? | Expense[] |
| save_expense | input: ExpenseInput | Expense |
| delete_expense | id | void |
| get_active_shift | – | Shift? |
| open_shift | input: OpenShiftInput | Shift |
| close_shift | input: CloseShiftInput | Shift |
| list_shifts | limit | Shift[] |
| product_sales_report | from, to, brands[] | ProductSalesRow[] |
| brand_sales_report | from, to, brands[] | BrandSalesRow[] |
| sales_item_detail_report | from, to, brands[] | SalesItemDetailRow[] |
| daily_sales_report | from, to, brands[] | DailySalesRow[] |
| stock_flow_recap | from, to, search? | StockFlowRow[] |
| stock_flow_detail | product_id, from, to | StockFlowDetailRow[] |

Tabel `dispatch` yang sama ini dipakai kedua transport — `relay.rs` memanggil
`lan::dispatch` langsung, jadi menambah command cukup di satu tempat.

Command desktop yang **tidak** dilayani LAN/relay (dan memang tidak dipakai
mobile): settings (per-device), store registry, sync_*, bridge_*, lan_server_*,
relay_*, mobile_device_*, printer Windows, print window. Mobile menggantinya
dengan DataStore lokal + printer Bluetooth sendiri.

## Paritas format struk (EscPos.kt vs escpos.ts)

- Golden fixtures byte di `app/src/test/resources/fixtures/*.hex` dihasilkan
  dari implementasi TS (script di git history; commit Tauri 947e11b).
- Format tanggal struk: `new Date(iso).toLocaleString("id-ID")` di Node =
  `d/M/yyyy, HH.mm.ss` (contoh `23/7/2026, 10.30.00`) — Kotlin memakai pola
  eksplisit itu, BUKAN locale device.
- Uang struk: `"Rp" + pembulatan.toLocaleString("id-ID")` = pemisah ribuan
  titik (contoh `Rp15.000`).
- Lebar kertas: 58mm = 32 kolom, 80mm = 48 kolom.
