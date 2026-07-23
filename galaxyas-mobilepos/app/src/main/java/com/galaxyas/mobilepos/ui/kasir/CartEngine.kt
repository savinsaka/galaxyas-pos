package com.galaxyas.mobilepos.ui.kasir

import com.galaxyas.mobilepos.data.model.DiscountPeriod
import com.galaxyas.mobilepos.data.model.ProductWithStock
import java.util.Calendar
import kotlin.math.max
import kotlin.math.min
import kotlinx.serialization.Serializable

/** Satu baris keranjang (padanan CartLine di KasirPOS.svelte). Serializable
 *  agar bisa dipersist di PendingSalesStore (hold transaksi). */
@Serializable
data class CartLine(
    val product_id: String,
    val name: String,
    val price: Double,
    val qty: Double,
    val discount: Double,
    val brand: String?,
    val default_discount: Double,
    val periodic: Boolean,
    val manualOverride: Boolean,
    val stock_qty: Double,
) {
    val lineTotal: Double get() = price * qty - discount
}

/**
 * Engine diskon periodik — port PERSIS dari KasirPOS.svelte (applyDiscount/
 * dayMatches/discountValue/todayKey). Dijaga oleh DiscountEngineTest.
 */
object DiscountEngine {
    // JS getDay(): 0=Min..6=Sab. Calendar.DAY_OF_WEEK: 1=Min..7=Sab.
    val DAY_KEYS = listOf("min", "sen", "sel", "rab", "kam", "jum", "sab")

    fun todayKey(cal: Calendar = Calendar.getInstance()): String =
        DAY_KEYS[cal.get(Calendar.DAY_OF_WEEK) - 1]

    fun dayMatches(days: String, key: String): Boolean {
        if (days == "everyday") return true
        return days.split(",").map { it.trim() }.contains(key)
    }

    fun discountValue(d: DiscountPeriod, price: Double, qty: Double): Double =
        if (d.discount_type == "percent") price * qty * d.value / 100 else d.value * qty

    /**
     * Hitung nominal diskon baris: diskon periodik (item > brand, ambil nominal
     * terbesar) atau fallback default_discount. Return (diskon, periodic).
     * Diklamp ke [0, price*qty].
     */
    fun computeLineDiscount(
        productId: String,
        brand: String?,
        price: Double,
        qty: Double,
        defaultDiscount: Double,
        discounts: List<DiscountPeriod>,
        dayKey: String,
    ): Pair<Double, Boolean> {
        val matches = discounts.filter { d ->
            d.is_active &&
                dayMatches(d.days, dayKey) &&
                ((d.scope == "item" && d.target == productId) ||
                    (d.scope == "brand" && d.target == brand))
        }
        val discount: Double
        val periodic: Boolean
        if (matches.isNotEmpty()) {
            val items = matches.filter { it.scope == "item" }
            val pool = items.ifEmpty { matches }
            discount = pool.maxOf { discountValue(it, price, qty) }
            periodic = true
        } else {
            discount = defaultDiscount * qty
            periodic = false
        }
        return min(max(discount, 0.0), price * qty) to periodic
    }
}

/**
 * Operasi keranjang murni (tanpa state Compose) supaya bisa diuji unit.
 * Mengembalikan daftar baris baru + info error stok jika ada.
 */
object Cart {
    data class AddResult(val cart: List<CartLine>, val stockError: StockError? = null)
    data class StockError(val name: String, val available: Double)

    fun add(
        cart: List<CartLine>,
        p: ProductWithStock,
        addQty: Double,
        discounts: List<DiscountPeriod>,
        dayKey: String,
    ): AddResult {
        val existing = cart.firstOrNull { it.product_id == p.id }
        val currentQty = existing?.qty ?: 0.0
        if (currentQty + addQty > p.stock_qty) {
            return AddResult(cart, StockError(p.name, p.stock_qty))
        }
        return if (existing != null) {
            val newQty = existing.qty + addQty
            val line = if (!existing.manualOverride) {
                val (disc, per) = DiscountEngine.computeLineDiscount(
                    p.id, p.brand, existing.price, newQty, existing.default_discount, discounts, dayKey,
                )
                existing.copy(qty = newQty, stock_qty = p.stock_qty, discount = disc, periodic = per)
            } else {
                existing.copy(qty = newQty, stock_qty = p.stock_qty)
            }
            AddResult(cart.map { if (it.product_id == p.id) line else it })
        } else {
            val (disc, per) = DiscountEngine.computeLineDiscount(
                p.id, p.brand, p.sell_price, addQty, p.default_discount, discounts, dayKey,
            )
            val line = CartLine(
                product_id = p.id, name = p.name, price = p.sell_price, qty = addQty,
                discount = disc, brand = p.brand, default_discount = p.default_discount,
                periodic = per, manualOverride = false, stock_qty = p.stock_qty,
            )
            AddResult(cart + line)
        }
    }

    fun setQty(
        cart: List<CartLine>,
        productId: String,
        qty: Double,
        discounts: List<DiscountPeriod>,
        dayKey: String,
    ): AddResult {
        val line = cart.firstOrNull { it.product_id == productId } ?: return AddResult(cart)
        val wanted = max(1.0, qty)
        if (wanted > line.stock_qty) {
            val clamped = if (line.stock_qty > 0) line.stock_qty else line.qty
            val updated = recalc(line.copy(qty = clamped), discounts, dayKey)
            return AddResult(
                cart.map { if (it.product_id == productId) updated else it },
                StockError(line.name, line.stock_qty),
            )
        }
        val updated = recalc(line.copy(qty = wanted), discounts, dayKey)
        return AddResult(cart.map { if (it.product_id == productId) updated else it })
    }

    private fun recalc(line: CartLine, discounts: List<DiscountPeriod>, dayKey: String): CartLine =
        if (!line.manualOverride) {
            val (disc, per) = DiscountEngine.computeLineDiscount(
                line.product_id, line.brand, line.price, line.qty, line.default_discount, discounts, dayKey,
            )
            line.copy(discount = disc, periodic = per)
        } else {
            line.copy(discount = min(line.discount, line.price * line.qty))
        }

    fun setManualDiscount(cart: List<CartLine>, productId: String, discount: Double): List<CartLine> =
        cart.map {
            if (it.product_id == productId) {
                it.copy(
                    discount = min(max(0.0, discount), it.price * it.qty),
                    manualOverride = true,
                    periodic = false,
                )
            } else it
        }

    fun setPrice(cart: List<CartLine>, productId: String, price: Double): List<CartLine> =
        cart.map {
            if (it.product_id == productId) {
                val p = max(0.0, price)
                it.copy(price = p, discount = min(it.discount, p * it.qty), manualOverride = true)
            } else it
        }

    fun remove(cart: List<CartLine>, productId: String): List<CartLine> =
        cart.filter { it.product_id != productId }

    // Total-total (padanan $derived di KasirPOS.svelte).
    fun totalQty(cart: List<CartLine>) = cart.sumOf { it.qty }
    fun subtotal(cart: List<CartLine>) = cart.sumOf { it.price * it.qty }
    fun totalDiscount(cart: List<CartLine>) = cart.sumOf { it.discount }
    fun total(cart: List<CartLine>) = max(subtotal(cart) - totalDiscount(cart), 0.0)
}
