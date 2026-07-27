use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{
    Product, ProductInput, ProductWithStock, SaleInput, SyncLogEntry, Transaction,
    TransactionDetail, TransactionItem,
};

/// Buat semua tabel lokal bila belum ada. Stok & transaksi 100% lokal (tidak di-sync).
pub fn init_schema(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        r#"
        PRAGMA journal_mode = WAL;
        PRAGMA foreign_keys = ON;

        -- Master data (mirror dari server) + metadata sync.
        CREATE TABLE IF NOT EXISTS products (
            id               TEXT PRIMARY KEY,
            name             TEXT NOT NULL,
            barcode          TEXT,
            category         TEXT,
            brand            TEXT,
            unit             TEXT,
            sell_price       REAL NOT NULL DEFAULT 0,
            cost_price       REAL NOT NULL DEFAULT 0,
            default_discount REAL NOT NULL DEFAULT 0,
            is_active        INTEGER NOT NULL DEFAULT 1,
            is_deleted       INTEGER NOT NULL DEFAULT 0,
            updated_at       TEXT NOT NULL,
            dirty            INTEGER NOT NULL DEFAULT 0  -- 1 = berubah lokal, belum di-upload
        );
        CREATE INDEX IF NOT EXISTS idx_products_barcode ON products(barcode);
        CREATE INDEX IF NOT EXISTS idx_products_dirty ON products(dirty);

        -- Stok per barang (LOKAL, tidak di-sync).
        CREATE TABLE IF NOT EXISTS stock (
            product_id TEXT PRIMARY KEY REFERENCES products(id) ON DELETE CASCADE,
            qty        REAL NOT NULL DEFAULT 0,
            updated_at TEXT NOT NULL
        );

        -- Transaksi penjualan (LOKAL, tidak di-sync).
        CREATE TABLE IF NOT EXISTS transactions (
            id             TEXT PRIMARY KEY,
            invoice_no     TEXT NOT NULL,
            cashier_id     TEXT NOT NULL,
            subtotal       REAL NOT NULL,
            discount       REAL NOT NULL,
            total          REAL NOT NULL,
            paid           REAL NOT NULL,
            change         REAL NOT NULL,
            payment_method TEXT NOT NULL,
            created_at     TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_tx_created ON transactions(created_at);

        CREATE TABLE IF NOT EXISTS transaction_items (
            id             INTEGER PRIMARY KEY AUTOINCREMENT,
            transaction_id TEXT NOT NULL REFERENCES transactions(id) ON DELETE CASCADE,
            product_id     TEXT NOT NULL,
            name           TEXT NOT NULL,
            price          REAL NOT NULL,
            qty            REAL NOT NULL,
            discount       REAL NOT NULL DEFAULT 0,
            line_total     REAL NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_txitems_tx ON transaction_items(transaction_id);

        -- Kasir / pengguna lokal (tidak di-sync).
        CREATE TABLE IF NOT EXISTS users (
            id          TEXT PRIMARY KEY,
            username    TEXT NOT NULL UNIQUE,
            name        TEXT NOT NULL,
            role        TEXT NOT NULL DEFAULT 'kasir',
            pin         TEXT,
            permissions TEXT NOT NULL DEFAULT '[]'
        );

        -- Riwayat pergerakan stok (Item Masuk/Keluar/Opname/Penjualan). Lokal.
        CREATE TABLE IF NOT EXISTS stock_movements (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            product_id  TEXT NOT NULL,
            kind        TEXT NOT NULL,        -- in | out | opname | sale
            qty         REAL NOT NULL,
            stock_after REAL NOT NULL DEFAULT 0,
            note        TEXT,
            user_id     TEXT,
            created_at  TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_mov_kind ON stock_movements(kind);
        CREATE INDEX IF NOT EXISTS idx_mov_created ON stock_movements(created_at);
        -- Dipakai Alur Barang (buku besar per barang): telusur mutasi satu barang
        -- dan pencarian "stok terakhir sebelum tanggal X" per barang.
        CREATE INDEX IF NOT EXISTS idx_mov_product ON stock_movements(product_id, created_at);

        -- Diskon periodik (master data; CRUD lokal).
        CREATE TABLE IF NOT EXISTS discount_periods (
            id            TEXT PRIMARY KEY,
            scope         TEXT NOT NULL,       -- item | brand
            target        TEXT NOT NULL,
            target_label  TEXT,
            discount_type TEXT NOT NULL,       -- amount | percent
            value         REAL NOT NULL,
            days          TEXT NOT NULL,       -- everyday | csv hari
            is_active     INTEGER NOT NULL DEFAULT 1,
            updated_at    TEXT NOT NULL
        );

        -- Daftar merek (master, CRUD lokal).
        CREATE TABLE IF NOT EXISTS brands (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL UNIQUE,
            updated_at TEXT NOT NULL
        );

        -- Pelanggan (lokal, tidak di-sync).
        CREATE TABLE IF NOT EXISTS customers (
            id         TEXT PRIMARY KEY,
            name       TEXT NOT NULL,
            phone      TEXT,
            email      TEXT,
            address    TEXT,
            note       TEXT,
            is_active  INTEGER NOT NULL DEFAULT 1,
            updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_customers_name ON customers(name);

        -- Pengeluaran / kas keluar operasional (lokal).
        CREATE TABLE IF NOT EXISTS expenses (
            id         TEXT PRIMARY KEY,
            date       TEXT NOT NULL,       -- YYYY-MM-DD
            category   TEXT NOT NULL,
            amount     REAL NOT NULL,
            note       TEXT,
            user_id    TEXT,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_expenses_date ON expenses(date);

        -- Shift kasir (buka/tutup, rekonsiliasi kas). Lokal.
        CREATE TABLE IF NOT EXISTS shifts (
            id             TEXT PRIMARY KEY,
            user_id        TEXT NOT NULL,
            user_name      TEXT NOT NULL,
            opening_cash   REAL NOT NULL DEFAULT 0,
            closing_cash   REAL,             -- diisi saat tutup (uang fisik dihitung)
            expected_cash  REAL,             -- modal awal + total tunai penjualan (dihitung sistem)
            difference     REAL,             -- closing_cash - expected_cash
            note           TEXT,
            opened_at      TEXT NOT NULL,
            closed_at      TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_shifts_opened ON shifts(opened_at);

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT
        );

        -- Batch Item Masuk/Keluar: satu baris per "Simpan Semua Item" (transaksi),
        -- setiap barang di dalamnya jadi satu baris stock_movements dengan batch_id ini.
        CREATE TABLE IF NOT EXISTS stock_movement_batches (
            id         TEXT PRIMARY KEY,
            no         TEXT NOT NULL,
            kind       TEXT NOT NULL,       -- in | out
            note       TEXT,
            user_id    TEXT,
            created_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_batch_kind ON stock_movement_batches(kind);
        CREATE INDEX IF NOT EXISTS idx_batch_created ON stock_movement_batches(created_at);
        "#,
    )?;

    // Migrasi: kolom permissions pada tabel users.
    let has_perm: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('users') WHERE name='permissions'")?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !has_perm {
        conn.execute("ALTER TABLE users ADD COLUMN permissions TEXT NOT NULL DEFAULT '[]'", [])?;
    }

    // Migrasi: kolom code & priority pada tabel discount_periods.
    let has_dp_code: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('discount_periods') WHERE name='code'")?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !has_dp_code {
        conn.execute("ALTER TABLE discount_periods ADD COLUMN code TEXT NOT NULL DEFAULT ''", [])?;
    }
    let has_dp_priority: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('discount_periods') WHERE name='priority'")?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !has_dp_priority {
        conn.execute("ALTER TABLE discount_periods ADD COLUMN priority INTEGER NOT NULL DEFAULT 1", [])?;
    }

    // Migrasi: kolom customer_id & shift_id pada tabel transactions.
    let has_tx_customer: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('transactions') WHERE name='customer_id'")?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !has_tx_customer {
        conn.execute("ALTER TABLE transactions ADD COLUMN customer_id TEXT", [])?;
    }
    let has_tx_shift: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('transactions') WHERE name='shift_id'")?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !has_tx_shift {
        conn.execute("ALTER TABLE transactions ADD COLUMN shift_id TEXT", [])?;
    }

    // Migrasi: kolom batch_id pada tabel stock_movements (grouping Item Masuk/Keluar).
    let has_mov_batch: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('stock_movements') WHERE name='batch_id'")?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !has_mov_batch {
        conn.execute("ALTER TABLE stock_movements ADD COLUMN batch_id TEXT", [])?;
        conn.execute("CREATE INDEX IF NOT EXISTS idx_mov_batch ON stock_movements(batch_id)", [])?;
    }

    // Migrasi: kolom paid_cash & paid_qris pada tabel transactions, untuk
    // pembayaran metode "Kombinasi" (sebagian tunai, sebagian QRIS).
    let has_tx_paid_cash: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('transactions') WHERE name='paid_cash'")?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !has_tx_paid_cash {
        conn.execute("ALTER TABLE transactions ADD COLUMN paid_cash REAL", [])?;
        conn.execute("ALTER TABLE transactions ADD COLUMN paid_qris REAL", [])?;
    }

    // Migrasi: kolom ever_synced pada tabel products — 1 = server sudah pernah
    // tahu ID produk ini (lewat pull ATAU push yang berhasil), jadi LWW timestamp
    // normal berlaku. 0 = murni lokal, belum pernah "kenal" server — kalau nanti
    // pull membawa ID yang sama, data server WAJIB menang mutlak walau updated_at
    // lokal terlihat lebih baru (skenario reset data / import awal).
    let has_prod_ever_synced: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('products') WHERE name='ever_synced'")?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !has_prod_ever_synced {
        conn.execute("ALTER TABLE products ADD COLUMN ever_synced INTEGER NOT NULL DEFAULT 0", [])?;
    }

    Ok(())
}

