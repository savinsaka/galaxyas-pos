// TRANSKRIP 1:1 dari desktop/src/lib/types.ts — field SENGAJA snake_case
// persis nama field wire/serde Rust supaya diff terhadap types.ts mekanis dan
// bebas kesalahan naming-strategy. Perubahan types.ts di desktop WAJIB
// direplikasi ke file ini (lihat PROTOCOL.md).
// Hanya tipe yang dipakai 44 command LAN (lan.rs::dispatch); tipe desktop-only
// (SyncResult, StoreInfo, LanServerStatus, PullItem) tidak ikut.
// == types.ts per commit 947e11b ==
@file:Suppress("PropertyName")

package com.galaxyas.mobilepos.data.model

import kotlinx.serialization.Serializable

// ---------- Barang & stok ----------

@Serializable
data class Product(
    val id: String,
    val name: String,
    val barcode: String? = null,
    val category: String? = null,
    val brand: String? = null,
    val unit: String? = null,
    val sell_price: Double,
    val cost_price: Double,
    val default_discount: Double = 0.0,
    val is_active: Boolean = true,
    val is_deleted: Boolean = false,
    val updated_at: String = "",
)

@Serializable
data class ProductWithStock(
    val id: String,
    val name: String,
    val barcode: String? = null,
    val category: String? = null,
    val brand: String? = null,
    val unit: String? = null,
    val sell_price: Double,
    val cost_price: Double,
    val default_discount: Double = 0.0,
    val is_active: Boolean = true,
    val is_deleted: Boolean = false,
    val updated_at: String = "",
    val stock_qty: Double = 0.0,
)

@Serializable
data class ProductPage(
    val items: List<ProductWithStock>,
    val total: Long,
)

@Serializable
data class ProductInput(
    val id: String? = null,
    val name: String,
    val barcode: String? = null,
    val category: String? = null,
    val brand: String? = null,
    val unit: String? = null,
    val sell_price: Double,
    val cost_price: Double,
    val default_discount: Double = 0.0,
    val is_active: Boolean = true,
)

@Serializable
data class DedupeDetail(
    val barcode: String,
    val kept_name: String,
    val removed_count: Long,
)

@Serializable
data class DedupeResult(
    val groups: Long,
    val removed: Long,
    val details: List<DedupeDetail>,
)

// ---------- Penjualan ----------

@Serializable
data class SaleItemInput(
    val product_id: String,
    val name: String,
    val price: Double,
    val qty: Double,
    val discount: Double = 0.0,
)

@Serializable
data class SaleInput(
    val cashier_id: String,
    val payment_method: String, // Tunai | QRIS | Kombinasi | Kartu
    val paid: Double,
    val items: List<SaleItemInput>,
    val customer_id: String? = null,
    val shift_id: String? = null,
    val created_at: String? = null,
    /** Hanya diisi kalau payment_method = "Kombinasi". */
    val paid_cash: Double? = null,
    val paid_qris: Double? = null,
)

@Serializable
data class Transaction(
    val id: String,
    val invoice_no: String,
    val cashier_id: String,
    val subtotal: Double,
    val discount: Double,
    val total: Double,
    val paid: Double,
    val change: Double,
    val payment_method: String,
    val created_at: String,
    val customer_id: String? = null,
    val shift_id: String? = null,
    val paid_cash: Double? = null,
    val paid_qris: Double? = null,
)

@Serializable
data class TransactionItem(
    val product_id: String,
    val name: String,
    val price: Double,
    val qty: Double,
    val discount: Double,
    val line_total: Double,
)

/** Di Rust: struct Transaction ter-flatten + items (serde(flatten)). */
@Serializable
data class TransactionDetail(
    val id: String,
    val invoice_no: String,
    val cashier_id: String,
    val subtotal: Double,
    val discount: Double,
    val total: Double,
    val paid: Double,
    val change: Double,
    val payment_method: String,
    val created_at: String,
    val customer_id: String? = null,
    val shift_id: String? = null,
    val paid_cash: Double? = null,
    val paid_qris: Double? = null,
    val items: List<TransactionItem>,
)

@Serializable
data class TransactionPage(
    val items: List<Transaction>,
    val total: Long,
)

// ---------- Merek ----------

@Serializable
data class Brand(
    val id: String,
    val name: String,
    val updated_at: String = "",
)

@Serializable
data class BrandInput(
    val id: String? = null,
    val name: String,
)

// ---------- Pengguna / hak akses ----------

/** ModuleKey: master | penjualan | persediaan | laporan | pengaturan */
@Serializable
data class User(
    val id: String,
    val username: String,
    val name: String,
    val role: String,
    val permissions: List<String>,
)

@Serializable
data class UserInput(
    val id: String? = null,
    val username: String,
    val name: String,
    val role: String,
    val permissions: List<String>,
    val pin: String? = null,
)

// ---------- Pergerakan stok ----------

/** StockKind: in | out | opname | sale */
@Serializable
data class StockMovement(
    val id: Long,
    val product_id: String,
    val product_name: String,
    val kind: String,
    val qty: Double,
    val note: String? = null,
    val user_id: String? = null,
    val created_at: String,
    val stock_after: Double,
)

