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

fn render_escpos(data: &ReceiptData) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&[ESC, b'@']); // init
    out.extend_from_slice(&[ESC, b'a', 1]); // center
    line(&mut out, &data.printer.header_text);
    if let Some(addr) = &data.store.address {
        line(&mut out, addr);
    }
    out.extend_from_slice(&[ESC, b'a', 0]); // left
    sep(&mut out);
    if let Some(inv) = &data.sale.invoice_no {
        line(&mut out, &format!("No: {inv}"));
    }
    sep(&mut out);
    for it in &data.items {
        line(&mut out, &it.nama_item);
        line(
            &mut out,
            &format!("  {} x {:.0} = {:.0}", it.qty, it.harga, it.subtotal),
        );
    }
    sep(&mut out);
    line(&mut out, &format!("Subtotal : {:.0}", data.sale.subtotal));
    line(&mut out, &format!("Diskon   : {:.0}", data.sale.diskon));
    line(&mut out, &format!("Pajak    : {:.0}", data.sale.pajak));
    line(&mut out, &format!("TOTAL    : {:.0}", data.sale.total));
    line(&mut out, &format!("Bayar    : {:.0}", data.sale.bayar));
    line(&mut out, &format!("Kembali  : {:.0}", data.sale.kembali));
    sep(&mut out);
    out.extend_from_slice(&[ESC, b'a', 1]);
    line(&mut out, &data.printer.footer_text);
    out.extend_from_slice(&[GS, b'V', 66, 0]); // partial cut
    out
}

fn line(out: &mut Vec<u8>, text: &str) {
    out.extend_from_slice(text.as_bytes());
    out.push(b'\n');
}

fn sep(out: &mut Vec<u8>) {
    line(out, "--------------------------------");
}
