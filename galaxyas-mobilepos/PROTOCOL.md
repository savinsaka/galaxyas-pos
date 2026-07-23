# Kontrak Protokol Server Pusat ↔ Mobile POS

App ini adalah **client LAN murni** dari GALAXYAS POS desktop yang berjalan
sebagai *Server Pusat*. Kontrak wire didefinisikan oleh desktop dan **dijaga
sinkron secara manual** — tidak ada codegen.

> ⚠ **Aturan sinkronisasi:** setiap perubahan pada
> `desktop/src-tauri/src/lan.rs` (tabel `dispatch`), `desktop/src-tauri/src/models.rs`,
> atau `desktop/src/lib/types.ts` WAJIB direplikasi ke
> `app/src/main/java/com/galaxyas/mobilepos/data/model/Models.kt` +
> `data/network/ApiClient.kt` + tabel di file ini.
> Models.kt memakai field snake_case 1:1 supaya diff terhadap types.ts mekanis.

## Transport

- `GET http://<host>:<port>/health` — liveness. Dengan header token: validasi
  kode pairing (200 = sah, 401 = "kode pairing salah"). Dipakai layar pairing
  dan ConnectionWatcher.
- `POST http://<host>:<port>/rpc/<command>` — panggil command. Body = JSON args
  (key **snake_case**, persis nama parameter command Rust). Sukses = JSON hasil
  (body kosong = null untuk command void). Gagal = `{"error":"..."}` + status 4xx/5xx.
- Header wajib: `X-Galaxyas-Token: <kode pairing 6 karakter>`.
- Port default 8899. HTTP polos (LAN privat) — `network_security_config.xml`
  mengizinkan cleartext.
- Pesan error koneksi di `RpcClient.CONNECT_ERR_MSG` disalin VERBATIM dari
  lan.rs supaya UX konsisten dengan client desktop.

## 44 command (lan.rs::dispatch @ desktop v1.1.4)

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

Command desktop yang **tidak** dilayani LAN (dan memang tidak dipakai mobile):
settings (per-device), store registry, sync_*, bridge_*, lan_server_*, printer
Windows, print window. Mobile menggantinya dengan DataStore lokal + printer
Bluetooth sendiri.

## Paritas format struk (EscPos.kt vs escpos.ts)

- Golden fixtures byte di `app/src/test/resources/fixtures/*.hex` dihasilkan
  dari implementasi TS (script di git history; commit Tauri 947e11b).
- Format tanggal struk: `new Date(iso).toLocaleString("id-ID")` di Node =
  `d/M/yyyy, HH.mm.ss` (contoh `23/7/2026, 10.30.00`) — Kotlin memakai pola
  eksplisit itu, BUKAN locale device.
- Uang struk: `"Rp" + pembulatan.toLocaleString("id-ID")` = pemisah ribuan
  titik (contoh `Rp15.000`).
- Lebar kertas: 58mm = 32 kolom, 80mm = 48 kolom.
