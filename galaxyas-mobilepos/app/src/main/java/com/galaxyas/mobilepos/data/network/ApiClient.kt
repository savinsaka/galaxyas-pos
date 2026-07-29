package com.galaxyas.mobilepos.data.network

import com.galaxyas.mobilepos.data.model.*
import kotlinx.serialization.json.JsonElement
import kotlinx.serialization.json.JsonNull
import kotlinx.serialization.json.JsonObjectBuilder
import kotlinx.serialization.json.buildJsonObject
import kotlinx.serialization.json.encodeToJsonElement
import kotlinx.serialization.json.put
import kotlinx.serialization.serializer

/**
 * Wrapper terketik untuk SEMUA command LAN Server Pusat - mirror satu-banding-
 * satu dari desktop/src/lib/api.ts (nama method sama, arg wire snake_case sama;
 * lihat PROTOCOL.md). Command yang di desktop bersifat lokal (get_settings,
 * list_printers, sync_*, bridge_*, store/server registry) TIDAK ada di sini.
 */
class ApiClient(private val rpc: RpcClient) {

    private val json = RpcClient.json

    private suspend inline fun <reified R> call(name: String, args: JsonElement): R {
        val result = rpc.call(name, args)
        return json.decodeFromJsonElement(json.serializersModule.serializer<R>(), result)
    }

    private suspend inline fun <reified R> call(
        name: String,
        crossinline build: JsonObjectBuilder.() -> Unit,
    ): R = call(name, buildJsonObject { build() })

    /** Command yang me-return () di Rust - body respons null, tidak di-decode. */
    private suspend inline fun callVoid(
        name: String,
        crossinline build: JsonObjectBuilder.() -> Unit,
    ) {
        rpc.call(name, buildJsonObject { build() })
    }

    // ---------- Barang & stok ----------

    suspend fun listProducts(
        search: String = "",
        includeInactive: Boolean = false,
        limit: Long? = null,
    ): List<ProductWithStock> = call("list_products") {
        put("search", search)
        put("include_inactive", includeInactive)
        limit?.let { put("limit", it) }
    }

    suspend fun listProductsPage(
        search: String = "",
        includeInactive: Boolean = false,
        brand: String? = null,
        sortBy: String = "name",
        sortDir: String = "asc",
        limit: Long,
        offset: Long,
    ): ProductPage = call("list_products_page") {
        put("search", search)
        put("include_inactive", includeInactive)
        brand?.let { put("brand", it) }
        put("sort_by", sortBy)
        put("sort_dir", sortDir)
        put("limit", limit)
        put("offset", offset)
    }