/// Isi data awal (kasir & setting default) bila kosong.
pub fn seed_defaults(conn: &Connection) -> AppResult<()> {
    const ALL_PERMS: &str = r#"["master","penjualan","persediaan","laporan","pengaturan"]"#;
    let user_count: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
    if user_count == 0 {
        conn.execute(
            "INSERT INTO users (id, username, name, role, pin, permissions)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                Uuid::new_v4().to_string(),
                "admin",
                "Administrator",
                "admin",
                "1234",
                ALL_PERMS
            ],
        )?;
    }
    // Pastikan akun admin selalu punya seluruh hak akses.
    conn.execute(
        "UPDATE users SET permissions = ?1 WHERE role = 'admin'",
        params![ALL_PERMS],
    )?;

    let defaults = [
        ("store_id", "toko-001"),
        ("store_name", "GALAXYAS Toko 1"),
        ("server_url", "http://localhost:8000"),
        ("receipt_footer", "Terima kasih telah berbelanja!"),
        ("tax_percent", "0"),
        ("last_pull_at", ""),
        ("invoice_seq", "0"),
    ];
    for (k, v) in defaults {
        conn.execute(
            "INSERT OR IGNORE INTO settings (key, value) VALUES (?1, ?2)",
            params![k, v],
        )?;
    }

    backfill_brands_from_products(conn)?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> AppResult<Option<String>> {
    let v = conn
        .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
            r.get::<_, Option<String>>(0)
        })
        .optional()?
        .flatten();
    Ok(v)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> AppResult<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn all_settings(conn: &Connection) -> AppResult<Vec<(String, String)>> {
    let mut stmt = conn.prepare("SELECT key, COALESCE(value,'') FROM settings ORDER BY key")?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

fn map_product(row: &rusqlite::Row) -> rusqlite::Result<Product> {
    Ok(Product {
        id: row.get("id")?,
        name: row.get("name")?,
        barcode: row.get("barcode")?,
        category: row.get("category")?,
        brand: row.get("brand")?,
        unit: row.get("unit")?,
        sell_price: row.get("sell_price")?,
        cost_price: row.get("cost_price")?,
        default_discount: row.get("default_discount")?,
        is_active: row.get::<_, i64>("is_active")? != 0,
        is_deleted: row.get::<_, i64>("is_deleted")? != 0,
        updated_at: row.get("updated_at")?,
    })
}

/// Daftar barang + stok. `search` mencocokkan nama atau barcode. `limit`
/// dikirim apa adanya ke SQLite; `None`/negatif berarti tanpa batas (dipakai
/// oleh pemanggil "muat semua utk autocomplete lokal"; pemanggil yang butuh
/// hasil ringkas seperti pencarian di kasir mengirim limit kecil).
pub fn list_products(
    conn: &Connection,
    search: Option<String>,
    include_inactive: bool,
    limit: Option<i64>,
) -> AppResult<Vec<ProductWithStock>> {
    let like = format!("%{}%", search.unwrap_or_default());
    let active_clause = if include_inactive { "1=1" } else { "p.is_active = 1" };
    let sql = format!(
        "SELECT p.*, COALESCE(s.qty, 0) AS stock_qty
         FROM products p
         LEFT JOIN stock s ON s.product_id = p.id
         WHERE p.is_deleted = 0 AND {active_clause}
           AND (p.name LIKE ?1 OR COALESCE(p.barcode,'') LIKE ?1)
         ORDER BY p.name ASC
         LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![like, limit.unwrap_or(-1)], |row| {
            let stock_qty: f64 = row.get("stock_qty")?;
            Ok(ProductWithStock { product: map_product(row)?, stock_qty })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Daftar barang dengan paginasi (limit/offset), filter merek, dan sort
/// pilihan kolom. Dipakai oleh tabel Data Barang agar tidak me-render
/// ribuan baris sekaligus.
pub fn list_products_page(
    conn: &Connection,
    search: Option<String>,
    include_inactive: bool,
    brand: Option<String>,
    sort_by: Option<String>,
    sort_dir: Option<String>,
    limit: i64,
    offset: i64,
) -> AppResult<crate::models::ProductPage> {
    let like = format!("%{}%", search.unwrap_or_default());
    let active_clause = if include_inactive { "1=1" } else { "p.is_active = 1" };
    let col = match sort_by.as_deref() {
        Some("sell_price") => "p.sell_price",
        Some("cost_price") => "p.cost_price",
        Some("stock_qty") => "stock_qty",
        Some("updated_at") => "p.updated_at",
        Some("barcode") => "p.barcode",
        _ => "p.name COLLATE NOCASE",
    };
    let dir = if sort_dir.as_deref() == Some("desc") { "DESC" } else { "ASC" };

    let where_sql = format!(
        "FROM products p LEFT JOIN stock s ON s.product_id = p.id
         WHERE p.is_deleted = 0 AND {active_clause}
           AND (p.name LIKE ?1 OR COALESCE(p.barcode,'') LIKE ?1)
           AND (?2 IS NULL OR p.brand = ?2)"
    );

    let total: i64 = conn.query_row(
        &format!("SELECT COUNT(*) {where_sql}"),
        params![like, brand],
        |r| r.get(0),
    )?;

    let sql = format!(
        "SELECT p.*, COALESCE(s.qty, 0) AS stock_qty {where_sql}
         ORDER BY {col} {dir}
         LIMIT ?3 OFFSET ?4"
    );
    let mut stmt = conn.prepare(&sql)?;
    let items = stmt
        .query_map(params![like, brand, limit, offset], |row| {
            let stock_qty: f64 = row.get("stock_qty")?;
            Ok(ProductWithStock { product: map_product(row)?, stock_qty })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(crate::models::ProductPage { items, total })
}

pub fn find_by_barcode(conn: &Connection, barcode: &str) -> AppResult<Option<ProductWithStock>> {
    let res = conn
        .query_row(
            "SELECT p.*, COALESCE(s.qty,0) AS stock_qty
             FROM products p LEFT JOIN stock s ON s.product_id = p.id
             WHERE p.barcode = ?1 COLLATE NOCASE AND p.is_deleted = 0 AND p.is_active = 1",
            params![barcode],
            |row| {
                let stock_qty: f64 = row.get("stock_qty")?;
                Ok(ProductWithStock { product: map_product(row)?, stock_qty })
            },
        )
        .optional()?;
    Ok(res)
}

/// Tambah / edit barang. Menandai `dirty = 1` & memperbarui `updated_at` (untuk sync).
pub fn upsert_product(conn: &Connection, input: ProductInput) -> AppResult<Product> {
    let now = Utc::now().to_rfc3339();
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let existed: bool = conn
        .query_row("SELECT 1 FROM products WHERE id = ?1", params![id], |_| Ok(true))
        .optional()?
        .unwrap_or(false);

    conn.execute(
        "INSERT INTO products
            (id, name, barcode, category, brand, unit, sell_price, cost_price,
             default_discount, is_active, is_deleted, updated_at, dirty)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,0,?11,1)
         ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, barcode=excluded.barcode, category=excluded.category,
            brand=excluded.brand, unit=excluded.unit, sell_price=excluded.sell_price,
            cost_price=excluded.cost_price, default_discount=excluded.default_discount,
            is_active=excluded.is_active, updated_at=excluded.updated_at, dirty=1",
        params![
            id,
            input.name,
            input.barcode,
            input.category,
            input.brand,
            input.unit,
            input.sell_price,
            input.cost_price,
            input.default_discount,
            input.is_active as i64,
            now,
        ],
    )?;

    if !existed {
        conn.execute(
            "INSERT OR IGNORE INTO stock (product_id, qty, updated_at) VALUES (?1, 0, ?2)",
            params![id, now],
        )?;
    }

    ensure_brand_exists(conn, input.brand.as_deref(), &now)?;

    get_product(conn, &id)?.ok_or_else(|| AppError::Other("produk gagal disimpan".into()))
}

/// Pastikan nama merek dari barang (input manual atau import) muncul di
/// tabel `brands` juga, supaya tampil di Daftar Merek, filter Data Barang,
/// dan filter Laporan Item — bukan cuma tersimpan sebagai teks bebas di
/// `products.brand`. Dedup otomatis lewat UNIQUE(name) + INSERT OR IGNORE.
fn ensure_brand_exists(conn: &Connection, brand: Option<&str>, now: &str) -> AppResult<()> {
    let Some(name) = brand.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(());
    };
    conn.execute(
        "INSERT OR IGNORE INTO brands (id, name, updated_at) VALUES (?1, ?2, ?3)",
        params![Uuid::new_v4().to_string(), name, now],
    )?;
    Ok(())
}

/// Backfill sekali jalan: isi `brands` dari nilai `products.brand` yang sudah
/// ada tapi belum tercatat di tabel `brands` (mis. dari import lama sebelum
/// perbaikan ini ada). Idempoten — aman dipanggil tiap start aplikasi.
pub fn backfill_brands_from_products(conn: &Connection) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    let mut stmt = conn.prepare(
        "SELECT DISTINCT brand FROM products WHERE brand IS NOT NULL AND trim(brand) != ''",
    )?;
    let names: Vec<String> = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    for name in names {
        ensure_brand_exists(conn, Some(&name), &now)?;
    }
    Ok(())
}

pub fn get_product(conn: &Connection, id: &str) -> AppResult<Option<Product>> {
    let p = conn
        .query_row("SELECT * FROM products WHERE id = ?1", params![id], map_product)
        .optional()?;
    Ok(p)
}

/// Hapus barang (soft delete / tombstone agar penghapusan ikut tersinkron).
pub fn delete_product(conn: &Connection, id: &str) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE products SET is_deleted = 1, is_active = 0, updated_at = ?2, dirty = 1 WHERE id = ?1",
        params![id, now],
    )?;
    Ok(())
}

/// Non-aktifkan / aktifkan barang (master data, di-sync).
pub fn set_product_active(conn: &Connection, id: &str, active: bool) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE products SET is_active = ?2, updated_at = ?3, dirty = 1 WHERE id = ?1",
        params![id, active as i64, now],
    )?;
    Ok(())
}

/// Cari & tandai barang duplikat (barcode sama, aktif/non-deleted). Barang
/// yang paling baru di-update dipertahankan; sisanya di-soft-delete supaya
/// tidak lagi muncul di kasir maupun pencarian, tanpa menghapus riwayat
/// transaksi/pergerakan stok yang mungkin mengacu ke id lama.
pub fn dedupe_products_by_barcode(conn: &Connection) -> AppResult<crate::models::DedupeResult> {
    let now = Utc::now().to_rfc3339();
    let barcodes: Vec<String> = {
        let mut stmt = conn.prepare(
            "SELECT barcode FROM products
             WHERE is_deleted = 0 AND barcode IS NOT NULL AND TRIM(barcode) != ''
             GROUP BY barcode COLLATE NOCASE HAVING COUNT(*) > 1",
        )?;
        let x = stmt.query_map([], |r| r.get(0))?.collect::<Result<Vec<_>, _>>()?;
        x
    };

    let mut details = Vec::new();
    let mut removed_total = 0i64;

    for barcode in &barcodes {
        let rows: Vec<(String, String)> = {
            let mut stmt = conn.prepare(
                "SELECT id, name FROM products
                 WHERE barcode = ?1 COLLATE NOCASE AND is_deleted = 0
                 ORDER BY updated_at DESC, rowid DESC",
            )?;
            let x = stmt
                .query_map(params![barcode], |r| Ok((r.get(0)?, r.get(1)?)))?
                .collect::<Result<Vec<_>, _>>()?;
            x
        };
        if rows.len() < 2 {
            continue;
        }
        let kept_name = rows[0].1.clone();
        for (id, _) in &rows[1..] {
            conn.execute(
                "UPDATE products SET is_deleted = 1, is_active = 0, updated_at = ?2, dirty = 1 WHERE id = ?1",
                params![id, now],
            )?;
        }
        let removed_count = (rows.len() - 1) as i64;
        removed_total += removed_count;
        details.push(crate::models::DedupeDetail {
            barcode: barcode.clone(),
            kept_name,
            removed_count,
        });
    }

    Ok(crate::models::DedupeResult {
        groups: details.len() as i64,
        removed: removed_total,
        details,
    })
}

