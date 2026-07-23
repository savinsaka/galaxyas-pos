package com.galaxyas.mobilepos.printer

import com.galaxyas.mobilepos.data.model.StockMovementBatchDetail
import com.galaxyas.mobilepos.data.model.TransactionDetail
import com.galaxyas.mobilepos.util.parseIso
import java.io.ByteArrayOutputStream
import java.math.BigDecimal
import java.math.RoundingMode
import java.text.Normalizer
import java.text.NumberFormat
import java.text.SimpleDateFormat
import java.util.Locale
import kotlin.math.floor
import kotlin.math.max
import kotlin.math.min

/**
 * Port line-for-line dari desktop/src/lib/escpos.ts. Byte hasilnya harus IDENTIK
 * dengan implementasi TS — dijaga oleh EscPosGoldenTest terhadap fixture di
 * app/src/test/resources/fixtures (berkas .hex). Jangan ubah tanpa memperbarui golden.
 */

private const val ESC = 0x1b
private const val GS = 0x1d

private val idLocale: Locale = Locale.forLanguageTag("id-ID")
private val idInteger: NumberFormat = NumberFormat.getIntegerInstance(idLocale)

/** "Rp157.500" — money() escpos.ts: "Rp" + Math.round(n).toLocaleString("id-ID"). */
private fun money(n: Double): String {
    val rounded = BigDecimal(n).setScale(0, RoundingMode.HALF_UP).toLong()
    return "Rp" + idInteger.format(rounded)
}

/** JS `${n}` template-literal: 3.0 -> "3", 2.5 -> "2.5" (dipakai baris item struk). */
private fun jsNum(n: Double): String {
    if (n == floor(n) && !n.isInfinite()) return n.toLong().toString()
    return BigDecimal.valueOf(n).stripTrailingZeros().toPlainString()
}

/** formatQty escpos.ts: Number.isInteger ? n.toString() : n.toFixed(2) (dipakai dok stok). */
private fun escFormatQty(n: Double): String =
    if (n == floor(n) && !n.isInfinite()) n.toLong().toString()
    else String.format(Locale.US, "%.2f", n)

/**
 * Padanan `new Date(iso).toLocaleString("id-ID")` di Node — "23/7/2026, 10.30.00"
 * (hari/bulan tanpa nol depan, jam 24h dengan pemisah titik). Pola EKSPLISIT,
 * bukan locale device, supaya byte struk identik lintas HP.
 */
private val idDateTimeFmt = SimpleDateFormat("d/M/yyyy, HH.mm.ss", Locale.US)
private fun idDateTime(iso: String): String {
    val d = parseIso(iso) ?: return iso
    return idDateTimeFmt.format(d)
}

/** Printer thermal umumnya CP437/ASCII: buang diakritik, sisanya non-ASCII -> '?'. */
private fun asciiBytes(s: String): List<Int> {
    val norm = Normalizer.normalize(s, Normalizer.Form.NFKD)
        .replace(Regex("[\\u0300-\\u036f]"), "")
    val out = ArrayList<Int>(norm.length)
    var i = 0
    while (i < norm.length) {
        val cp = norm.codePointAt(i)
        out.add(if (cp < 128) cp else 63)
        i += Character.charCount(cp)
    }
    return out
}

private class EscPosBuilder {
    private val bytes = ByteArrayOutputStream()

    fun raw(vararg b: Int): EscPosBuilder {
        for (x in b) bytes.write(x and 0xff)
        return this
    }

    fun text(s: String): EscPosBuilder {
        for (b in asciiBytes(s)) bytes.write(b and 0xff)
        return this
    }

    fun line(s: String = ""): EscPosBuilder = text(s).raw(0x0a)

    fun init(): EscPosBuilder = raw(ESC, 0x40)

    fun align(a: String): EscPosBuilder {
        val n = when (a) {
            "center" -> 1
            "right" -> 2
            else -> 0
        }
        return raw(ESC, 0x61, n)
    }

