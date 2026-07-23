package com.galaxyas.mobilepos

import com.galaxyas.mobilepos.data.model.StockMovementBatchDetail
import com.galaxyas.mobilepos.data.model.StockMovementBatchItem
import com.galaxyas.mobilepos.data.model.TransactionDetail
import com.galaxyas.mobilepos.data.model.TransactionItem
import com.galaxyas.mobilepos.printer.ReceiptConfig
import com.galaxyas.mobilepos.printer.ReceiptShowFlags
import com.galaxyas.mobilepos.printer.ReportEscPosDoc
import com.galaxyas.mobilepos.printer.ReportEscPosRow
import com.galaxyas.mobilepos.printer.ReportEscPosSection
import com.galaxyas.mobilepos.printer.buildReportEscPos
import com.galaxyas.mobilepos.printer.buildReceiptEscPos
import com.galaxyas.mobilepos.printer.buildStockDocEscPos
import java.util.TimeZone
import org.junit.Assert.assertEquals
import org.junit.Before
import org.junit.Test

/**
 * Bukti paritas byte: byte ESC/POS dari port Kotlin harus IDENTIK dengan
 * implementasi TS (escpos.ts). Fixture .hex dihasilkan dari TS — lihat
 * PROTOCOL.md. Input di sini HARUS sama persis dengan scratchpad
 * gen-escpos-goldens.ts.
 */
class EscPosGoldenTest {

    @Before
    fun setUp() {
        // created_at tak berzona; samakan zona dengan Node saat golden dibuat
        // agar komponen jam struk konsisten. Node dijalankan di zona lokal mesin
        // (WIB); parse+format Kotlin memakai zona default proses test.
        TimeZone.setDefault(TimeZone.getTimeZone("Asia/Jakarta"))
    }

    private val fullShow = ReceiptShowFlags(
        storeName = true, address = true, phone = true, taxId = true, social = true,
        header = true, invoiceNo = true, date = true, items = true, subtotal = true,
        discount = true, total = true, paymentMethod = true, change = true, footer = true,
    )

    private val cfg58 = ReceiptConfig(
        storeName = "TOKO GALAXY JAYA",
        address = "Jl. Merdeka No. 123\nBandung",
        phone = "022-1234567",
        taxId = "01.234.567.8-901.000",
        instagram = "@galaxyjaya",
        tiktok = "@galaxytok",
        whatsapp = "0812-3456-7890",
        header = "Struk Penjualan",
        footer = "Terima kasih!\nBarang yang dibeli tidak dapat ditukar",
        paper = "58",
        printer = null,
        show = fullShow,
    )

    private val cfg80minimal = cfg58.copy(
        paper = "80",
        show = fullShow.copy(
            address = false, phone = false, taxId = false, social = false, header = false,
            subtotal = false, discount = false, change = false, footer = false,
        ),
    )

    private val trx = TransactionDetail(
        id = "trx-1",
        invoice_no = "INV-20260723-0001",
        cashier_id = "kasir1",
        subtotal = 157_500.0,
        discount = 7_500.0,
        total = 150_000.0,
        paid = 150_000.0,
        change = 0.0,
        payment_method = "Kombinasi",
        created_at = "2026-07-23T10:30:00",
        customer_id = null,
        shift_id = "shift-1",
        paid_cash = 100_000.0,
        paid_qris = 50_000.0,
        items = listOf(
            TransactionItem("p1", "Kopi Susu Gula Aren Botol 250ml", 15_000.0, 3.0, 4_500.0, 40_500.0),
            TransactionItem("p2", "Roti Bakar Coklat", 25_000.0, 2.5, 3_000.0, 59_500.0),
            TransactionItem("p3", "Air Mineral", 50_000.0, 1.0, 0.0, 50_000.0),
        ),
    )

    private val stockDoc = StockMovementBatchDetail(
        id = "batch-1",
        no = "IN-20260723-0002",
        kind = "in",
        note = "Restock mingguan",
        user_id = "admin",
        created_at = "2026-07-23T08:15:00",
        items = listOf(
            StockMovementBatchItem("p1", "Kopi Susu Gula Aren Botol 250ml", 24.0, "dus A"),
            StockMovementBatchItem("p2", "Roti Bakar Coklat", 10.5, null),
        ),
    )

    private val report = ReportEscPosDoc(
        title = "LAPORAN PENJUALAN",
        subtitle = "01/07/2026 - 23/07/2026",
        meta = "Semua merek",
        sections = listOf(
            ReportEscPosSection(
                heading = "Ringkasan",
                rows = listOf(
                    ReportEscPosRow(listOf("Omzet", "Rp12.345.678")),
                    ReportEscPosRow(listOf("Transaksi", "321")),
                    ReportEscPosRow(listOf("TOTAL", "Rp12.345.678"), bold = true),
                ),
            ),
            ReportEscPosSection(
                heading = "Per Barang",
                columns = listOf("Barang", "Qty", "Omzet"),
                rows = listOf(
                    ReportEscPosRow(listOf("Kopi Susu Gula Aren", "120", "Rp1.800.000")),
                    ReportEscPosRow(listOf("Roti Bakar Coklat", "80", "Rp2.000.000"), bold = true),
                ),
            ),
        ),
    )

    private fun golden(name: String): ByteArray {
        val hex = javaClass.classLoader!!
            .getResourceAsStream("fixtures/$name.hex")!!
            .bufferedReader().readText().trim()
        return hex.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    }

    private fun toHex(b: ByteArray) = b.joinToString("") { "%02x".format(it) }

    @Test
    fun `struk 58mm lengkap Kombinasi cocok golden`() {
        assertEquals(toHex(golden("receipt_58_full_kombinasi")), toHex(buildReceiptEscPos(trx, cfg58)))
    }

    @Test
    fun `struk 80mm minimal cocok golden`() {
        val trx80 = trx.copy(payment_method = "Tunai", paid = 200_000.0, change = 50_000.0)
        assertEquals(toHex(golden("receipt_80_minimal")), toHex(buildReceiptEscPos(trx80, cfg80minimal)))
    }

    @Test
    fun `dokumen stok masuk cocok golden`() {
        assertEquals(toHex(golden("stockdoc_58_in")), toHex(buildStockDocEscPos(stockDoc, cfg58)))
    }

    @Test
    fun `laporan sebagai struk cocok golden`() {
        assertEquals(toHex(golden("report_80")), toHex(buildReportEscPos(report, cfg58.copy(paper = "80"))))
    }
}