@Serializable
data class StockMovementInput(
    val product_id: String,
    val kind: String,
    val qty: Double,
    val note: String? = null,
    val user_id: String? = null,
    val created_at: String? = null,
)

@Serializable
data class StockMovementBatchItemInput(
    val product_id: String,
    val qty: Double,
    val note: String? = null,
)

@Serializable
data class StockMovementBatchInput(
    val kind: String, // in | out
    val note: String? = null,
    val user_id: String? = null,
    val items: List<StockMovementBatchItemInput>,
    val created_at: String? = null,
)

@Serializable
data class StockMovementBatchItem(
    val product_id: String,
    val product_name: String,
    val qty: Double,
    val note: String? = null,
)

@Serializable
data class StockMovementBatch(
    val id: String,
    val no: String,
    val kind: String,
    val note: String? = null,
    val user_id: String? = null,
    val created_at: String,
    val item_count: Long,
    val total_qty: Double,
)

@Serializable
data class StockMovementBatchDetail(
    val id: String,
    val no: String,
    val kind: String,
    val note: String? = null,
    val user_id: String? = null,
    val created_at: String,
    val items: List<StockMovementBatchItem>,
)

@Serializable
data class StockMovementBatchPage(
    val items: List<StockMovementBatch>,
    val total: Long,
)

// ---------- Diskon periodik ----------

@Serializable
data class DiscountPeriod(
    val id: String,
    val code: String,
    val scope: String, // item | brand
    val target: String,
    val target_label: String? = null,
    val discount_type: String, // amount | percent
    val value: Double,
    val days: String,
    val is_active: Boolean = true,
    val priority: Long = 0,
    val updated_at: String = "",
)

@Serializable
data class DiscountPeriodInput(
    val id: String? = null,
    val code: String,
    val scope: String,
    val target: String,
    val target_label: String? = null,
    val discount_type: String,
    val value: Double,
    val days: String,
    val is_active: Boolean = true,
    val priority: Long = 0,
)

// ---------- Pelanggan ----------

@Serializable
data class Customer(
    val id: String,
    val name: String,
    val phone: String? = null,
    val email: String? = null,
    val address: String? = null,
    val note: String? = null,
    val is_active: Boolean = true,
    val updated_at: String = "",
)

@Serializable
data class CustomerInput(
    val id: String? = null,
    val name: String,
    val phone: String? = null,
    val email: String? = null,
    val address: String? = null,
    val note: String? = null,
    val is_active: Boolean = true,
)

// ---------- Pengeluaran ----------

@Serializable
data class Expense(
    val id: String,
    val date: String,
    val category: String,
    val amount: Double,
    val note: String? = null,
    val user_id: String? = null,
    val created_at: String,
)

@Serializable
data class ExpenseInput(
    val id: String? = null,
    val date: String,
    val category: String,
    val amount: Double,
    val note: String? = null,
    val user_id: String? = null,
)

// ---------- Shift kasir ----------

@Serializable
data class Shift(
    val id: String,
    val user_id: String,
    val user_name: String,
    val opening_cash: Double,
    val closing_cash: Double? = null,
    val expected_cash: Double? = null,
    val difference: Double? = null,
    val note: String? = null,
    val opened_at: String,
    val closed_at: String? = null,
)

@Serializable
data class OpenShiftInput(
    val user_id: String,
    val user_name: String,
    val opening_cash: Double,
)

@Serializable
data class CloseShiftInput(
    val id: String,
    val closing_cash: Double,
    val note: String? = null,
)

// ---------- Laporan ----------

@Serializable
data class ProductSalesRow(
    val product_id: String,
    val name: String,
    val brand: String? = null,
    val qty: Double,
    val gross: Double,
    val discount: Double,
    val net: Double,
    val cogs: Double,
)

@Serializable
data class BrandSalesRow(
    val brand: String,
    val qty: Double,
    val gross: Double,
    val discount: Double,
    val net: Double,
)

@Serializable
data class SalesItemDetailRow(
    val invoice_no: String,
    val created_at: String,
    val cashier_id: String,
    val product_id: String,
    val name: String,
    val brand: String? = null,
    val qty: Double,
    val price: Double,
    val discount: Double,
    val net: Double,
)

@Serializable
data class DailySalesRow(
    val day: String,
    val qty: Double,
    val gross: Double,
    val discount: Double,
    val net: Double,
)

// ---------- Pairing perangkat ----------

/**
 * Balasan `POST /pair` (desktop `models.rs::PairResult`). Satu-satunya saat
 * token perangkat dikirim ke HP — sesudah itu hanya ada hash-nya di PC kasir.
 */
@Serializable
data class PairResult(
    val device_id: String,
    val device_token: String,
    val store_name: String,
)

/**
 * Isi QR pairing (desktop `lan.rs::PairingPayload`). Semua bawaan diberi nilai
 * default supaya QR dari desktop versi lain tidak bikin app gagal parse.
 */
@Serializable
data class PairingPayload(
    val v: Int = 1,
    val name: String = "",
    val host: String = "",
    val port: Int = 8899,
    val relay: String = "",
    val store_id: String = "",
    val code: String = "",
)
