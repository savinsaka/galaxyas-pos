-- GalaxyAS POS initial local SQLite schema (UUID v7 primary keys, epoch-ms timestamps).
PRAGMA foreign_keys = ON;

CREATE TABLE stores (
  id           TEXT PRIMARY KEY,
  name         TEXT NOT NULL,
  address      TEXT,
  phone        TEXT,
  tax_percent  REAL NOT NULL DEFAULT 0,
  created_at   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  sync_status  TEXT NOT NULL DEFAULT 'pending'
);

CREATE TABLE users (
  id            TEXT PRIMARY KEY,
  store_id      TEXT NOT NULL,
  username      TEXT NOT NULL UNIQUE,
  full_name     TEXT NOT NULL,
  password_hash TEXT NOT NULL,
  role          TEXT NOT NULL CHECK (role IN ('admin','supervisor','kasir')),
  is_active     INTEGER NOT NULL DEFAULT 1,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  sync_status   TEXT NOT NULL DEFAULT 'pending'
);

CREATE TABLE items (
  id            TEXT PRIMARY KEY,
  store_id      TEXT NOT NULL,
  kode_item     TEXT,
  barcode       TEXT,
  nama_item     TEXT NOT NULL,
  jenis         TEXT,
  merek         TEXT,
  satuan_dasar  TEXT,
  harga_beli    REAL NOT NULL DEFAULT 0,
  harga_jual    REAL NOT NULL DEFAULT 0,
  diskon_persen REAL NOT NULL DEFAULT 0,
  stok          REAL NOT NULL DEFAULT 0,
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  deleted_at    INTEGER,
  sync_status   TEXT NOT NULL DEFAULT 'pending'
);
-- Barcode unique per store (operational scope; cross-store handled by conflict log).
CREATE UNIQUE INDEX idx_items_barcode_store ON items(store_id, barcode) WHERE barcode IS NOT NULL AND deleted_at IS NULL;
CREATE INDEX idx_items_kode ON items(kode_item);
CREATE INDEX idx_items_nama ON items(nama_item);

CREATE TABLE stock_transactions (
  id           TEXT PRIMARY KEY,
  store_id     TEXT NOT NULL,
  item_id      TEXT NOT NULL,
  type         TEXT NOT NULL CHECK (type IN ('in','out','adjustment','opname')),
  qty          REAL NOT NULL,
  qty_before   REAL NOT NULL,
  qty_after    REAL NOT NULL,
  ref_doc      TEXT,
  note         TEXT,
  user_id      TEXT NOT NULL,
  created_at   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  sync_status  TEXT NOT NULL DEFAULT 'pending'
);
CREATE INDEX idx_stocktx_item ON stock_transactions(item_id, created_at DESC);

CREATE TABLE shifts (
  id            TEXT PRIMARY KEY,
  store_id      TEXT NOT NULL,
  user_id       TEXT NOT NULL,
  opening_cash  REAL NOT NULL DEFAULT 0,
  closing_cash  REAL,
  expected_cash REAL,
  total_sales   REAL NOT NULL DEFAULT 0,
  opened_at     INTEGER NOT NULL,
  closed_at     INTEGER,
  status        TEXT NOT NULL DEFAULT 'open' CHECK (status IN ('open','closed')),
  created_at    INTEGER NOT NULL,
  updated_at    INTEGER NOT NULL,
  sync_status   TEXT NOT NULL DEFAULT 'pending'
);
CREATE INDEX idx_shifts_status ON shifts(status);

CREATE TABLE sales (
  id           TEXT PRIMARY KEY,
  store_id     TEXT NOT NULL,
  invoice_no   TEXT,
  user_id      TEXT NOT NULL,
  shift_id     TEXT,
  subtotal     REAL NOT NULL DEFAULT 0,
  diskon       REAL NOT NULL DEFAULT 0,
  pajak        REAL NOT NULL DEFAULT 0,
  total        REAL NOT NULL DEFAULT 0,
  bayar        REAL NOT NULL DEFAULT 0,
  kembali      REAL NOT NULL DEFAULT 0,
  status       TEXT NOT NULL CHECK (status IN ('held','completed','void')),
  created_at   INTEGER NOT NULL,
  updated_at   INTEGER NOT NULL,
  deleted_at   INTEGER,
  sync_status  TEXT NOT NULL DEFAULT 'pending'
);
CREATE INDEX idx_sales_status ON sales(status, created_at DESC);
CREATE INDEX idx_sales_created ON sales(created_at);

CREATE TABLE sale_items (
  id          TEXT PRIMARY KEY,
  sale_id     TEXT NOT NULL,
  item_id     TEXT NOT NULL,
  nama_item   TEXT NOT NULL,
  qty         REAL NOT NULL,
  harga       REAL NOT NULL,
  diskon      REAL NOT NULL DEFAULT 0,
  subtotal    REAL NOT NULL,
  created_at  INTEGER NOT NULL,
  updated_at  INTEGER NOT NULL,
  sync_status TEXT NOT NULL DEFAULT 'pending'
);
CREATE INDEX idx_saleitems_sale ON sale_items(sale_id);
CREATE INDEX idx_saleitems_item ON sale_items(item_id);

CREATE TABLE audit_logs (
  id          TEXT PRIMARY KEY,
  store_id    TEXT NOT NULL,
  user_id     TEXT NOT NULL,
  action      TEXT NOT NULL,
  entity_type TEXT,
  entity_id   TEXT,
  detail      TEXT,
  created_at  INTEGER NOT NULL
);
CREATE INDEX idx_audit_created ON audit_logs(created_at DESC);

-- Sync infrastructure -------------------------------------------------------
CREATE TABLE sync_queue (
  id              TEXT PRIMARY KEY,
  entity_type     TEXT NOT NULL,
  entity_id       TEXT NOT NULL,
  operation       TEXT NOT NULL CHECK (operation IN ('insert','update','delete')),
  payload         TEXT NOT NULL,
  base_updated_at INTEGER,
  status          TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending','sent','acked','failed')),
  retry_count     INTEGER NOT NULL DEFAULT 0,
  last_error      TEXT,
  created_at      INTEGER NOT NULL,
  next_retry_at   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX idx_queue_status ON sync_queue(status, next_retry_at);

CREATE TABLE sync_conflicts (
  id             TEXT PRIMARY KEY,
  entity_type    TEXT NOT NULL,
  entity_id      TEXT NOT NULL,
  local_payload  TEXT NOT NULL,
  server_payload TEXT NOT NULL,
  conflict_field TEXT,
  resolution     TEXT NOT NULL DEFAULT 'pending' CHECK (resolution IN ('pending','kept_local','kept_server')),
  resolved_by    TEXT,
  resolved_at    INTEGER,
  created_at     INTEGER NOT NULL
);
CREATE INDEX idx_conflicts_resolution ON sync_conflicts(resolution);

CREATE TABLE sync_meta (
  key   TEXT PRIMARY KEY,
  value TEXT
);

INSERT INTO sync_meta (key, value) VALUES ('last_pull_at', '0'), ('last_push_at', '0');
