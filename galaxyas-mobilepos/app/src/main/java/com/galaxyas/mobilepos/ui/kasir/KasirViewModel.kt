package com.galaxyas.mobilepos.ui.kasir

import android.content.Context
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.galaxyas.mobilepos.data.PendingSale
import com.galaxyas.mobilepos.data.PendingSalesStore
import com.galaxyas.mobilepos.data.Session
import com.galaxyas.mobilepos.data.SettingsRepository
import com.galaxyas.mobilepos.data.model.Customer
import com.galaxyas.mobilepos.data.model.DiscountPeriod
import com.galaxyas.mobilepos.data.model.OpenShiftInput
import com.galaxyas.mobilepos.data.model.ProductWithStock
import com.galaxyas.mobilepos.data.model.SaleInput
import com.galaxyas.mobilepos.data.model.SaleItemInput
import com.galaxyas.mobilepos.data.model.Shift
import com.galaxyas.mobilepos.data.model.TransactionDetail
import com.galaxyas.mobilepos.data.network.ApiClient
import com.galaxyas.mobilepos.printer.BtPrinter
import com.galaxyas.mobilepos.printer.buildReceiptEscPos
import com.galaxyas.mobilepos.printer.parseReceiptConfig
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class KasirState(
    val shiftChecked: Boolean = false,
    val activeShift: Shift? = null,
    val openingCashPrefill: Double = 0.0,
    val cart: List<CartLine> = emptyList(),
    val discounts: List<DiscountPeriod> = emptyList(),
    val customers: List<Customer> = emptyList(),
    val selectedCustomer: Customer? = null,
    val paymentMethod: String = "Tunai",
    val paid: Double = 0.0,
    val paidCash: Double = 0.0,
    val paidQris: Double = 0.0,
    val busy: Boolean = false,
    val stockError: Cart.StockError? = null,
    val warning: String? = null,
    val lastReceipt: TransactionDetail? = null,
    val toast: String? = null,
) {
    val subtotal get() = Cart.subtotal(cart)
    val totalDiscount get() = Cart.totalDiscount(cart)
    val total get() = Cart.total(cart)
    val totalQty get() = Cart.totalQty(cart)
    val change get() = (paid - total).coerceAtLeast(0.0)
}