/// Hapus SELURUH data transaksional & master (barang, stok, transaksi,
/// pergerakan stok, diskon, merek). Akun pengguna & pengaturan toko TIDAK
/// ikut terhapus agar aplikasi tetap bisa login setelah reset — KECUALI
/// `last_pull_at`: watermark itu HARUS ikut dikosongkan, karena kalau tidak,
/// sync-in berikutnya cuma minta delta sejak watermark lama (bukan full pull)
/// padahal tabel products sudah kosong total — barang tidak akan balik.
pub fn reset_data(conn: &Connection) -> AppResult<()> {
    conn.execute_batch(
        "DELETE FROM transaction_items;
         DELETE FROM transactions;
         DELETE FROM stock_movements;
         DELETE FROM stock;
         DELETE FROM discount_periods;
         DELETE FROM products;
         DELETE FROM brands;
         DELETE FROM settings WHERE key = 'last_pull_at';",
    )?;
    Ok(())
}

/// Penyesuaian stok (Barang Masuk / Keluar / Opname). Stok TIDAK di-sync.
pub fn adjust_stock(conn: &Connection, product_id: &str, delta: f64) -> AppResult<f64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO stock (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(product_id) DO UPDATE SET qty = qty + ?2, updated_at = ?3",
        params![product_id, delta, now],
    )?;
    let qty: f64 = conn.query_row(
        "SELECT qty FROM stock WHERE product_id = ?1",
        params![product_id],
        |r| r.get(0),
    )?;
    Ok(qty)
}

/// Set stok absolut (opname).
pub fn set_stock(conn: &Connection, product_id: &str, qty: f64) -> AppResult<f64> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO stock (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(product_id) DO UPDATE SET qty = ?2, updated_at = ?3",
        params![product_id, qty, now],
    )?;
    Ok(qty)
}

fn next_invoice_no(conn: &Connection) -> AppResult<String> {
    let seq: i64 = get_setting(conn, "invoice_seq")?
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
        + 1;
    set_setting(conn, "invoice_seq", &seq.to_string())?;
    let date = Utc::now().format("%Y%m%d");
    Ok(format!("INV-{date}-{seq:04}"))
}

/// Pakai override waktu (dari admin, mis. mencatat transaksi mundur) bila
/// valid RFC3339; kalau kosong/tidak valid, jatuhkan ke waktu sekarang.
fn resolve_created_at(override_at: &Option<String>) -> String {
    match override_at {
        Some(s) if chrono::DateTime::parse_from_rfc3339(s).is_ok() => s.clone(),
        _ => Utc::now().to_rfc3339(),
    }
}

/// Simpan transaksi penjualan & kurangi stok dalam satu transaksi DB.
pub fn create_sale(conn: &mut Connection, input: SaleInput) -> AppResult<TransactionDetail> {
    if input.items.is_empty() {
        return Err(AppError::Other("transaksi tidak boleh kosong".into()));
    }
    let now = resolve_created_at(&input.created_at);
    let tx_id = Uuid::new_v4().to_string();

    let mut subtotal = 0.0;
    let mut total_discount = 0.0;
    let mut items: Vec<TransactionItem> = Vec::with_capacity(input.items.len());
    for it in &input.items {
        let line_gross = it.price * it.qty;
        let line_total = (line_gross - it.discount).max(0.0);
        subtotal += line_gross;
        total_discount += it.discount;
        items.push(TransactionItem {
            product_id: it.product_id.clone(),
            name: it.name.clone(),
            price: it.price,
            qty: it.qty,
            discount: it.discount,
            line_total,
        });
    }
    let total = (subtotal - total_discount).max(0.0);
    if input.paid < total {
        return Err(AppError::Other("pembayaran kurang dari total".into()));
    }
    let change = input.paid - total;

    // Cek stok cukup sebelum memulai transaksi DB.
    for it in &input.items {
        let current_stock: f64 = conn
            .query_row(
                "SELECT COALESCE(qty,0) FROM stock WHERE product_id = ?1",
                params![it.product_id],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        if current_stock < it.qty {
            return Err(AppError::Other(format!(
                "Stok \"{}\" tidak cukup (tersedia {:.0}, dibutuhkan {:.0}).",
                it.name, current_stock, it.qty
            )));
        }
    }

    // Nomor invoice baru dibangkitkan di sini, setelah semua validasi lolos
    // dan sebagai bagian dari transaksi atomic — supaya checkout yang gagal
    // (pembayaran kurang / stok tidak cukup) tidak "membakar" nomor urut.
    let db_tx = conn.transaction()?;
    let invoice_no = next_invoice_no(&db_tx)?;
    db_tx.execute(
        "INSERT INTO transactions
            (id, invoice_no, cashier_id, subtotal, discount, total, paid, change, payment_method,
             created_at, customer_id, shift_id, paid_cash, paid_qris)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)",
        params![
            tx_id, invoice_no, input.cashier_id, subtotal, total_discount, total,
            input.paid, change, input.payment_method, now, input.customer_id, input.shift_id,
            input.paid_cash, input.paid_qris
        ],
    )?;
    for it in &items {
        db_tx.execute(
            "INSERT INTO transaction_items
                (transaction_id, product_id, name, price, qty, discount, line_total)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![tx_id, it.product_id, it.name, it.price, it.qty, it.discount, it.line_total],
        )?;
        // Stok berkurang otomatis saat penjualan.
        db_tx.execute(
            "INSERT INTO stock (product_id, qty, updated_at) VALUES (?1, -?2, ?3)
             ON CONFLICT(product_id) DO UPDATE SET qty = qty - ?2, updated_at = ?3",
            params![it.product_id, it.qty, now],
        )?;
        let after: f64 = db_tx.query_row(
            "SELECT qty FROM stock WHERE product_id = ?1",
            params![it.product_id],
            |r| r.get(0),
        )?;
        db_tx.execute(
            "INSERT INTO stock_movements
                (product_id, kind, qty, stock_after, note, user_id, created_at)
             VALUES (?1, 'sale', ?2, ?3, ?4, ?5, ?6)",
            params![it.product_id, it.qty, after, format!("Invoice {invoice_no}"), input.cashier_id, now],
        )?;
    }
    db_tx.commit()?;

    Ok(TransactionDetail {
        header: Transaction {
            id: tx_id,
            invoice_no,
            cashier_id: input.cashier_id,
            subtotal,
            discount: total_discount,
            total,
            paid: input.paid,
            change,
            payment_method: input.payment_method,
            created_at: now,
            customer_id: input.customer_id,
            shift_id: input.shift_id,
            paid_cash: input.paid_cash,
            paid_qris: input.paid_qris,
        },
        items,
    })
}

fn map_transaction(r: &rusqlite::Row) -> rusqlite::Result<Transaction> {
    Ok(Transaction {
        id: r.get("id")?,
        invoice_no: r.get("invoice_no")?,
        cashier_id: r.get("cashier_id")?,
        subtotal: r.get("subtotal")?,
        discount: r.get("discount")?,
        total: r.get("total")?,
        paid: r.get("paid")?,
        change: r.get("change")?,
        payment_method: r.get("payment_method")?,
        created_at: r.get("created_at")?,
        customer_id: r.get("customer_id")?,
        shift_id: r.get("shift_id")?,
        paid_cash: r.get("paid_cash")?,
        paid_qris: r.get("paid_qris")?,
    })
}

fn transaction_where(
    from: &Option<String>,
    to: &Option<String>,
    search: &Option<String>,
) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut sql = String::from(" WHERE 1=1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(f) = from {
        sql.push_str(" AND date(created_at) >= date(?");
        sql.push_str(&(args.len() + 1).to_string());
        sql.push(')');
        args.push(Box::new(f.clone()));
    }
    if let Some(t) = to {
        sql.push_str(" AND date(created_at) <= date(?");
        sql.push_str(&(args.len() + 1).to_string());
        sql.push(')');
        args.push(Box::new(t.clone()));
    }
    if let Some(s) = search {
        let term = s.trim();
        if !term.is_empty() {
            sql.push_str(" AND invoice_no LIKE ?");
            sql.push_str(&(args.len() + 1).to_string());
            sql.push_str(" ESCAPE '\\'");
            let escaped = term.replace('\\', "\\\\").replace('%', "\\%").replace('_', "\\_");
            args.push(Box::new(format!("%{escaped}%")));
        }
    }
    (sql, args)
}

pub fn list_transactions(
    conn: &Connection,
    from: Option<String>,
    to: Option<String>,
    search: Option<String>,
    limit: i64,
    offset: i64,
) -> AppResult<crate::models::TransactionPage> {
    let (count_where, count_args) = transaction_where(&from, &to, &search);
    let total: i64 = {
        let count_sql = format!("SELECT COUNT(*) FROM transactions{count_where}");
        let params_ref: Vec<&dyn rusqlite::ToSql> = count_args.iter().map(|b| b.as_ref()).collect();
        conn.query_row(&count_sql, params_ref.as_slice(), |r| r.get(0))?
    };

    let (where_sql, mut args) = transaction_where(&from, &to, &search);
    let sql = format!(
        "SELECT * FROM transactions{where_sql} ORDER BY created_at DESC LIMIT ?{} OFFSET ?{}",
        args.len() + 1,
        args.len() + 2,
    );
    args.push(Box::new(limit));
    args.push(Box::new(offset));

    let mut stmt = conn.prepare(&sql)?;
    let params_ref: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let items = stmt
        .query_map(params_ref.as_slice(), map_transaction)?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(crate::models::TransactionPage { items, total })
}

