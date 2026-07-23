package com.galaxyas.mobilepos

import com.galaxyas.mobilepos.data.model.DiscountPeriod
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.ui.kasir.Cart
import com.galaxyas.mobilepos.ui.kasir.DiscountEngine
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

/** Table-driven — mereplikasi perilaku applyDiscount di KasirPOS.svelte. */
class DiscountEngineTest {

    private fun disc(
        id: String,
        scope: String,
        target: String,
        type: String,
        value: Double,
        days: String = "everyday",
        active: Boolean = true,
    ) = DiscountPeriod(
        id = id, code = id, scope = scope, target = target, target_label = null,
        discount_type = type, value = value, days = days, is_active = active, priority = 0,
    )

    private fun compute(
        productId: String,
        brand: String?,
        price: Double,
        qty: Double,
        defaultDiscount: Double,
        discounts: List<DiscountPeriod>,
        dayKey: String = "sen",
    ) = DiscountEngine.computeLineDiscount(productId, brand, price, qty, defaultDiscount, discounts, dayKey)

    @Test
    fun `tanpa diskon periodik pakai default_discount per qty`() {
        val (d, periodic) = compute("p1", "A", 10_000.0, 3.0, 500.0, emptyList())
        assertEquals(1_500.0, d, 0.0)
        assertFalse(periodic)
    }

    @Test
    fun `diskon item percent dihitung dari price x qty`() {
        val (d, periodic) = compute("p1", "A", 10_000.0, 2.0, 0.0, listOf(disc("x", "item", "p1", "percent", 10.0)))
        assertEquals(2_000.0, d, 0.0) // 10% dari 20.000
        assertTrue(periodic)
    }

    @Test
    fun `diskon item amount dikali qty`() {
        val (d, _) = compute("p1", "A", 10_000.0, 3.0, 0.0, listOf(disc("x", "item", "p1", "amount", 500.0)))
        assertEquals(1_500.0, d, 0.0)
    }

    @Test
    fun `scope item menang atas scope brand`() {
        val discounts = listOf(
            disc("brandBig", "brand", "A", "amount", 5_000.0),
            disc("itemSmall", "item", "p1", "amount", 1_000.0),
        )
        val (d, _) = compute("p1", "A", 10_000.0, 1.0, 0.0, discounts)
        // Item-scope dipilih walau brand-scope nominalnya lebih besar.
        assertEquals(1_000.0, d, 0.0)
    }

    @Test
    fun `beberapa diskon item ambil nominal terbesar`() {
        val discounts = listOf(
            disc("a", "item", "p1", "amount", 1_000.0),
            disc("b", "item", "p1", "percent", 20.0), // 20% dari 10.000 = 2.000
        )
        val (d, _) = compute("p1", "A", 10_000.0, 1.0, 0.0, discounts)
        assertEquals(2_000.0, d, 0.0)
    }

    @Test
    fun `diskon di hari tak cocok diabaikan`() {
        val (d, periodic) = compute(
            "p1", "A", 10_000.0, 1.0, 0.0,
            listOf(disc("x", "item", "p1", "amount", 3_000.0, days = "sel,rab")),
            dayKey = "sen",
        )
        assertEquals(0.0, d, 0.0)
        assertFalse(periodic)
    }

    @Test
    fun `diskon nonaktif diabaikan`() {
        val (d, _) = compute(
            "p1", "A", 10_000.0, 1.0, 0.0,
            listOf(disc("x", "item", "p1", "amount", 3_000.0, active = false)),
        )
        assertEquals(0.0, d, 0.0)
    }

    @Test
    fun `diskon diklamp tidak melebihi price x qty`() {
        val (d, _) = compute("p1", "A", 10_000.0, 1.0, 0.0, listOf(disc("x", "item", "p1", "amount", 999_999.0)))
        assertEquals(10_000.0, d, 0.0)
    }

    @Test
    fun `everyday cocok di hari apa pun`() {
        assertTrue(DiscountEngine.dayMatches("everyday", "min"))
        assertTrue(DiscountEngine.dayMatches("sen,rab,jum", "rab"))
        assertFalse(DiscountEngine.dayMatches("sen,rab", "sel"))
    }

    @Test
    fun `add ke cart menolak melebihi stok`() {
        val p = ProductWithStock(
            id = "p1", name = "Kopi", sell_price = 5_000.0, cost_price = 3_000.0, stock_qty = 2.0,
        )
        val r1 = Cart.add(emptyList(), p, 2.0, emptyList(), "sen")
        assertEquals(1, r1.cart.size)
        val r2 = Cart.add(r1.cart, p, 1.0, emptyList(), "sen") // 2 + 1 > 2
        assertEquals(2.0, r2.cart[0].qty, 0.0) // tidak berubah
        assertEquals("Kopi", r2.stockError?.name)
    }

    @Test
    fun `add barang sama menambah qty dan hitung ulang diskon`() {
        val p = ProductWithStock(
            id = "p1", name = "Kopi", brand = "A", sell_price = 10_000.0, cost_price = 5_000.0,
            default_discount = 0.0, stock_qty = 10.0,
        )
        val discounts = listOf(disc("x", "item", "p1", "amount", 1_000.0))
        val r1 = Cart.add(emptyList(), p, 1.0, discounts, "sen")
        assertEquals(1_000.0, r1.cart[0].discount, 0.0)
        val r2 = Cart.add(r1.cart, p, 2.0, discounts, "sen")
        assertEquals(3.0, r2.cart[0].qty, 0.0)
        assertEquals(3_000.0, r2.cart[0].discount, 0.0) // 1.000 x 3
    }

    @Test
    fun `manual override tidak dihitung ulang saat qty berubah`() {
        val p = ProductWithStock(
            id = "p1", name = "Kopi", brand = "A", sell_price = 10_000.0, cost_price = 5_000.0, stock_qty = 10.0,
        )
        val discounts = listOf(disc("x", "item", "p1", "amount", 1_000.0))
        var cart = Cart.add(emptyList(), p, 1.0, discounts, "sen").cart
        cart = Cart.setManualDiscount(cart, "p1", 500.0)
        assertTrue(cart[0].manualOverride)
        val r = Cart.setQty(cart, "p1", 4.0, discounts, "sen")
        // Diskon manual tidak di-recompute jadi 4.000; tetap 500 (di-clamp saja).
        assertEquals(500.0, r.cart[0].discount, 0.0)
    }

    @Test
    fun `totals subtotal diskon dan total benar`() {
        val p1 = ProductWithStock(id = "p1", name = "A", sell_price = 10_000.0, cost_price = 0.0, stock_qty = 100.0)
        val p2 = ProductWithStock(id = "p2", name = "B", sell_price = 5_000.0, cost_price = 0.0, stock_qty = 100.0)
        var cart = Cart.add(emptyList(), p1, 2.0, emptyList(), "sen").cart
        cart = Cart.add(cart, p2, 3.0, emptyList(), "sen").cart
        cart = Cart.setManualDiscount(cart, "p1", 2_000.0)
        assertEquals(35_000.0, Cart.subtotal(cart), 0.0) // 20.000 + 15.000
        assertEquals(2_000.0, Cart.totalDiscount(cart), 0.0)
        assertEquals(33_000.0, Cart.total(cart), 0.0)
        assertEquals(5.0, Cart.totalQty(cart), 0.0)
    }
}
