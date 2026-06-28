use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::models::{
    Product, ProductInput, ProductWithStock, SaleInput, Transaction, TransactionDetail,
    TransactionItem,
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

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY,
            value TEXT
        );
        "#,
    )?;

    // Migrasi ringan untuk instalasi lama: tambah kolom permissions bila belum ada.
    let has_perm: bool = conn
        .prepare("SELECT 1 FROM pragma_table_info('users') WHERE name='permissions'")?
        .query_row([], |_| Ok(true))
        .optional()?
        .unwrap_or(false);
    if !has_perm {
        conn.execute("ALTER TABLE users ADD COLUMN permissions TEXT NOT NULL DEFAULT '[]'", [])?;
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

/// Daftar barang + stok. `search` mencocokkan nama atau barcode.
pub fn list_products(
    conn: &Connection,
    search: Option<String>,
    include_inactive: bool,
) -> AppResult<Vec<ProductWithStock>> {
    let like = format!("%{}%", search.unwrap_or_default());
    let active_clause = if include_inactive { "1=1" } else { "p.is_active = 1" };
    let sql = format!(
        "SELECT p.*, COALESCE(s.qty, 0) AS stock_qty
         FROM products p
         LEFT JOIN stock s ON s.product_id = p.id
         WHERE p.is_deleted = 0 AND {active_clause}
           AND (p.name LIKE ?1 OR COALESCE(p.barcode,'') LIKE ?1)
         ORDER BY p.name ASC"
    );
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt
        .query_map(params![like], |row| {
            let stock_qty: f64 = row.get("stock_qty")?;
            Ok(ProductWithStock { product: map_product(row)?, stock_qty })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn find_by_barcode(conn: &Connection, barcode: &str) -> AppResult<Option<ProductWithStock>> {
    let res = conn
        .query_row(
            "SELECT p.*, COALESCE(s.qty,0) AS stock_qty
             FROM products p LEFT JOIN stock s ON s.product_id = p.id
             WHERE p.barcode = ?1 AND p.is_deleted = 0 AND p.is_active = 1",
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

    get_product(conn, &id)?.ok_or_else(|| AppError::Other("produk gagal disimpan".into()))
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

/// Simpan transaksi penjualan & kurangi stok dalam satu transaksi DB.
pub fn create_sale(conn: &mut Connection, input: SaleInput) -> AppResult<TransactionDetail> {
    if input.items.is_empty() {
        return Err(AppError::Other("transaksi tidak boleh kosong".into()));
    }
    let now = Utc::now().to_rfc3339();
    let tx_id = Uuid::new_v4().to_string();
    let invoice_no = next_invoice_no(conn)?;

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
    db_tx.execute(
        "INSERT INTO transactions
            (id, invoice_no, cashier_id, subtotal, discount, total, paid, change, payment_method, created_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
        params![
            tx_id, invoice_no, input.cashier_id, subtotal, total_discount, total,
            input.paid, change, input.payment_method, now
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
        },
        items,
    })
}

pub fn list_transactions(conn: &Connection, limit: i64) -> AppResult<Vec<Transaction>> {
    let mut stmt = conn.prepare(
        "SELECT * FROM transactions ORDER BY created_at DESC LIMIT ?1",
    )?;
    let rows = stmt
        .query_map(params![limit], |r| {
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
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn get_transaction(conn: &Connection, id: &str) -> AppResult<Option<TransactionDetail>> {
    let header = conn
        .query_row("SELECT * FROM transactions WHERE id = ?1", params![id], |r| {
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
            })
        })
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

pub fn mark_products_synced(conn: &Connection, ids: &[String]) -> AppResult<()> {
    for id in ids {
        conn.execute("UPDATE products SET dirty = 0 WHERE id = ?1", params![id])?;
    }
    Ok(())
}

/// Terapkan produk hasil pull dari server dengan Last Write Wins berbasis `updated_at`.
pub fn apply_pulled_products(conn: &Connection, incoming: &[Product]) -> AppResult<(i64, i64)> {
    let mut applied = 0i64;
    let mut skipped = 0i64;
    for p in incoming {
        let local_updated: Option<String> = conn
            .query_row(
                "SELECT updated_at FROM products WHERE id = ?1",
                params![p.id],
                |r| r.get(0),
            )
            .optional()?;

        let should_apply = match &local_updated {
            None => true,
            // Bandingkan ISO-8601 (UTC) secara leksikografis = urutan kronologis.
            Some(local) => p.updated_at.as_str() > local.as_str(),
        };

        if should_apply {
            conn.execute(
                "INSERT INTO products
                    (id, name, barcode, category, brand, unit, sell_price, cost_price,
                     default_discount, is_active, is_deleted, updated_at, dirty)
                 VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,0)
                 ON CONFLICT(id) DO UPDATE SET
                    name=excluded.name, barcode=excluded.barcode, category=excluded.category,
                    brand=excluded.brand, unit=excluded.unit, sell_price=excluded.sell_price,
                    cost_price=excluded.cost_price, default_discount=excluded.default_discount,
                    is_active=excluded.is_active, is_deleted=excluded.is_deleted,
                    updated_at=excluded.updated_at, dirty=0",
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
            applied += 1;
        } else {
            skipped += 1;
        }
    }
    Ok((applied, skipped))
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
    let now = Utc::now().to_rfc3339();
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
        scope: row.get("scope")?,
        target: row.get("target")?,
        target_label: row.get("target_label")?,
        discount_type: row.get("discount_type")?,
        value: row.get("value")?,
        days: row.get("days")?,
        is_active: row.get::<_, i64>("is_active")? != 0,
        updated_at: row.get("updated_at")?,
    })
}

pub fn list_discounts(conn: &Connection) -> AppResult<Vec<DiscountPeriod>> {
    let mut stmt = conn.prepare("SELECT * FROM discount_periods ORDER BY updated_at DESC")?;
    let rows = stmt.query_map([], map_discount)?.collect::<Result<Vec<_>, _>>()?;
    Ok(rows)
}

pub fn save_discount(conn: &Connection, input: DiscountPeriodInput) -> AppResult<DiscountPeriod> {
    let id = input.id.clone().unwrap_or_else(|| Uuid::new_v4().to_string());
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "INSERT INTO discount_periods
            (id, scope, target, target_label, discount_type, value, days, is_active, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)
         ON CONFLICT(id) DO UPDATE SET
            scope=excluded.scope, target=excluded.target, target_label=excluded.target_label,
            discount_type=excluded.discount_type, value=excluded.value, days=excluded.days,
            is_active=excluded.is_active, updated_at=excluded.updated_at",
        params![
            id, input.scope, input.target, input.target_label, input.discount_type,
            input.value, input.days, input.is_active as i64, now
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
