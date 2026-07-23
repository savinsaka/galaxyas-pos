package com.galaxyas.mobilepos.printer

/**
 * Port line-for-line dari desktop/src/lib/receipt.ts. Key setting SAMA dengan
 * desktop sehingga map dari SettingsRepository langsung dipakai.
 */

data class ReceiptShowFlags(
    val storeName: Boolean,
    val address: Boolean,
    val phone: Boolean,
    val taxId: Boolean,
    val social: Boolean,
    val header: Boolean,
    val invoiceNo: Boolean,
    val date: Boolean,
    val items: Boolean,
    val subtotal: Boolean,
    val discount: Boolean,
    val total: Boolean,
    val paymentMethod: Boolean,
    val change: Boolean,
    val footer: Boolean,
)

data class ReceiptConfig(
    val storeName: String,
    val address: String,
    val phone: String,
    val taxId: String,
    val instagram: String,
    val tiktok: String,
    val whatsapp: String,
    val header: String,
    val footer: String,
    val paper: String, // "58" | "80"
    val printer: String?, // MAC printer Bluetooth
    val show: ReceiptShowFlags,
)

/** (key setting, label) untuk toggle blok struk di layar Pengaturan. */
val RECEIPT_SHOW_KEYS: List<Triple<String, String, (ReceiptShowFlags) -> Boolean>> = listOf(
    Triple("receipt_show_store_name", "Nama Toko", ReceiptShowFlags::storeName),
    Triple("receipt_show_address", "Alamat", ReceiptShowFlags::address),
    Triple("receipt_show_phone", "Telepon", ReceiptShowFlags::phone),
    Triple("receipt_show_tax_id", "NPWP", ReceiptShowFlags::taxId),
    Triple("receipt_show_social", "Sosial Media", ReceiptShowFlags::social),
    Triple("receipt_show_header", "Header Tambahan", ReceiptShowFlags::header),
    Triple("receipt_show_invoice_no", "No. Invoice", ReceiptShowFlags::invoiceNo),
    Triple("receipt_show_date", "Tanggal", ReceiptShowFlags::date),
    Triple("receipt_show_items", "Daftar Item", ReceiptShowFlags::items),
    Triple("receipt_show_subtotal", "Subtotal", ReceiptShowFlags::subtotal),
    Triple("receipt_show_discount", "Diskon", ReceiptShowFlags::discount),
    Triple("receipt_show_total", "Total", ReceiptShowFlags::total),
    Triple("receipt_show_payment_method", "Metode Bayar", ReceiptShowFlags::paymentMethod),
    Triple("receipt_show_change", "Kembalian", ReceiptShowFlags::change),
    Triple("receipt_show_footer", "Footer", ReceiptShowFlags::footer),
)

private fun boolSetting(s: Map<String, String>, key: String, def: Boolean = true): Boolean {
    val v = s[key]
    if (v.isNullOrEmpty()) return def
    return v != "0"
}

fun parseReceiptConfig(s: Map<String, String>): ReceiptConfig {
    val paper = if (s["receipt_paper"] == "58") "58" else "80"
    val show = ReceiptShowFlags(
        storeName = boolSetting(s, "receipt_show_store_name"),
        address = boolSetting(s, "receipt_show_address"),
        phone = boolSetting(s, "receipt_show_phone"),
        taxId = boolSetting(s, "receipt_show_tax_id"),
        social = boolSetting(s, "receipt_show_social"),
        header = boolSetting(s, "receipt_show_header"),
        invoiceNo = boolSetting(s, "receipt_show_invoice_no"),
        date = boolSetting(s, "receipt_show_date"),
        items = boolSetting(s, "receipt_show_items"),
        subtotal = boolSetting(s, "receipt_show_subtotal"),
        discount = boolSetting(s, "receipt_show_discount"),
        total = boolSetting(s, "receipt_show_total"),
        paymentMethod = boolSetting(s, "receipt_show_payment_method"),
        change = boolSetting(s, "receipt_show_change"),
        footer = boolSetting(s, "receipt_show_footer"),
    )
    return ReceiptConfig(
        storeName = s["store_name"].orEmptyOr("GALAXYAS POS"),
        address = s["store_address"] ?: "",
        phone = s["store_phone"] ?: "",
        taxId = s["store_tax_id"] ?: "",
        instagram = s["store_instagram"] ?: "",
        tiktok = s["store_tiktok"] ?: "",
        whatsapp = s["store_whatsapp"] ?: "",
        header = s["receipt_header"] ?: "",
        footer = s["receipt_footer"] ?: "",
        paper = paper,
        printer = s["receipt_printer"]?.ifBlank { null },
        show = show,
    )
}

private fun String?.orEmptyOr(fallback: String): String =
    if (this.isNullOrEmpty()) fallback else this

/** Perkiraan jumlah karakter monospace per baris untuk kertas thermal. */
fun paperCols(paper: String): Int = if (paper == "58") 32 else 48