pub fn get_transaction(conn: &Connection, id: &str) -> AppResult<Option<TransactionDetail>> {
    let header = conn
        .query_row("SELECT * FROM transactions WHERE id = ?1", params![id], map_transaction)
        .optional()?;

    let Some(header) = header else { return Ok(None) };

    let mut stmt = conn.prepare(
        "SELECT product_id, name, price, qty, discount, line_total
         FROM transaction_items WHERE transaction_id = ?1 ORDER BY id ASC",
    )?;
    let items = stmt
        .query_map(params![id], |r| {
            Ok(TransactionItem {
                product_id: r.get(0)?,
                name: r.get(1)?,
                price: r.get(2)?,
                qty: r.get(3)?,
                discount: r.get(4)?,
                line_total: r.get(5)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(TransactionDetail { header, items }))
}

// ---------- Sinkronisasi master data ----------

/// Produk yang berubah lokal & belum di-upload.
pub fn get_dirty_products(conn: &Connection) -> AppResult<Vec<Product>> {
    let mut stmt = conn.prepare("SELECT * FROM products WHERE dirty = 1")?;
    let rows = stmt.query_map([], map_product)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Dipanggil setelah push sukses. `ever_synced=1` ikut di-set: begitu produk
/// berhasil dikirim ke server, server sudah "kenal" ID ini, jadi LWW timestamp
/// normal berlaku untuk pull berikutnya (kalau tidak, produk yang baru dibuat
/// lokal lalu di-push, saat di-pull lagi malah dianggap "belum pernah sync"
/// dan bisa ketimpa mundur oleh respons server yang lebih lama).
pub fn mark_products_synced(conn: &Connection, ids: &[String]) -> AppResult<()> {
    for id in ids {
        conn.execute("UPDATE products SET dirty = 0, ever_synced = 1 WHERE id = ?1", params![id])?;
    }
    Ok(())
}

/// Terapkan produk hasil pull dari server dengan Last Write Wins berbasis
/// `updated_at` — KECUALI kalau produk itu `ever_synced=0` (belum pernah
/// benar-benar tersinkron dengan server, mis. row lokal usang dari sebelum
/// reset/first-sync): dalam kasus itu data server WAJIB diterapkan, terlepas
/// dari `updated_at` lokal terlihat lebih baru.
pub fn apply_pulled_products(
    conn: &Connection,
    incoming: &[Product],
) -> AppResult<(i64, i64, Vec<SyncLogEntry>)> {
    let mut applied = 0i64;
    let mut skipped = 0i64;
    let mut log: Vec<SyncLogEntry> = Vec::with_capacity(incoming.len());
    for p in incoming {
        let local: Option<(String, bool)> = conn
            .query_row(
                "SELECT updated_at, ever_synced FROM products WHERE id = ?1",
                params![p.id],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)? != 0)),
            )
            .optional()?;

        let should_apply = match &local {
            None => true,
            // Bandingkan ISO-8601 (UTC) secara leksikografis = urutan kronologis.
            Some((local_updated, ever_synced)) => {
                !ever_synced || p.updated_at.as_str() > local_updated.as_str()
            }
        };

        if should_apply {
            conn.execute(
                "INSERT INTO products
                    (id, name, barcode, category, brand, unit, sell_price, cost_price,
                     default_discount, is_active, is_deleted, updated_at, dirty, ever_synced)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0,1)
                 ON CONFLICT(id) DO UPDATE SET
                    name=excluded.name, barcode=excluded.barcode, category=excluded.category,
                    brand=excluded.brand, unit=excluded.unit, sell_price=excluded.sell_price,
                    cost_price=excluded.cost_price, default_discount=excluded.default_discount,
                    is_active=excluded.is_active, is_deleted=excluded.is_deleted,
                    updated_at=excluded.updated_at, dirty=0, ever_synced=1",
                params![
                    p.id, p.name, p.barcode, p.category, p.brand, p.unit, p.sell_price,
                    p.cost_price, p.default_discount, p.is_active as i64, p.is_deleted as i64,
                    p.updated_at,
                ],
            )?;
            // Pastikan baris stok ada untuk produk baru.
            conn.execute(
                "INSERT OR IGNORE INTO stock (product_id, qty, updated_at) VALUES (?1, 0, ?2)",
                params![p.id, p.updated_at],
            )?;
            // Sama seperti upsert_product (input manual/Excel) — merek dari
            // produk hasil pull juga harus masuk tabel brands, bukan cuma
            // tersimpan sebagai teks bebas di products.brand.
            ensure_brand_exists(conn, p.brand.as_deref(), &p.updated_at)?;
            applied += 1;
            log.push(SyncLogEntry { id: p.id.clone(), name: p.name.clone(), action: "Diterapkan".into() });
        } else {
            skipped += 1;
            log.push(SyncLogEntry { id: p.id.clone(), name: p.name.clone(), action: "Dilewati".into() });
        }
    }
    Ok((applied, skipped, log))
}

// ---------- Pengguna / hak akses ----------

use crate::models::{DiscountPeriod, DiscountPeriodInput, StockMovement, StockMovementInput, User, UserInput};

fn map_user(row: &rusqlite::Row) -> rusqlite::Result<User> {
    let perms_json: String = row.get("permissions")?;
    let permissions: Vec<String> = serde_json::from_str(&perms_json).unwrap_or_default();
    Ok(User {
        id: row.get("id")?,
        username: row.get("username")?,
        name: row.get("name")?,
        role: row.get("role")?,
        permissions,
    })
}

pub fn login(conn: &Connection, username: &str, pin: &str) -> AppResult<Option<User>> {
    let res = conn
        .query_row(
            "SELECT id, username, name, role, COALESCE(permissions,'[]') AS permissions
             FROM users WHERE username = ?1 AND COALESCE(pin,'') = ?2",
            params![username, pin],
            map_user,
        )
        .optional()?;
    Ok(res)
}

pub fn get_user(conn: &Connection, id: &str) -> AppResult<Option<User>> {
    let res = conn
        .query_row(
            "SELECT id, username, name, role, COALESCE(permissions,'[]') AS permissions
             FROM users WHERE id = ?1",
            params![id],
            map_user,
        )
        .optional()?;
    Ok(res)
}

pub fn list_users(conn: &Connection) -> AppResult<Vec<User>> {
    let mut stmt = conn.prepare(
        "SELECT id, username, name, role, COALESCE(permissions,'[]') AS permissions
         FROM users ORDER BY username ASC",
    )?;
    let rows = stmt.query_map([], map_user)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn save_user(conn: &Connection, input: UserInput) -> AppResult<User> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let perms = serde_json::to_string(&input.permissions).unwrap_or_else(|_| "[]".into());
    let existed = conn
        .query_row("SELECT 1 FROM users WHERE id = ?1", params![id], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    let pin_provided = input.pin.as_deref().map(|p| !p.is_empty()).unwrap_or(false);

    if !existed {
        conn.execute(
            "INSERT INTO users (id, username, name, role, pin, permissions)
             VALUES (?1,?2,?3,?4,?5,?6)",
            params![id, input.username, input.name, input.role, input.pin, perms],
        )?;
    } else if pin_provided {
        conn.execute(
            "UPDATE users SET username=?2, name=?3, role=?4, pin=?5, permissions=?6 WHERE id=?1",
            params![id, input.username, input.name, input.role, input.pin, perms],
        )?;
    } else {
        conn.execute(
            "UPDATE users SET username=?2, name=?3, role=?4, permissions=?5 WHERE id=?1",
            params![id, input.username, input.name, input.role, perms],
        )?;
    }
    get_user(conn, &id)?.ok_or_else(|| AppError::Other("pengguna gagal disimpan".into()))
}

pub fn delete_user(conn: &Connection, id: &str) -> AppResult<()> {
    let total: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |r| r.get(0))?;
    if total <= 1 {
        return Err(AppError::Other("tidak bisa menghapus pengguna terakhir".into()));
    }
    conn.execute("DELETE FROM users WHERE id = ?1", params![id])?;
    Ok(())
}

// ---------- Pergerakan stok ----------

pub fn create_stock_movement(
    conn: &Connection,
    input: StockMovementInput,
) -> AppResult<StockMovement> {
    let now = resolve_created_at(&input.created_at);
    let qty = input.qty.abs();
    let stock_after = match input.kind.as_str() {
        "opname" => set_stock(conn, &input.product_id, qty)?,
        "in" => adjust_stock(conn, &input.product_id, qty)?,
        "out" => adjust_stock(conn, &input.product_id, -qty)?,
        other => return Err(AppError::Other(format!("jenis pergerakan tidak valid: {other}"))),
    };
    conn.execute(
        "INSERT INTO stock_movements
            (product_id, kind, qty, stock_after, note, user_id, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)",
        params![input.product_id, input.kind, qty, stock_after, input.note, input.user_id, now],
    )?;
    let id = conn.last_insert_rowid();
    let product_name: String = conn
        .query_row("SELECT name FROM products WHERE id = ?1", params![input.product_id], |r| {
            r.get(0)
        })
        .optional()?
        .unwrap_or_default();

    Ok(StockMovement {
        id,
        product_id: input.product_id,
        product_name,
        kind: input.kind,
        qty,
        note: input.note,
        user_id: input.user_id,
        created_at: now,
        stock_after,
    })
}

fn next_batch_no(conn: &Connection, kind: &str) -> AppResult<String> {
    let key = format!("batch_seq_{kind}");
    let seq: i64 = get_setting(conn, &key)?.and_then(|v| v.parse().ok()).unwrap_or(0) + 1;
    set_setting(conn, &key, &seq.to_string())?;
    let date = Utc::now().format("%Y%m%d");
    let prefix = if kind == "in" { "MSK" } else { "KLR" };
    Ok(format!("{prefix}-{date}-{seq:04}"))
}