class KasirViewModel(
    private val api: ApiClient,
    private val session: Session,
    private val settings: SettingsRepository,
    private val pendingStore: PendingSalesStore,
    private val appContext: Context,
) : ViewModel() {

    private val _state = MutableStateFlow(KasirState())
    val state: StateFlow<KasirState> = _state.asStateFlow()
    val pending get() = pendingStore.pending

    private fun dayKey() = DiscountEngine.todayKey()

    fun load() {
        viewModelScope.launch {
            try {
                val shift = api.getActiveShift()
                var prefill = 0.0
                if (shift == null) {
                    prefill = api.listShifts(1).firstOrNull()?.closing_cash ?: 0.0
                }
                val discounts = runCatching { api.listDiscounts() }.getOrDefault(emptyList())
                val customers = runCatching { api.listCustomers() }.getOrDefault(emptyList())
                _state.value = _state.value.copy(
                    shiftChecked = true, activeShift = shift, openingCashPrefill = prefill,
                    discounts = discounts, customers = customers,
                )
            } catch (e: Exception) {
                _state.value = _state.value.copy(shiftChecked = true, warning = e.message)
            }
        }
    }

    fun openShift(openingCash: Double) {
        val user = session.user.value ?: return
        _state.value = _state.value.copy(busy = true)
        viewModelScope.launch {
            try {
                val shift = api.openShift(OpenShiftInput(user.username, user.name, openingCash))
                _state.value = _state.value.copy(activeShift = shift, busy = false, toast = "Shift dibuka. Selamat berjualan!")
            } catch (e: Exception) {
                _state.value = _state.value.copy(busy = false, warning = e.message)
            }
        }
    }

    // --- Keranjang ---

    fun addProduct(p: ProductWithStock, qty: Double = 1.0) {
        val r = Cart.add(_state.value.cart, p, qty, _state.value.discounts, dayKey())
        _state.value = _state.value.copy(cart = r.cart, stockError = r.stockError)
    }

    /** Cari barcode di Server Pusat lalu tambahkan (scan kasir). */
    fun scanBarcode(barcode: String) {
        viewModelScope.launch {
            try {
                val p = api.findByBarcode(barcode)
                if (p == null) {
                    _state.value = _state.value.copy(warning = "Barcode \"$barcode\" tidak ditemukan.")
                } else {
                    addProduct(p, 1.0)
                }
            } catch (e: Exception) {
                _state.value = _state.value.copy(warning = e.message)
            }
        }
    }

    fun setQty(productId: String, qty: Double) {
        val r = Cart.setQty(_state.value.cart, productId, qty, _state.value.discounts, dayKey())
        _state.value = _state.value.copy(cart = r.cart, stockError = r.stockError)
    }

    fun setManualDiscount(productId: String, discount: Double) {
        _state.value = _state.value.copy(cart = Cart.setManualDiscount(_state.value.cart, productId, discount))
    }

    fun removeLine(productId: String) {
        _state.value = _state.value.copy(cart = Cart.remove(_state.value.cart, productId))
    }

    fun clearCart() {
        _state.value = _state.value.copy(cart = emptyList(), paid = 0.0, paidCash = 0.0, paidQris = 0.0)
    }

    // --- Pembayaran ---

    fun selectPayment(method: String) {
        val s = _state.value
        _state.value = when (method) {
            "QRIS" -> s.copy(paymentMethod = method, paid = s.total)
            "Kombinasi" -> s.copy(paymentMethod = method, paid = 0.0, paidCash = 0.0, paidQris = 0.0)
            else -> s.copy(paymentMethod = method)
        }
    }

    fun setPaid(n: Double) { _state.value = _state.value.copy(paid = n) }
    fun setPaidCash(n: Double) {
        val s = _state.value; _state.value = s.copy(paidCash = n, paid = n + s.paidQris)
    }
    fun setPaidQris(n: Double) {
        val s = _state.value; _state.value = s.copy(paidQris = n, paid = s.paidCash + n)
    }

    fun setCustomer(c: Customer?) { _state.value = _state.value.copy(selectedCustomer = c) }

    fun clearFlags() {
        _state.value = _state.value.copy(stockError = null, warning = null, toast = null)
    }

    fun dismissReceipt() { _state.value = _state.value.copy(lastReceipt = null) }

    // --- Checkout ---

    fun checkout() {
        val s = _state.value
        when {
            s.cart.isEmpty() -> { _state.value = s.copy(warning = "Keranjang kosong."); return }
            s.paid < s.total -> { _state.value = s.copy(warning = "Pembayaran kurang dari total."); return }
            s.activeShift == null -> { _state.value = s.copy(warning = "Buka shift terlebih dahulu."); return }
        }
        _state.value = s.copy(busy = true)
        viewModelScope.launch {
            try {
                val user = session.user.value
                val sale = SaleInput(
                    cashier_id = user?.username ?: "admin",
                    payment_method = s.paymentMethod,
                    paid = s.paid,
                    items = s.cart.map { SaleItemInput(it.product_id, it.name, it.price, it.qty, it.discount) },
                    customer_id = s.selectedCustomer?.id,
                    shift_id = s.activeShift!!.id,
                    paid_cash = if (s.paymentMethod == "Kombinasi") s.paidCash else null,
                    paid_qris = if (s.paymentMethod == "Kombinasi") s.paidQris else null,
                )
                val tx = api.checkout(sale)
                _state.value = _state.value.copy(
                    busy = false, lastReceipt = tx, cart = emptyList(),
                    paid = 0.0, paidCash = 0.0, paidQris = 0.0,
                    selectedCustomer = null, paymentMethod = "Tunai",
                    toast = "Transaksi ${tx.invoice_no} tersimpan.",
                )
            } catch (e: Exception) {
                // Cart DIPERTAHANKAN saat gagal — hindari dobel jual bila koneksi putus
                // setelah server sebenarnya sudah menyimpan (cek riwayat dulu).
                _state.value = _state.value.copy(busy = false, warning = e.message)
            }
        }
    }

    // --- Cetak struk ---

    /** Cetak struk transaksi terakhir ke printer Bluetooth. */
    fun printLastReceipt(onResult: (Boolean, String) -> Unit) {
        val tx = _state.value.lastReceipt ?: return
        val cfg = parseReceiptConfig(settings.settings.value)
        val mac = cfg.printer
        if (mac.isNullOrBlank()) {
            onResult(false, "Printer belum dipilih. Atur di Menu → Pengaturan → Printer.")
            return
        }
        viewModelScope.launch {
            val bytes = buildReceiptEscPos(tx, cfg)
            val result = BtPrinter.print(appContext, mac, bytes)
            result.fold(
                onSuccess = { onResult(true, "Struk dikirim ke printer.") },
                onFailure = { onResult(false, it.message ?: "Gagal mencetak.") },
            )
        }
    }

    // --- Pending (hold/resume) ---

    fun holdCart() {
        val s = _state.value
        if (s.cart.isEmpty()) return
        val label = s.selectedCustomer?.name ?: "Pending ${pendingStore.pending.value.size + 1}"
        viewModelScope.launch {
            pendingStore.add(
                PendingSale(
                    label = label, cart = s.cart, customerId = s.selectedCustomer?.id,
                    paymentMethod = s.paymentMethod, paid = s.paid, paidCash = s.paidCash, paidQris = s.paidQris,
                ),
            )
            _state.value = _state.value.copy(
                cart = emptyList(), paid = 0.0, paidCash = 0.0, paidQris = 0.0,
                selectedCustomer = null, toast = "Transaksi disimpan sebagai pending.",
            )
        }
    }

    fun resumePending(id: String) {
        val p = pendingStore.pending.value.firstOrNull { it.id == id } ?: return
        val cust = p.customerId?.let { cid -> _state.value.customers.firstOrNull { it.id == cid } }
        _state.value = _state.value.copy(
            cart = p.cart, selectedCustomer = cust, paymentMethod = p.paymentMethod,
            paid = p.paid, paidCash = p.paidCash, paidQris = p.paidQris,
        )
        viewModelScope.launch { pendingStore.remove(id) }
    }

    fun deletePending(id: String) {
        viewModelScope.launch { pendingStore.remove(id) }
    }
}