    fun bold(on: Boolean): EscPosBuilder = raw(ESC, 0x45, if (on) 1 else 0)

    fun doubleHeight(on: Boolean): EscPosBuilder = raw(GS, 0x21, if (on) 0x01 else 0x00)

    fun feed(lines: Int): EscPosBuilder = raw(ESC, 0x64, max(0, min(255, lines)))

    fun cut(partial: Boolean = true): EscPosBuilder = raw(GS, 0x56, if (partial) 1 else 0)

    fun build(): ByteArray = bytes.toByteArray()
}

private fun repeatChar(ch: Char, w: Int): String = ch.toString().repeat(w)

/** two(l, r): kolom kiri + kanan flush ke lebar w (padding spasi di tengah). */
private fun two(l: String, r: String, w: Int): String {
    val left = l.substring(0, max(0, min(l.length, w - r.length - 1)))
    val space = max(1, w - left.length - r.length)
    return left + " ".repeat(space) + r
}

private fun splitNonEmpty(s: String): List<String> =
    s.split("\n").map { it.trim() }.filter { it.isNotEmpty() }

/** Byte ESC/POS untuk struk transaksi, diakhiri feed + autocut. */
fun buildReceiptEscPos(detail: TransactionDetail, cfg: ReceiptConfig): ByteArray {
    val w = paperCols(cfg.paper)
    val show = cfg.show
    val b = EscPosBuilder()
    b.init()
    b.align("center")

    if (show.storeName) b.bold(true).doubleHeight(true).line(cfg.storeName).doubleHeight(false).bold(false)
    if (show.address && cfg.address.trim().isNotEmpty()) splitNonEmpty(cfg.address).forEach { b.line(it) }
    if (show.phone && cfg.phone.trim().isNotEmpty()) b.line(cfg.phone.trim())
    if (show.taxId && cfg.taxId.trim().isNotEmpty()) b.line("NPWP: ${cfg.taxId.trim()}")
    if (show.social) {
        val social = listOfNotNull(
            cfg.instagram.trim().ifEmpty { null }?.let { "IG: $it" },
            cfg.tiktok.trim().ifEmpty { null }?.let { "TikTok: $it" },
            cfg.whatsapp.trim().ifEmpty { null }?.let { "WA: $it" },
        ).joinToString(" · ")
        if (social.isNotEmpty()) b.line(social)
    }
    if (show.header) splitNonEmpty(cfg.header).forEach { b.line(it) }
    if (show.date) b.line(idDateTime(detail.created_at))
    if (show.invoiceNo) b.line(detail.invoice_no)

    if (show.items) {
        b.align("left").line(repeatChar('-', w))
        for (it in detail.items) {
            b.line(it.name.take(w))
            b.line(two("  ${jsNum(it.qty)} x ${money(it.price)}", money(it.price * it.qty), w))
            if (it.discount > 0) b.line(two("  Diskon", "-" + money(it.discount), w))
        }
    }

    val hasSummary = show.subtotal || show.discount || show.total || show.paymentMethod || show.change
    if (hasSummary) {
        b.align("left").line(repeatChar('-', w))
        if (show.subtotal) b.line(two("Subtotal", money(detail.subtotal), w))
        if (show.discount) b.line(two("Diskon", "-" + money(detail.discount), w))
        if (show.total) b.bold(true).line(two("TOTAL", money(detail.total), w)).bold(false)
        if (show.paymentMethod) {
            if (detail.payment_method == "Kombinasi") {
                b.line(two("Tunai", money(detail.paid_cash ?: 0.0), w))
                b.line(two("QRIS", money(detail.paid_qris ?: 0.0), w))
            } else {
                b.line(two(detail.payment_method, money(detail.paid), w))
            }
        }
        if (show.change) b.line(two("Kembali", money(detail.change), w))
        b.line(repeatChar('-', w))
    }

    if (show.footer && cfg.footer.trim().isNotEmpty()) {
        b.align("center")
        splitNonEmpty(cfg.footer).forEach { b.line(it) }
    }

    b.feed(3)
    b.cut(true)
    return b.build()
}