    suspend fun saveProduct(input: ProductInput): Product =
        call("save_product", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    suspend fun toggleProductActive(id: String, active: Boolean) =
        callVoid("toggle_product_active") {
            put("id", id)
            put("active", active)
        }

    suspend fun deleteProduct(id: String) = callVoid("delete_product") { put("id", id) }

    suspend fun dedupeProducts(): DedupeResult = call("dedupe_products", JsonNull)

    suspend fun findByBarcode(barcode: String): ProductWithStock? =
        call("find_by_barcode") { put("barcode", barcode) }

    suspend fun adjustStock(productId: String, delta: Double): Double =
        call("adjust_stock") {
            put("product_id", productId)
            put("delta", delta)
        }

    suspend fun setStock(productId: String, qty: Double): Double =
        call("set_stock") {
            put("product_id", productId)
            put("qty", qty)
        }

    // ---------- Penjualan / kasir ----------

    suspend fun checkout(sale: SaleInput): TransactionDetail =
        call("checkout", buildJsonObject { put("sale", json.encodeToJsonElement(sale)) })

    suspend fun listTransactionsPage(
        from: String? = null,
        to: String? = null,
        limit: Long = 50,
        offset: Long = 0,
        search: String? = null,
    ): TransactionPage = call("list_transactions") {
        from?.let { put("from", it) }
        to?.let { put("to", it) }
        search?.let { put("search", it) }
        put("limit", limit)
        put("offset", offset)
    }

    suspend fun getTransaction(id: String): TransactionDetail? =
        call("get_transaction") { put("id", id) }

    suspend fun deleteTransaction(id: String) =
        callVoid("delete_transaction") { put("id", id) }

    suspend fun updateTransaction(id: String, sale: SaleInput): TransactionDetail =
        call(
            "update_transaction",
            buildJsonObject {
                put("id", id)
                put("input", json.encodeToJsonElement(sale))
            },
        )

    // ---------- Pengguna / hak akses ----------

    suspend fun login(username: String, pin: String): User? =
        call("login") {
            put("username", username)
            put("pin", pin)
        }

    suspend fun listUsers(): List<User> = call("list_users", JsonNull)

    suspend fun saveUser(input: UserInput): User =
        call("save_user", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    suspend fun deleteUser(id: String) = callVoid("delete_user") { put("id", id) }

    // ---------- Pergerakan stok ----------

    suspend fun createStockMovement(input: StockMovementInput): StockMovement =
        call("create_stock_movement", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    suspend fun listStockMovements(
        kind: String? = null,
        from: String? = null,
        to: String? = null,
        limit: Long = 500,
    ): List<StockMovement> = call("list_stock_movements") {
        kind?.let { put("kind", it) }
        from?.let { put("from", it) }
        to?.let { put("to", it) }
        put("limit", limit)
    }

    suspend fun deleteStockMovement(id: Long) =
        callVoid("delete_stock_movement") { put("id", id) }

    // ---------- Opname Spesial ----------

    /** Satu merek disisir habis: yang dihitung diset, sisanya dinolkan di server. */
    suspend fun createOpnameSpecial(input: OpnameSpecialInput): OpnameSpecialResult =
        call("create_opname_special", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    // ---------- Batch Item Masuk / Keluar ----------

    suspend fun createStockMovementBatch(input: StockMovementBatchInput): StockMovementBatchDetail =
        call(
            "create_stock_movement_batch",
            buildJsonObject { put("input", json.encodeToJsonElement(input)) },
        )

    suspend fun listStockMovementBatches(
        kind: String? = null,
        from: String? = null,
        to: String? = null,
        limit: Long = 50,
        offset: Long = 0,
    ): StockMovementBatchPage = call("list_stock_movement_batches") {
        kind?.let { put("kind", it) }
        from?.let { put("from", it) }
        to?.let { put("to", it) }
        put("limit", limit)
        put("offset", offset)
    }

    suspend fun getStockMovementBatch(id: String): StockMovementBatchDetail? =
        call("get_stock_movement_batch") { put("id", id) }

    suspend fun updateStockMovementBatch(
        id: String,
        items: List<StockMovementBatchItemInput>,
        note: String?,
    ): StockMovementBatchDetail = call(
        "update_stock_movement_batch",
        buildJsonObject {
            put("id", id)
            put("items", json.encodeToJsonElement(items))
            note?.let { put("note", it) }
        },
    )

    suspend fun deleteStockMovementBatch(id: String) =
        callVoid("delete_stock_movement_batch") { put("id", id) }

    // ---------- Diskon periodik ----------

    suspend fun listDiscounts(): List<DiscountPeriod> = call("list_discounts", JsonNull)

    suspend fun saveDiscount(input: DiscountPeriodInput): DiscountPeriod =
        call("save_discount", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    suspend fun deleteDiscount(id: String) = callVoid("delete_discount") { put("id", id) }

    // ---------- Merek ----------

    suspend fun listBrands(): List<Brand> = call("list_brands", JsonNull)

    suspend fun saveBrand(input: BrandInput): Brand =
        call("save_brand", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    suspend fun deleteBrand(id: String) = callVoid("delete_brand") { put("id", id) }

    // ---------- Pelanggan ----------

    suspend fun listCustomers(search: String = "", includeInactive: Boolean = false): List<Customer> =
        call("list_customers") {
            put("search", search)
            put("include_inactive", includeInactive)
        }

    suspend fun saveCustomer(input: CustomerInput): Customer =
        call("save_customer", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    suspend fun deleteCustomer(id: String) = callVoid("delete_customer") { put("id", id) }

    // ---------- Pengeluaran ----------

    suspend fun listExpenses(from: String? = null, to: String? = null): List<Expense> =
        call("list_expenses") {
            from?.let { put("from", it) }
            to?.let { put("to", it) }
        }

    suspend fun saveExpense(input: ExpenseInput): Expense =
        call("save_expense", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    suspend fun deleteExpense(id: String) = callVoid("delete_expense") { put("id", id) }

    // ---------- Shift kasir ----------

    suspend fun getActiveShift(): Shift? = call("get_active_shift", JsonNull)

    suspend fun openShift(input: OpenShiftInput): Shift =
        call("open_shift", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    suspend fun closeShift(input: CloseShiftInput): Shift =
        call("close_shift", buildJsonObject { put("input", json.encodeToJsonElement(input)) })

    suspend fun listShifts(limit: Long = 100): List<Shift> =
        call("list_shifts") { put("limit", limit) }

    // ---------- Laporan ----------

    suspend fun productSalesReport(from: String, to: String, brands: List<String> = emptyList()): List<ProductSalesRow> =
        reportCall("product_sales_report", from, to, brands)

    suspend fun brandSalesReport(from: String, to: String, brands: List<String> = emptyList()): List<BrandSalesRow> =
        reportCall("brand_sales_report", from, to, brands)

    suspend fun salesItemDetailReport(from: String, to: String, brands: List<String> = emptyList()): List<SalesItemDetailRow> =
        reportCall("sales_item_detail_report", from, to, brands)

    suspend fun dailySalesReport(from: String, to: String, brands: List<String> = emptyList()): List<DailySalesRow> =
        reportCall("daily_sales_report", from, to, brands)

    private suspend inline fun <reified R> reportCall(
        name: String,
        from: String,
        to: String,
        brands: List<String>,
    ): R = call(name) {
        put("from", from)
        put("to", to)
        put("brands", json.encodeToJsonElement(brands))
    }
}