/// Simpan satu batch Item Masuk/Keluar (banyak barang sekaligus, satu klik
/// "Simpan Semua Item") — mirip `create_sale`: satu baris header + N baris
/// stock_movements yang berbagi `batch_id`, dalam satu transaksi DB.
pub fn create_stock_movement_batch(
    conn: &mut Connection,
    input: crate::models::StockMovementBatchInput,
) -> AppResult<crate::models::StockMovementBatchDetail> {
    if input.items.is_empty() {
        return Err(AppError::Other("batch tidak boleh kosong".into()));
    }
    if input.kind != "in" && input.kind != "out" {
        return Err(AppError::Other(format!("jenis batch tidak valid: {}", input.kind)));
    }
    let now = resolve_created_at(&input.created_at);
    let id = Uuid::new_v4().to_string();
    let no = next_batch_no(conn, &input.kind)?;

    let db_tx = conn.transaction()?;
    db_tx.execute(
        "INSERT INTO stock_movement_batches (id, no, kind, note, user_id, created_at)
         VALUES (?1,?2,?3,?4,?5,?6)",
        params![id, no, input.kind, input.note, input.user_id, now],
    )?;

    let mut items_out = Vec::with_capacity(input.items.len());
    for it in &input.items {
        let qty = it.qty.abs();
        let stock_after = if input.kind == "in" {
            adjust_stock(&db_tx, &it.product_id, qty)?
        } else {
            adjust_stock(&db_tx, &it.product_id, -qty)?
        };
        db_tx.execute(
            "INSERT INTO stock_movements
                (product_id, kind, qty, stock_after, note, user_id, created_at, batch_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![it.product_id, input.kind, qty, stock_after, it.note, input.user_id, now, id],
        )?;
        let product_name: String = db_tx
            .query_row("SELECT name FROM products WHERE id = ?1", params![it.product_id], |r| r.get(0))
            .optional()?
            .unwrap_or_default();
        items_out.push(crate::models::StockMovementBatchItem {
            product_id: it.product_id.clone(),
            product_name,
            qty,
            note: it.note.clone(),
        });
    }
    db_tx.commit()?;

    Ok(crate::models::StockMovementBatchDetail {
        id,
        no,
        kind: input.kind,
        note: input.note,
        user_id: input.user_id,
        created_at: now,
        items: items_out,
    })
}

fn stock_movement_batch_where(
    kind: &Option<String>,
    from: &Option<String>,
    to: &Option<String>,
) -> (String, Vec<Box<dyn rusqlite::ToSql>>) {
    let mut sql = String::from(" WHERE 1=1");
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(k) = kind {
        sql.push_str(" AND b.kind = ?");
        sql.push_str(&(args.len() + 1).to_string());
        args.push(Box::new(k.clone()));
    }
    if let Some(f) = from {
        sql.push_str(" AND date(b.created_at) >= date(?");
        sql.push_str(&(args.len() + 1).to_string());
        sql.push(')');
        args.push(Box::new(f.clone()));
    }
    if let Some(t) = to {
        sql.push_str(" AND date(b.created_at) <= date(?");
        sql.push_str(&(args.len() + 1).to_string());
        sql.push(')');
        args.push(Box::new(t.clone()));
    }
    (sql, args)
}

/// Cek apakah sudah ada baris stock_movements lokal yang note-nya mengandung
/// `needle` — dipakai bridge Pull (lihat commands.rs:bridge_confirm_pull)
/// sebagai guard idempotensi PER BARIS menu Barang (di-tag `[pull-row:<id>]`
/// per item), biar satu baris dari app mobile gak ketulis dobel ke stok
/// kalau konfirmasi ke server sempat gagal di percobaan sebelumnya (bukan
/// sebagai retry, tapi biar gagal jelas & aman).
pub fn stock_movement_note_contains(conn: &Connection, needle: &str) -> AppResult<bool> {
    let pattern = format!("%{needle}%");
    let found: bool = conn
        .query_row(
            "SELECT 1 FROM stock_movements WHERE note LIKE ?1 LIMIT 1",
            params![pattern],
            |_| Ok(true),
        )
        .optional()?
        .unwrap_or(false);
    Ok(found)
}

pub fn list_stock_movement_batches(
    conn: &Connection,
    kind: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: i64,
    offset: i64,
) -> AppResult<crate::models::StockMovementBatchPage> {
    let (count_where, count_args) = stock_movement_batch_where(&kind, &from, &to);
    let total: i64 = {
        let count_sql = format!("SELECT COUNT(*) FROM stock_movement_batches b{count_where}");
        let params_ref: Vec<&dyn rusqlite::ToSql> = count_args.iter().map(|b| b.as_ref()).collect();
        conn.query_row(&count_sql, params_ref.as_slice(), |r| r.get(0))?
    };

    let (where_sql, mut args) = stock_movement_batch_where(&kind, &from, &to);
    let sql = format!(
        "SELECT b.id, b.no, b.kind, b.note, b.user_id, b.created_at,
                COUNT(m.id) AS item_count, COALESCE(SUM(m.qty), 0) AS total_qty
         FROM stock_movement_batches b
         LEFT JOIN stock_movements m ON m.batch_id = b.id{where_sql}
         GROUP BY b.id ORDER BY b.created_at DESC LIMIT ?{} OFFSET ?{}",
        args.len() + 1,
        args.len() + 2,
    );
    args.push(Box::new(limit));
    args.push(Box::new(offset));

    let mut stmt = conn.prepare(&sql)?;
    let params_ref: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let items = stmt
        .query_map(params_ref.as_slice(), |r| {
            Ok(crate::models::StockMovementBatch {
                id: r.get(0)?,
                no: r.get(1)?,
                kind: r.get(2)?,
                note: r.get(3)?,
                user_id: r.get(4)?,
                created_at: r.get(5)?,
                item_count: r.get(6)?,
                total_qty: r.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(crate::models::StockMovementBatchPage { items, total })
}

pub fn get_stock_movement_batch(
    conn: &Connection,
    id: &str,
) -> AppResult<Option<crate::models::StockMovementBatchDetail>> {
    let header = conn
        .query_row(
            "SELECT id, no, kind, note, user_id, created_at FROM stock_movement_batches WHERE id = ?1",
            params![id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, Option<String>>(4)?,
                    r.get::<_, String>(5)?,
                ))
            },
        )
        .optional()?;
    let Some((id, no, kind, note, user_id, created_at)) = header else {
        return Ok(None);
    };

    let mut stmt = conn.prepare(
        "SELECT m.product_id, COALESCE(p.name, '') AS product_name, m.qty, m.note
         FROM stock_movements m LEFT JOIN products p ON p.id = m.product_id
         WHERE m.batch_id = ?1 ORDER BY m.id ASC",
    )?;
    let items = stmt
        .query_map(params![id], |r| {
            Ok(crate::models::StockMovementBatchItem {
                product_id: r.get(0)?,
                product_name: r.get(1)?,
                qty: r.get(2)?,
                note: r.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(Some(crate::models::StockMovementBatchDetail {
        id,
        no,
        kind,
        note,
        user_id,
        created_at,
        items,
    }))
}

/// Edit batch: kembalikan efek stok item lama, hapus baris lama, validasi &
/// catat ulang item baru — id/no/kind/created_at dipertahankan. Mirip `update_transaction`.
pub fn update_stock_movement_batch(
    conn: &mut Connection,
    id: &str,
    items: Vec<crate::models::StockMovementBatchItemInput>,
    note: Option<String>,
) -> AppResult<crate::models::StockMovementBatchDetail> {
    if items.is_empty() {
        return Err(AppError::Other("batch tidak boleh kosong".into()));
    }

    let (no, kind, user_id, created_at): (String, String, Option<String>, String) = conn
        .query_row(
            "SELECT no, kind, user_id, created_at FROM stock_movement_batches WHERE id = ?1",
            params![id],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
        )
        .optional()?
        .ok_or_else(|| AppError::Other("Batch tidak ditemukan.".into()))?;

    let db_tx = conn.transaction()?;

    // 1. Kembalikan efek stok item LAMA (kebalikan dari kind).
    let old_items: Vec<(String, f64)> = {
        let mut stmt =
            db_tx.prepare("SELECT product_id, qty FROM stock_movements WHERE batch_id = ?1")?;
        let rows = stmt
            .query_map(params![id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    for (pid, qty) in &old_items {
        let reverse = if kind == "in" { -qty } else { *qty };
        adjust_stock(&db_tx, pid, reverse)?;
    }
    db_tx.execute("DELETE FROM stock_movements WHERE batch_id = ?1", params![id])?;

    // 2. Validasi stok cukup untuk item BARU (khusus "out", setelah restore di atas).
    if kind == "out" {
        for it in &items {
            let current: f64 = db_tx
                .query_row(
                    "SELECT COALESCE(qty,0) FROM stock WHERE product_id = ?1",
                    params![it.product_id],
                    |r| r.get(0),
                )
                .unwrap_or(0.0);
            if current < it.qty.abs() {
                return Err(AppError::Other(format!(
                    "Stok tidak cukup untuk salah satu barang (tersedia {:.0}, dibutuhkan {:.0}).",
                    current,
                    it.qty.abs()
                )));
            }
        }
    }

    // 3. Catat ulang item baru.
    let mut items_out = Vec::with_capacity(items.len());
    for it in &items {
        let qty = it.qty.abs();
        let stock_after = if kind == "in" {
            adjust_stock(&db_tx, &it.product_id, qty)?
        } else {
            adjust_stock(&db_tx, &it.product_id, -qty)?
        };
        db_tx.execute(
            "INSERT INTO stock_movements
                (product_id, kind, qty, stock_after, note, user_id, created_at, batch_id)
             VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![it.product_id, kind, qty, stock_after, it.note, user_id, created_at, id],
        )?;
        let product_name: String = db_tx
            .query_row("SELECT name FROM products WHERE id = ?1", params![it.product_id], |r| r.get(0))
            .optional()?
            .unwrap_or_default();
        items_out.push(crate::models::StockMovementBatchItem {
            product_id: it.product_id.clone(),
            product_name,
            qty,
            note: it.note.clone(),
        });
    }

    db_tx.execute(
        "UPDATE stock_movement_batches SET note = ?2 WHERE id = ?1",
        params![id, note],
    )?;
    db_tx.commit()?;

    Ok(crate::models::StockMovementBatchDetail {
        id: id.to_string(),
        no,
        kind,
        note,
        user_id,
        created_at,
        items: items_out,
    })
}

/// Hapus batch: kembalikan efek stok semua item, lalu hapus baris movement & header.
pub fn delete_stock_movement_batch(conn: &mut Connection, id: &str) -> AppResult<()> {
    let kind: String = conn
        .query_row(
            "SELECT kind FROM stock_movement_batches WHERE id = ?1",
            params![id],
            |r| r.get(0),
        )
        .optional()?
        .ok_or_else(|| AppError::Other("Batch tidak ditemukan.".into()))?;

    let db_tx = conn.transaction()?;
    let items: Vec<(String, f64)> = {
        let mut stmt =
            db_tx.prepare("SELECT product_id, qty FROM stock_movements WHERE batch_id = ?1")?;
        let rows = stmt
            .query_map(params![id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    for (pid, qty) in &items {
        let reverse = if kind == "in" { -qty } else { *qty };
        adjust_stock(&db_tx, pid, reverse)?;
    }
    db_tx.execute("DELETE FROM stock_movements WHERE batch_id = ?1", params![id])?;
    db_tx.execute("DELETE FROM stock_movement_batches WHERE id = ?1", params![id])?;
    db_tx.commit()?;
    Ok(())
}

pub fn list_stock_movements(
    conn: &Connection,
    kind: Option<String>,
    from: Option<String>,
    to: Option<String>,
    limit: i64,
) -> AppResult<Vec<StockMovement>> {
    let mut sql = String::from(
        "SELECT m.id, m.product_id, COALESCE(p.name,'') AS product_name, m.kind, m.qty,
                m.stock_after, m.note, m.user_id, m.created_at
         FROM stock_movements m LEFT JOIN products p ON p.id = m.product_id WHERE 1=1",
    );
    let mut args: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
    if let Some(k) = &kind {
        sql.push_str(" AND m.kind = ?");
        sql.push_str(&(args.len() + 1).to_string());
        args.push(Box::new(k.clone()));
    }
    if let Some(f) = &from {
        sql.push_str(" AND m.created_at >= ?");
        sql.push_str(&(args.len() + 1).to_string());
        args.push(Box::new(f.clone()));
    }
    if let Some(t) = &to {
        sql.push_str(" AND m.created_at <= ?");
        sql.push_str(&(args.len() + 1).to_string());
        args.push(Box::new(t.clone()));
    }
    sql.push_str(" ORDER BY m.created_at DESC, m.id DESC LIMIT ?");
    sql.push_str(&(args.len() + 1).to_string());
    args.push(Box::new(limit));

    let mut stmt = conn.prepare(&sql)?;
    let params_ref: Vec<&dyn rusqlite::ToSql> = args.iter().map(|b| b.as_ref()).collect();
    let rows = stmt
        .query_map(params_ref.as_slice(), |r| {
            Ok(StockMovement {
                id: r.get("id")?,
                product_id: r.get("product_id")?,
                product_name: r.get("product_name")?,
                kind: r.get("kind")?,
                qty: r.get("qty")?,
                stock_after: r.get("stock_after")?,
                note: r.get("note")?,
                user_id: r.get("user_id")?,
                created_at: r.get("created_at")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ---------- Diskon periodik ----------

fn map_discount(row: &rusqlite::Row) -> rusqlite::Result<DiscountPeriod> {
    Ok(DiscountPeriod {
        id: row.get("id")?,
        code: row.get::<_, Option<String>>("code")?.unwrap_or_default(),
        scope: row.get("scope")?,
        target: row.get("target")?,
        target_label: row.get("target_label")?,
        discount_type: row.get("discount_type")?,
        value: row.get("value")?,
        days: row.get("days")?,
        is_active: row.get::<_, i64>("is_active")? != 0,
        priority: row.get::<_, Option<i64>>("priority")?.unwrap_or(1),
        updated_at: row.get("updated_at")?,
    })
}

pub fn list_discounts(conn: &Connection) -> AppResult<Vec<DiscountPeriod>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM discount_periods ORDER BY code ASC, priority ASC, updated_at DESC",
    )?;
    let rows = stmt.query_map([], map_discount)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn save_discount(conn: &Connection, input: DiscountPeriodInput) -> AppResult<DiscountPeriod> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO discount_periods
            (id, code, scope, target, target_label, discount_type, value, days, is_active, priority, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
         ON CONFLICT(id) DO UPDATE SET
            code=excluded.code, scope=excluded.scope, target=excluded.target,
            target_label=excluded.target_label, discount_type=excluded.discount_type,
            value=excluded.value, days=excluded.days, is_active=excluded.is_active,
            priority=excluded.priority, updated_at=excluded.updated_at",
        params![
            id, input.code, input.scope, input.target, input.target_label, input.discount_type,
            input.value, input.days, input.is_active as i64, input.priority, now
        ],
    )?;
    conn.query_row("SELECT * FROM discount_periods WHERE id = ?1", params![id], map_discount)
        .map_err(AppError::from)
}

pub fn delete_discount(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM discount_periods WHERE id = ?1", params![id])?;
    Ok(())
}

// ---------- Hapus transaksi (kembalikan stok) ----------

pub fn delete_transaction(conn: &mut Connection, id: &str) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    let db_tx = conn.transaction()?;
    {
        let mut stmt =
            db_tx.prepare("SELECT product_id, qty FROM transaction_items WHERE transaction_id = ?1")?;
        let items = stmt
            .query_map(params![id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        for (pid, qty) in items {
            db_tx.execute(
                "INSERT INTO stock (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
                 ON CONFLICT(product_id) DO UPDATE SET qty = qty + ?2, updated_at = ?3",
                params![pid, qty, now],
            )?;
        }
    }
    db_tx.execute("DELETE FROM transaction_items WHERE transaction_id = ?1", params![id])?;
    db_tx.execute("DELETE FROM transactions WHERE id = ?1", params![id])?;
    db_tx.commit()?;
    Ok(())
}

/// Edit penuh transaksi tersimpan: item/qty/harga/diskon/metode/pelanggan bisa
/// diubah. Stok & totals dihitung ulang; id/invoice_no/cashier_id/shift_id/
/// created_at dipertahankan dari transaksi lama.
pub fn update_transaction(
    conn: &mut Connection,
    id: &str,
    input: SaleInput,
) -> AppResult<TransactionDetail> {
    if input.items.is_empty() {
        return Err(AppError::Other("transaksi tidak boleh kosong".into()));
    }
    let now = Utc::now().to_rfc3339();

    let existing = conn
        .query_row("SELECT * FROM transactions WHERE id = ?1", params![id], map_transaction)
        .optional()?
        .ok_or_else(|| AppError::Other("Transaksi tidak ditemukan.".into()))?;

    let mut subtotal = 0.0;
    let mut total_discount = 0.0;
    let mut items: Vec<TransactionItem> = Vec::with_capacity(input.items.len());
    for it in &input.items {
        let line_gross = it.price * it.qty;
        let line_total = (line_gross - it.discount).max(0.0);
        subtotal += line_gross;
        total_discount += it.discount;
        items.push(TransactionItem {
            product_id: it.product_id.clone(),
            name: it.name.clone(),
            price: it.price,
            qty: it.qty,
            discount: it.discount,
            line_total,
        });
    }
    let total = (subtotal - total_discount).max(0.0);
    if input.paid < total {
        return Err(AppError::Other("pembayaran kurang dari total".into()));
    }
    let change = input.paid - total;

    let db_tx = conn.transaction()?;

    // 1. Kembalikan stok item LAMA dulu supaya cek kecukupan stok di bawah
    //    memakai angka yang benar (termasuk kalau item barunya sama persis).
    let old_items: Vec<(String, f64)> = {
        let mut stmt = db_tx
            .prepare("SELECT product_id, qty FROM transaction_items WHERE transaction_id = ?1")?;
        let rows = stmt
            .query_map(params![id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;
        rows
    };
    for (pid, qty) in &old_items {
        db_tx.execute(
            "INSERT INTO stock (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(product_id) DO UPDATE SET qty = qty + ?2, updated_at = ?3",
            params![pid, qty, now],
        )?;
    }

    // 2. Hapus stock_movements 'sale' lama milik invoice ini (dibuat ulang di
    //    langkah 4) — mencegah riwayat hantu dari baris item yang sudah tak
    //    relevan lagi setelah diedit.
    db_tx.execute(
        "DELETE FROM stock_movements WHERE kind = 'sale' AND note = ?1",
        params![format!("Invoice {}", existing.invoice_no)],
    )?;

    // 3. Cek kecukupan stok untuk item BARU (setelah restore di atas).
    for it in &input.items {
        let current: f64 = db_tx
            .query_row(
                "SELECT COALESCE(qty,0) FROM stock WHERE product_id = ?1",
                params![it.product_id],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        if current < it.qty {
            return Err(AppError::Other(format!(
                "Stok \"{}\" tidak cukup (tersedia {:.0}, dibutuhkan {:.0}).",
                it.name, current, it.qty
            )));
        }
    }

    // 4. Ganti transaction_items, kurangi stok, catat stock_movements baru.
    db_tx.execute("DELETE FROM transaction_items WHERE transaction_id = ?1", params![id])?;
    for it in &items {
        db_tx.execute(
            "INSERT INTO transaction_items
                (transaction_id, product_id, name, price, qty, discount, line_total)
             VALUES (?1,?2,?3,?4,?5,?6,?7)",
            params![id, it.product_id, it.name, it.price, it.qty, it.discount, it.line_total],
        )?;
        db_tx.execute(
            "INSERT INTO stock (product_id, qty, updated_at) VALUES (?1, -?2, ?3)
             ON CONFLICT(product_id) DO UPDATE SET qty = qty - ?2, updated_at = ?3",
            params![it.product_id, it.qty, now],
        )?;
        let after: f64 = db_tx.query_row(
            "SELECT qty FROM stock WHERE product_id = ?1",
            params![it.product_id],
            |r| r.get(0),
        )?;
        db_tx.execute(
            "INSERT INTO stock_movements
                (product_id, kind, qty, stock_after, note, user_id, created_at)
             VALUES (?1, 'sale', ?2, ?3, ?4, ?5, ?6)",
            params![
                it.product_id, it.qty, after,
                format!("Invoice {}", existing.invoice_no), existing.cashier_id, now
            ],
        )?;
    }

    // 5. Update header transaksi (id/invoice_no/cashier_id/shift_id/created_at tetap).
    db_tx.execute(
        "UPDATE transactions SET subtotal=?2, discount=?3, total=?4, paid=?5, change=?6,
            payment_method=?7, customer_id=?8, paid_cash=?9, paid_qris=?10 WHERE id=?1",
        params![
            id, subtotal, total_discount, total, input.paid, change,
            input.payment_method, input.customer_id, input.paid_cash, input.paid_qris
        ],
    )?;
    db_tx.commit()?;

    Ok(TransactionDetail {
        header: Transaction {
            id: id.to_string(),
            invoice_no: existing.invoice_no,
            cashier_id: existing.cashier_id,
            subtotal,
            discount: total_discount,
            total,
            paid: input.paid,
            change,
            payment_method: input.payment_method,
            created_at: existing.created_at,
            customer_id: input.customer_id,
            shift_id: existing.shift_id,
            paid_cash: input.paid_cash,
            paid_qris: input.paid_qris,
        },
        items,
    })
}

/// Hapus satu pergerakan stok (Item Masuk/Keluar) sambil mengoreksi stok.
pub fn delete_stock_movement(conn: &mut Connection, id: i64) -> AppResult<()> {
    let row = conn
        .query_row(
            "SELECT product_id, kind, qty FROM stock_movements WHERE id = ?1",
            params![id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?, r.get::<_, f64>(2)?)),
        )
        .optional()?;
    let Some((pid, kind, qty)) = row else { return Ok(()) };

    // Hanya pergerakan in/out yang reversibel; opname/sale tidak.
    let delta = match kind.as_str() {
        "in" => -qty,
        "out" => qty,
        other => {
            return Err(AppError::Other(format!(
                "pergerakan '{other}' tidak bisa dihapus dari sini"
            )))
        }
    };

    let now = Utc::now().to_rfc3339();
    let db_tx = conn.transaction()?;
    db_tx.execute(
        "INSERT INTO stock (product_id, qty, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(product_id) DO UPDATE SET qty = qty + ?2, updated_at = ?3",
        params![pid, delta, now],
    )?;
    db_tx.execute("DELETE FROM stock_movements WHERE id = ?1", params![id])?;
    db_tx.commit()?;
    Ok(())
}

// ---------- Merek (brand) ----------

use crate::models::{Brand, BrandInput};

pub fn list_brands(conn: &Connection) -> AppResult<Vec<Brand>> {
    let mut stmt = conn.prepare("SELECT id, name, updated_at FROM brands ORDER BY name ASC")?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Brand { id: r.get(0)?, name: r.get(1)?, updated_at: r.get(2)? })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn save_brand(conn: &Connection, input: BrandInput) -> AppResult<Brand> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Other("nama merek kosong".into()));
    }
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO brands (id, name, updated_at) VALUES (?1, ?2, ?3)
         ON CONFLICT(id) DO UPDATE SET name = excluded.name, updated_at = excluded.updated_at",
        params![id, name, now],
    )?;
    let real_id: String =
        conn.query_row("SELECT id FROM brands WHERE name = ?1", params![name], |r| r.get(0))?;
    Ok(Brand { id: real_id, name, updated_at: now })
}

pub fn delete_brand(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM brands WHERE id = ?1", params![id])?;
    Ok(())
}

// ---------- Pelanggan ----------

use crate::models::{Customer, CustomerInput};

fn map_customer(r: &rusqlite::Row) -> rusqlite::Result<Customer> {
    Ok(Customer {
        id: r.get("id")?,
        name: r.get("name")?,
        phone: r.get("phone")?,
        email: r.get("email")?,
        address: r.get("address")?,
        note: r.get("note")?,
        is_active: r.get::<_, i64>("is_active")? != 0,
        updated_at: r.get("updated_at")?,
    })
}

pub fn list_customers(
    conn: &Connection,
    search: Option<String>,
    include_inactive: bool,
) -> AppResult<Vec<Customer>> {
    let like = format!("%{}%", search.unwrap_or_default());
    let active_clause = if include_inactive { "1=1" } else { "is_active = 1" };
    let sql = format!(
        "SELECT * FROM customers
         WHERE {active_clause}
           AND (name LIKE ?1 OR COALESCE(phone,'') LIKE ?1)
         ORDER BY name ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![like], map_customer)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn upsert_customer(conn: &Connection, input: CustomerInput) -> AppResult<Customer> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::Other("Nama pelanggan wajib diisi.".into()));
    }
    let now = Utc::now().to_rfc3339();
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    conn.execute(
        "INSERT INTO customers (id, name, phone, email, address, note, is_active, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8)
         ON CONFLICT(id) DO UPDATE SET
            name=excluded.name, phone=excluded.phone, email=excluded.email,
            address=excluded.address, note=excluded.note, is_active=excluded.is_active,
            updated_at=excluded.updated_at",
        params![id, name, input.phone, input.email, input.address, input.note, input.is_active as i64, now],
    )?;
    conn.query_row("SELECT * FROM customers WHERE id = ?1", params![id], map_customer)
        .map_err(AppError::from)
}

pub fn delete_customer(conn: &Connection, id: &str) -> AppResult<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE customers SET is_active = 0, updated_at = ?2 WHERE id = ?1",
        params![id, now],
    )?;
    Ok(())
}

// ---------- Pengeluaran (Kas Keluar) ----------

use crate::models::{Expense, ExpenseInput};

fn map_expense(r: &rusqlite::Row) -> rusqlite::Result<Expense> {
    Ok(Expense {
        id: r.get("id")?,
        date: r.get("date")?,
        category: r.get("category")?,
        amount: r.get("amount")?,
        note: r.get("note")?,
        user_id: r.get("user_id")?,
        created_at: r.get("created_at")?,
    })
}

pub fn list_expenses(conn: &Connection, from: Option<String>, to: Option<String>) -> AppResult<Vec<Expense>> {
    let sql = "SELECT * FROM expenses
               WHERE (?1 IS NULL OR date >= ?1) AND (?2 IS NULL OR date <= ?2)
               ORDER BY date DESC, created_at DESC";
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params![from, to], map_expense)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn save_expense(conn: &Connection, input: ExpenseInput) -> AppResult<Expense> {
    if input.category.trim().is_empty() {
        return Err(AppError::Other("Kategori pengeluaran wajib diisi.".into()));
    }
    if input.amount <= 0.0 {
        return Err(AppError::Other("Nominal pengeluaran harus lebih dari 0.".into()));
    }
    let now = Utc::now().to_rfc3339();
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    conn.execute(
        "INSERT INTO expenses (id, date, category, amount, note, user_id, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7)
         ON CONFLICT(id) DO UPDATE SET
            date=excluded.date, category=excluded.category, amount=excluded.amount,
            note=excluded.note, user_id=excluded.user_id",
        params![id, input.date, input.category.trim(), input.amount, input.note, input.user_id, now],
    )?;
    conn.query_row("SELECT * FROM expenses WHERE id = ?1", params![id], map_expense)
        .map_err(AppError::from)
}

pub fn delete_expense(conn: &Connection, id: &str) -> AppResult<()> {
    conn.execute("DELETE FROM expenses WHERE id = ?1", params![id])?;
    Ok(())
}

// ---------- Shift kasir (buka/tutup, rekonsiliasi) ----------

use crate::models::{CloseShiftInput, OpenShiftInput, Shift, ShiftSummary};

fn map_shift(r: &rusqlite::Row) -> rusqlite::Result<Shift> {
    Ok(Shift {
        id: r.get("id")?,
        user_id: r.get("user_id")?,
        user_name: r.get("user_name")?,
        opening_cash: r.get("opening_cash")?,
        closing_cash: r.get("closing_cash")?,
        expected_cash: r.get("expected_cash")?,
        difference: r.get("difference")?,
        note: r.get("note")?,
        opened_at: r.get("opened_at")?,
        closed_at: r.get("closed_at")?,
    })
}

/// Shift yang sedang berjalan (belum ditutup), kalau ada.
pub fn get_active_shift(conn: &Connection) -> AppResult<Option<Shift>> {
    conn.query_row(
        "SELECT * FROM shifts WHERE closed_at IS NULL ORDER BY opened_at DESC LIMIT 1",
        [],
        map_shift,
    )
    .optional()
    .map_err(AppError::from)
}

pub fn open_shift(conn: &Connection, input: OpenShiftInput) -> AppResult<Shift> {
    if get_active_shift(conn)?.is_some() {
        return Err(AppError::Other("Sudah ada shift yang sedang berjalan.".into()));
    }
    let id = Uuid::new_v4().to_string();
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO shifts (id, user_id, user_name, opening_cash, opened_at)
         VALUES (?1,?2,?3,?4,?5)",
        params![id, input.user_id, input.user_name, input.opening_cash, now],
    )?;
    conn.query_row("SELECT * FROM shifts WHERE id = ?1", params![id], map_shift)
        .map_err(AppError::from)
}

/// Ringkasan penjualan selama shift (dipakai untuk hitung `expected_cash`).
/// Untuk metode "Kombinasi": bagian tunai yang masuk laci = paid_cash dikurangi
/// kembalian (asumsi kembalian selalu diberikan tunai dari laci, praktik
/// standar), dibatasi minimum 0. Sisanya (total - bagian tunai) dihitung
/// sebagai non-tunai, supaya cash_sales + non_cash_sales tetap = total penjualan.
pub fn shift_summary(conn: &Connection, shift_id: &str) -> AppResult<ShiftSummary> {
    conn.query_row(
        "SELECT
            COUNT(*),
            COALESCE(SUM(total), 0),
            COALESCE(SUM(CASE
                WHEN payment_method = 'Tunai' THEN total
                WHEN payment_method = 'Kombinasi' THEN MAX(COALESCE(paid_cash,0) - (paid - total), 0)
                ELSE 0 END), 0),
            COALESCE(SUM(CASE
                WHEN payment_method = 'Tunai' THEN 0
                WHEN payment_method = 'Kombinasi' THEN total - MAX(COALESCE(paid_cash,0) - (paid - total), 0)
                ELSE total END), 0)
         FROM transactions WHERE shift_id = ?1",
        params![shift_id],
        |r| {
            Ok(ShiftSummary {
                transaction_count: r.get(0)?,
                total_sales: r.get(1)?,
                cash_sales: r.get(2)?,
                non_cash_sales: r.get(3)?,
            })
        },
    )
    .map_err(AppError::from)
}

pub fn close_shift(conn: &Connection, input: CloseShiftInput) -> AppResult<Shift> {
    let shift = conn
        .query_row("SELECT * FROM shifts WHERE id = ?1", params![input.id], map_shift)
        .optional()?
        .ok_or_else(|| AppError::Other("Shift tidak ditemukan.".into()))?;
    if shift.closed_at.is_some() {
        return Err(AppError::Other("Shift ini sudah ditutup.".into()));
    }
    let summary = shift_summary(conn, &input.id)?;
    let expected_cash = shift.opening_cash + summary.cash_sales;
    let difference = input.closing_cash - expected_cash;
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE shifts SET closing_cash = ?2, expected_cash = ?3, difference = ?4,
            note = ?5, closed_at = ?6 WHERE id = ?1",
        params![input.id, input.closing_cash, expected_cash, difference, input.note, now],
    )?;
    conn.query_row("SELECT * FROM shifts WHERE id = ?1", params![input.id], map_shift)
        .map_err(AppError::from)
}

pub fn list_shifts(conn: &Connection, limit: i64) -> AppResult<Vec<Shift>> {
    let mut stmt = conn.prepare("SELECT * FROM shifts ORDER BY opened_at DESC LIMIT ?1")?;
    let rows = stmt.query_map(params![limit], map_shift)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Dipanggil sekali saat startup (sebelum window utama dibuka). Kalau sesi
/// sebelumnya TIDAK sempat menutup dengan bersih (crash / mati listrik / force
/// close via Task Manager — ditandai lewat setting `app_running` yang masih
/// "1" karena tidak pernah di-clear), dan ada shift yang masih terbuka, shift
/// itu otomatis ditutup dengan uang fisik = 0 supaya tidak mengganjal toko
/// besoknya. Penanda `app_running` di-set "1" lagi setelahnya untuk sesi ini,
/// dan di-clear balik ke "0" saat window utama benar-benar ditutup (lihat
/// `WindowEvent::Destroyed` di lib.rs) — TIDAK bisa mengandalkan cleanup di
/// titik lain karena kalau proses dibunuh paksa, tidak ada kode yang sempat jalan.
pub fn recover_stale_shift(conn: &Connection) -> AppResult<()> {
    let was_running = get_setting(conn, "app_running")?.as_deref() == Some("1");
    if was_running {
        if let Some(shift) = get_active_shift(conn)? {
            close_shift(
                conn,
                CloseShiftInput {
                    id: shift.id,
                    closing_cash: 0.0,
                    note: Some(
                        "Ditutup otomatis — aplikasi sebelumnya berhenti tidak wajar (force close / mati listrik).".into(),
                    ),
                },
            )?;
        }
    }
    set_setting(conn, "app_running", "1")
}

// ---------- Laporan per barang / per merek ----------

use crate::models::{BrandSalesRow, DailySalesRow, ProductSalesRow, SalesItemDetailRow};

/// Rekap penjualan per barang dalam rentang tanggal, opsional difilter ke
/// sekumpulan merek (kombinasi beberapa merek sekaligus).
pub fn product_sales_report(
    conn: &Connection,
    from: &str,
    to: &str,
    brands: &[String],
) -> AppResult<Vec<ProductSalesRow>> {
    let mut sql = String::from(
        "SELECT ti.product_id, ti.name, p.brand,
                SUM(ti.qty) AS qty,
                SUM(ti.price * ti.qty) AS gross,
                SUM(ti.discount) AS discount,
                SUM(ti.line_total) AS net,
                SUM(COALESCE(p.cost_price, 0) * ti.qty) AS cogs
         FROM transaction_items ti
         JOIN transactions t ON t.id = ti.transaction_id
         LEFT JOIN products p ON p.id = ti.product_id
         WHERE date(t.created_at) >= date(?1) AND date(t.created_at) <= date(?2)",
    );
    if !brands.is_empty() {
        let placeholders = brands.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        sql.push_str(&format!(" AND p.brand IN ({placeholders})"));
    }
    sql.push_str(" GROUP BY ti.product_id, ti.name, p.brand ORDER BY net DESC");

    let mut stmt = conn.prepare(&sql)?;
    let mut param_values: Vec<&dyn rusqlite::ToSql> = vec![&from, &to];
    for b in brands {
        param_values.push(b);
    }
    let rows = stmt
        .query_map(param_values.as_slice(), |r| {
            Ok(ProductSalesRow {
                product_id: r.get(0)?,
                name: r.get(1)?,
                brand: r.get(2)?,
                qty: r.get(3)?,
                gross: r.get(4)?,
                discount: r.get(5)?,
                net: r.get(6)?,
                cogs: r.get(7)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Rekap penjualan per merek dalam rentang tanggal, opsional difilter ke
/// sekumpulan merek.
pub fn brand_sales_report(
    conn: &Connection,
    from: &str,
    to: &str,
    brands: &[String],
) -> AppResult<Vec<BrandSalesRow>> {
    let mut sql = String::from(
        "SELECT COALESCE(p.brand, '(Tanpa Merek)') AS brand,
                SUM(ti.qty) AS qty,
                SUM(ti.price * ti.qty) AS gross,
                SUM(ti.discount) AS discount,
                SUM(ti.line_total) AS net
         FROM transaction_items ti
         JOIN transactions t ON t.id = ti.transaction_id
         LEFT JOIN products p ON p.id = ti.product_id
         WHERE date(t.created_at) >= date(?1) AND date(t.created_at) <= date(?2)",
    );
    if !brands.is_empty() {
        let placeholders = brands.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        sql.push_str(&format!(" AND p.brand IN ({placeholders})"));
    }
    sql.push_str(" GROUP BY brand ORDER BY net DESC");

    let mut stmt = conn.prepare(&sql)?;
    let mut param_values: Vec<&dyn rusqlite::ToSql> = vec![&from, &to];
    for b in brands {
        param_values.push(b);
    }
    let rows = stmt
        .query_map(param_values.as_slice(), |r| {
            Ok(BrandSalesRow {
                brand: r.get(0)?,
                qty: r.get(1)?,
                gross: r.get(2)?,
                discount: r.get(3)?,
                net: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Laporan Item Detail: satu baris per item terjual dalam rentang tanggal,
/// opsional difilter merek.
pub fn sales_item_detail_report(
    conn: &Connection,
    from: &str,
    to: &str,
    brands: &[String],
) -> AppResult<Vec<SalesItemDetailRow>> {
    let mut sql = String::from(
        "SELECT t.invoice_no, t.created_at, t.cashier_id, ti.product_id, ti.name, p.barcode, p.brand,
                ti.qty, ti.price, ti.discount, ti.line_total
         FROM transaction_items ti
         JOIN transactions t ON t.id = ti.transaction_id
         LEFT JOIN products p ON p.id = ti.product_id
         WHERE date(t.created_at) >= date(?1) AND date(t.created_at) <= date(?2)",
    );
    if !brands.is_empty() {
        let placeholders = brands.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        sql.push_str(&format!(" AND p.brand IN ({placeholders})"));
    }
    sql.push_str(" ORDER BY t.created_at DESC, ti.id ASC");

    let mut stmt = conn.prepare(&sql)?;
    let mut param_values: Vec<&dyn rusqlite::ToSql> = vec![&from, &to];
    for b in brands {
        param_values.push(b);
    }
    let rows = stmt
        .query_map(param_values.as_slice(), |r| {
            Ok(SalesItemDetailRow {
                invoice_no: r.get(0)?,
                created_at: r.get(1)?,
                cashier_id: r.get(2)?,
                product_id: r.get(3)?,
                name: r.get(4)?,
                barcode: r.get(5)?,
                brand: r.get(6)?,
                qty: r.get(7)?,
                price: r.get(8)?,
                discount: r.get(9)?,
                net: r.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

/// Laporan Item Per Hari: agregasi qty/nilai per tanggal, opsional difilter merek.
pub fn daily_sales_report(
    conn: &Connection,
    from: &str,
    to: &str,
    brands: &[String],
) -> AppResult<Vec<DailySalesRow>> {
    let mut sql = String::from(
        "SELECT date(t.created_at) AS day,
                SUM(ti.qty) AS qty,
                SUM(ti.price * ti.qty) AS gross,
                SUM(ti.discount) AS discount,
                SUM(ti.line_total) AS net
         FROM transaction_items ti
         JOIN transactions t ON t.id = ti.transaction_id
         LEFT JOIN products p ON p.id = ti.product_id
         WHERE date(t.created_at) >= date(?1) AND date(t.created_at) <= date(?2)",
    );
    if !brands.is_empty() {
        let placeholders = brands.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        sql.push_str(&format!(" AND p.brand IN ({placeholders})"));
    }
    sql.push_str(" GROUP BY day ORDER BY day ASC");

    let mut stmt = conn.prepare(&sql)?;
    let mut param_values: Vec<&dyn rusqlite::ToSql> = vec![&from, &to];
    for b in brands {
        param_values.push(b);
    }
    let rows = stmt
        .query_map(param_values.as_slice(), |r| {
            Ok(DailySalesRow {
                day: r.get(0)?,
                qty: r.get(1)?,
                gross: r.get(2)?,
                discount: r.get(3)?,
                net: r.get(4)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

// ---------- Alur Barang (buku besar stok) ----------

use crate::models::{StockFlowDetail, StockFlowRow};
use std::collections::HashMap;

/// Stok terakhir yang tercatat per barang pada batas tanggal tertentu, diambil
/// dari kolom `stock_after` mutasi paling akhir yang memenuhi `date_clause`.
/// Satu query untuk SEMUA barang (window function), bukan subquery per barang,
/// supaya rekap "semua barang" tetap cepat di database dengan ribuan produk.
fn stock_at_boundary(
    conn: &Connection,
    date_clause: &str,
    bound: &str,
) -> AppResult<HashMap<String, f64>> {
    let sql = format!(
        "SELECT product_id, stock_after FROM (
             SELECT product_id, stock_after,
                    ROW_NUMBER() OVER (PARTITION BY product_id
                                       ORDER BY created_at DESC, id DESC) AS rn
             FROM stock_movements
             WHERE {date_clause}
         ) WHERE rn = 1"
    );
    let mut stmt = conn.prepare(&sql)?;
    let mut out = HashMap::new();
    let rows = stmt.query_map(params![bound], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, f64>(1)?))
    })?;
    for row in rows {
        let (id, qty) = row?;
        out.insert(id, qty);
    }
    Ok(out)
}

/// Rekap alur stok semua barang dalam satu rentang tanggal: stok awal, total
/// masuk/keluar/terjual, penyesuaian opname, dan stok akhir. Barang yang sama
/// sekali tidak bergerak DAN stoknya nol di seluruh rentang tidak ditampilkan.
pub fn stock_flow_recap(
    conn: &Connection,
    from: &str,
    to: &str,
    search: Option<String>,
) -> AppResult<Vec<StockFlowRow>> {
    // 1. Agregasi mutasi di dalam rentang, satu baris per barang.
    let mut stmt = conn.prepare(
        "SELECT product_id,
                SUM(CASE WHEN kind = 'in'   THEN qty ELSE 0 END) AS masuk,
                SUM(CASE WHEN kind = 'out'  THEN qty ELSE 0 END) AS keluar,
                SUM(CASE WHEN kind = 'sale' THEN qty ELSE 0 END) AS terjual,
                COUNT(*) AS n
         FROM stock_movements
         WHERE date(created_at) >= date(?1) AND date(created_at) <= date(?2)
         GROUP BY product_id",
    )?;
    let mut agg: HashMap<String, (f64, f64, f64, i64)> = HashMap::new();
    let rows = stmt.query_map(params![from, to], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, f64>(1)?,
            r.get::<_, f64>(2)?,
            r.get::<_, f64>(3)?,
            r.get::<_, i64>(4)?,
        ))
    })?;
    for row in rows {
        let (id, masuk, keluar, terjual, n) = row?;
        agg.insert(id, (masuk, keluar, terjual, n));
    }

    // 2. Stok awal (sebelum `from`) & stok akhir (sampai `to`) per barang.
    let opening_map = stock_at_boundary(conn, "date(created_at) < date(?1)", from)?;
    let closing_map = stock_at_boundary(conn, "date(created_at) <= date(?1)", to)?;

    // 3. Gabung dengan master barang.
    let mut sql = String::from(
        "SELECT p.id, p.name, p.barcode, p.brand, COALESCE(s.qty, 0)
         FROM products p LEFT JOIN stock s ON s.product_id = p.id
         WHERE p.is_deleted = 0",
    );
    let term = search.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    if term.is_some() {
        sql.push_str(" AND (p.name LIKE ?1 OR IFNULL(p.barcode,'') LIKE ?1)");
    }
    sql.push_str(" ORDER BY p.name ASC");

    let mut stmt = conn.prepare(&sql)?;
    let map_row = |r: &rusqlite::Row| -> rusqlite::Result<(String, String, Option<String>, Option<String>, f64)> {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
    };
    let products: Vec<(String, String, Option<String>, Option<String>, f64)> = match &term {
        Some(t) => stmt
            .query_map(params![format!("%{t}%")], map_row)?
            .collect::<Result<Vec<_>, _>>()?,
        None => stmt.query_map([], map_row)?.collect::<Result<Vec<_>, _>>()?,
    };

    let mut out = Vec::new();
    for (id, name, barcode, brand, current_stock) in products {
        let (masuk, keluar, terjual, n) = agg.get(&id).copied().unwrap_or((0.0, 0.0, 0.0, 0));
        let opening = opening_map.get(&id).copied().unwrap_or(0.0);
        // Tanpa mutasi apa pun sampai `to`, stok akhir = stok awal.
        let closing = closing_map.get(&id).copied().unwrap_or(opening);
        if n == 0 && opening == 0.0 && closing == 0.0 && current_stock == 0.0 {
            continue;
        }
        out.push(StockFlowRow {
            product_id: id,
            name,
            barcode,
            brand,
            opening,
            masuk,
            keluar,
            terjual,
            // Sisa selisih yang tidak dijelaskan masuk/keluar/terjual = koreksi opname.
            adjustment: closing - (opening + masuk - keluar - terjual),
            closing,
            current_stock,
        });
    }
    Ok(out)
}

/// Buku besar stok satu barang dalam rentang tanggal: header barang, stok awal,
/// dan semua mutasi urut kronologis (paling lama di atas) supaya kolom
/// "stok setelah" terbaca sebagai stok berjalan.
pub fn stock_flow_detail(
    conn: &Connection,
    product_id: &str,
    from: &str,
    to: &str,
) -> AppResult<StockFlowDetail> {
    let (name, barcode, brand, unit, current_stock) = conn
        .query_row(
            "SELECT p.name, p.barcode, p.brand, p.unit, COALESCE(s.qty, 0)
             FROM products p LEFT JOIN stock s ON s.product_id = p.id
             WHERE p.id = ?1",
            params![product_id],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, Option<String>>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                    r.get::<_, f64>(4)?,
                ))
            },
        )
        .optional()?
        .ok_or_else(|| AppError::Other("barang tidak ditemukan".into()))?;

    let opening: f64 = conn
        .query_row(
            "SELECT stock_after FROM stock_movements
             WHERE product_id = ?1 AND date(created_at) < date(?2)
             ORDER BY created_at DESC, id DESC LIMIT 1",
            params![product_id, from],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or(0.0);

    let mut stmt = conn.prepare(
        "SELECT m.id, m.product_id, COALESCE(p.name,'') AS product_name, m.kind, m.qty,
                m.stock_after, m.note, m.user_id, m.created_at
         FROM stock_movements m LEFT JOIN products p ON p.id = m.product_id
         WHERE m.product_id = ?1
           AND date(m.created_at) >= date(?2) AND date(m.created_at) <= date(?3)
         ORDER BY m.created_at ASC, m.id ASC",
    )?;
    let rows = stmt
        .query_map(params![product_id, from, to], |r| {
            Ok(StockMovement {
                id: r.get("id")?,
                product_id: r.get("product_id")?,
                product_name: r.get("product_name")?,
                kind: r.get("kind")?,
                qty: r.get("qty")?,
                stock_after: r.get("stock_after")?,
                note: r.get("note")?,
                user_id: r.get("user_id")?,
                created_at: r.get("created_at")?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(StockFlowDetail {
        product_id: product_id.to_string(),
        name,
        barcode,
        brand,
        unit,
        current_stock,
        opening,
        rows,
    })
}
