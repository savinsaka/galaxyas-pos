package com.galaxyas.mobilepos.util

import java.text.NumberFormat
import java.text.SimpleDateFormat
import java.util.Locale

private val localeId: Locale = Locale.forLanguageTag("id-ID")

private val rupiah: NumberFormat = NumberFormat.getCurrencyInstance(localeId).apply {
    minimumFractionDigits = 2
    maximumFractionDigits = 2
}

/** "Rp15.000,00" — padanan formatIDR desktop (Intl id-ID currency). */
fun formatIDR(n: Double): String = rupiah.format(n)

/** Qty tanpa desimal bila bulat, else 2 desimal — padanan formatQty desktop. */
fun formatQty(n: Double): String =
    if (n == Math.floor(n) && !n.isInfinite()) n.toLong().toString() else String.format(localeId, "%.2f", n)

/** "23 Jul 2026, 10.30.00" — padanan formatDateTime desktop (id-ID). */
fun formatDateTime(iso: String): String {
    if (iso.isBlank()) return "-"
    val date = parseIso(iso) ?: return iso
    return SimpleDateFormat("dd MMM yyyy, HH.mm.ss", localeId).format(date)
}

/**
 * Parse ISO-8601 lokal ("2026-07-23T10:30:00", boleh dengan sub-detik) —
 * timestamp dari Server Pusat tanpa zona (waktu lokal PC toko).
 */
fun parseIso(iso: String): java.util.Date? {
    val cleaned = iso.substringBefore(".").replace("Z", "")
    return runCatching {
        SimpleDateFormat("yyyy-MM-dd'T'HH:mm:ss", Locale.US).parse(cleaned)
    }.getOrNull() ?: runCatching {
        SimpleDateFormat("yyyy-MM-dd", Locale.US).parse(cleaned)
    }.getOrNull()
}
