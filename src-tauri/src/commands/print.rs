use serde::Deserialize;
use tauri::State;

use crate::db::models::{PrinterSettings, Sale, SaleItem, Store};
use crate::error::AppResult;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ReceiptData {
    pub store: Store,
    pub printer: PrinterSettings,
    pub sale: Sale,
    pub items: Vec<SaleItem>,
}

/// Render an ESC/POS receipt as raw bytes and send it to the configured
/// printer. The raw byte rendering is platform agnostic; the actual spooling is
/// stubbed here (would use `escpos` / a Windows raw-print call in production).
#[tauri::command]
pub fn print_receipt(state: State<AppState>, data: ReceiptData) -> AppResult<()> {
    state.require_session()?;
    let bytes = render_escpos(&data);
    tracing::info!(
        invoice = data.sale.invoice_no.as_deref().unwrap_or("-"),
        bytes = bytes.len(),
        printer = %data.printer.printer_name,
        "print_receipt"
    );
    // TODO: spool `bytes` to the OS printer queue (raw mode).
    spool_to_printer(&data.printer.printer_name, &bytes)?;
    Ok(())
}

fn spool_to_printer(_printer: &str, _bytes: &[u8]) -> AppResult<()> {
    // Platform-specific raw spooling would go here (e.g. winspool on Windows).
    Ok(())
}

const ESC: u8 = 0x1B;
const GS: u8 = 0x1D;

/// Number of printable columns for the given paper width in mm.
fn cols_for_paper(paper_width: i64) -> usize {
    if paper_width >= 80 { 48 } else { 32 }
}

/// Format a number with dot-separated thousands (Indonesian style).
fn fmt_rp(v: f64) -> String {
    let n = v as i64;
    let neg = n < 0;
    let mut s = n.unsigned_abs().to_string();
    let len = s.len();
    if len > 3 {
        let mut with_sep = String::new();
        for (i, c) in s.chars().enumerate() {
            if i > 0 && (len - i) % 3 == 0 {
                with_sep.push('.');
            }
            with_sep.push(c);
        }
        s = with_sep;
    }
    if neg { format!("-{s}") } else { s }
}

/// Right-align a label + value pair within the given column width.
fn label_val(cols: usize, label: &str, val: &str) -> String {
    let val_space = cols.saturating_sub(label.len());
    format!("{label}{val:>width$}", width = val_space)
}

/// Format an epoch-ms timestamp as "dd/mm/yyyy HH:MM".
fn fmt_datetime(epoch_ms: i64) -> String {
    use chrono::{TimeZone, Utc, FixedOffset};
    // Use WIB (UTC+7) – adjust if multi-timezone support is needed later.
    let tz = FixedOffset::east_opt(7 * 3600).unwrap();
    match tz.timestamp_millis_opt(epoch_ms) {
        chrono::LocalResult::Single(dt) => dt.format("%d/%m/%Y %H:%M").to_string(),
        _ => Utc.timestamp_millis_opt(epoch_ms)
            .single()
            .map(|d| d.format("%d/%m/%Y %H:%M").to_string())
            .unwrap_or_default(),
    }
}

fn render_escpos(data: &ReceiptData) -> Vec<u8> {
    let cols = cols_for_paper(data.printer.paper_width);
    let mut out = Vec::new();

    // ── Initialise printer ──
    out.extend_from_slice(&[ESC, b'@']); // reset

    // ── Header (centered) ──
    out.extend_from_slice(&[ESC, b'a', 1]); // center align
    // Store name – bold + double-height
    out.extend_from_slice(&[ESC, b'E', 1]); // bold on
    out.extend_from_slice(&[GS, b'!', 0x01]); // double height
    line(&mut out, &data.printer.header_text);
    out.extend_from_slice(&[GS, b'!', 0x00]); // normal size
    out.extend_from_slice(&[ESC, b'E', 0]); // bold off
    if let Some(addr) = &data.store.address {
        line(&mut out, addr);
    }
    if let Some(phone) = &data.store.phone {
        line(&mut out, phone);
    }
    out.extend_from_slice(&[ESC, b'a', 0]); // left align

    // ── Invoice info ──
    sep(&mut out, cols);
    if let Some(inv) = &data.sale.invoice_no {
        line(&mut out, &format!("No: {inv}"));
    }
    line(&mut out, &fmt_datetime(data.sale.created_at));
    sep(&mut out, cols);

    // ── Items ──
    for it in &data.items {
        line(&mut out, &it.nama_item);
        let detail = format!(
            "  {} x {}",
            it.qty as i64,
            fmt_rp(it.harga),
        );
        let subtotal_str = fmt_rp(it.subtotal);
        let padded = label_val(cols, &detail, &subtotal_str);
        line(&mut out, &padded);
    }
    sep(&mut out, cols);

    // ── Totals ──
    line(&mut out, &label_val(cols, "Subtotal", &fmt_rp(data.sale.subtotal)));
    line(&mut out, &label_val(cols, "Diskon", &fmt_rp(data.sale.diskon)));
    line(&mut out, &label_val(cols, "Pajak", &fmt_rp(data.sale.pajak)));

    // TOTAL line – bold
    out.extend_from_slice(&[ESC, b'E', 1]);
    line(&mut out, &label_val(cols, "TOTAL", &fmt_rp(data.sale.total)));
    out.extend_from_slice(&[ESC, b'E', 0]);

    line(&mut out, &label_val(cols, "Bayar", &fmt_rp(data.sale.bayar)));
    line(&mut out, &label_val(cols, "Kembali", &fmt_rp(data.sale.kembali)));
    sep(&mut out, cols);

    // ── Footer (centered) ──
    out.extend_from_slice(&[ESC, b'a', 1]); // center align
    line(&mut out, &data.printer.footer_text);
    out.extend_from_slice(&[ESC, b'a', 0]); // reset to left

    // ── Feed & cut ──
    out.extend_from_slice(&[ESC, b'd', 4]); // feed 4 lines
    out.extend_from_slice(&[GS, b'V', 66, 0]); // partial cut
    out
}

fn line(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(text.as_bytes());
    out.push(b'\n');
}

fn sep(out: &mut Vec<u8>, cols: usize) {
    let dashes: String = "-".repeat(cols);
    line(out, &dashes);
}