/** Byte ESC/POS untuk dokumen Item Masuk/Keluar. */
fun buildStockDocEscPos(detail: StockMovementBatchDetail, cfg: ReceiptConfig): ByteArray {
    val w = paperCols(cfg.paper)
    val show = cfg.show
    val verb = if (detail.kind == "in") "ITEM MASUK" else "ITEM KELUAR"
    val b = EscPosBuilder()
    b.init()
    b.align("center")

    if (show.storeName && cfg.storeName.trim().isNotEmpty()) {
        b.bold(true).doubleHeight(true).line(cfg.storeName).doubleHeight(false).bold(false)
    }
    b.bold(true).line(verb).bold(false)
    b.line(detail.no)
    b.line(idDateTime(detail.created_at))

    b.align("left").line(repeatChar('-', w))
    for (it in detail.items) {
        b.line(it.product_name.take(w))
        b.line(two("  Qty: ${escFormatQty(it.qty)}", "", w))
        if (!it.note.isNullOrEmpty()) b.line("  Ket: ${it.note}".take(w))
    }
    b.line(repeatChar('-', w))
    b.line(two("Total Item", detail.items.size.toString(), w))
    b.line(two("Total Qty", escFormatQty(detail.items.sumOf { it.qty }), w))
    if (!detail.note.isNullOrEmpty()) b.line("Catatan: ${detail.note}".take(w))
    if (!detail.user_id.isNullOrEmpty()) b.line("Oleh: ${detail.user_id}".take(w))
    b.line(repeatChar('-', w))

    if (show.footer && cfg.footer.trim().isNotEmpty()) {
        b.align("center")
        splitNonEmpty(cfg.footer).forEach { b.line(it) }
    }

    b.feed(2)
    b.cut(true)
    return b.build()
}

// ---------- Laporan sebagai struk ----------

data class ReportEscPosRow(val cells: List<String>, val bold: Boolean = false)
data class ReportEscPosSection(
    val heading: String? = null,
    val columns: List<String>? = null,
    val rows: List<ReportEscPosRow>,
)
data class ReportEscPosDoc(
    val title: String,
    val subtitle: String? = null,
    val meta: String? = null,
    val sections: List<ReportEscPosSection>,
)

fun buildReportEscPos(doc: ReportEscPosDoc, cfg: ReceiptConfig): ByteArray {
    val w = paperCols(cfg.paper)
    val b = EscPosBuilder()
    b.init()
    b.align("center")
    b.bold(true).doubleHeight(true).line(doc.title).doubleHeight(false).bold(false)
    doc.subtitle?.let { b.line(it) }
    doc.meta?.let { b.line(it) }

    for (sec in doc.sections) {
        b.align("left").line(repeatChar('-', w))
        sec.heading?.let { b.bold(true).line(it.take(w)).bold(false) }

        val wide = (sec.columns?.size ?: 0) > 2
        if (wide) {
            for (row in sec.rows) {
                val first = row.cells.firstOrNull() ?: ""
                b.bold(row.bold).line(first.take(w)).bold(false)
                row.cells.drop(1).forEachIndexed { i, value ->
                    val header = sec.columns!!.getOrNull(i + 1) ?: ""
                    if (value.isNotEmpty()) b.line(two("  $header", value, w))
                }
            }
        } else {
            for (row in sec.rows) {
                val label = row.cells.getOrNull(0) ?: ""
                val value = row.cells.getOrNull(1) ?: ""
                b.bold(row.bold)
                b.line(two(label, value, w))
                b.bold(false)
            }
        }
    }
    b.align("left").line(repeatChar('-', w))

    b.align("center")
    b.feed(3)
    b.cut(true)
    return b.build()
}
