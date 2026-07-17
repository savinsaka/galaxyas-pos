//! Cetak byte mentah (ESC/POS) langsung ke spooler printer Windows (datatype
//! RAW), melewati driver GDI. Perlu jalur RAW agar perintah seperti autocut
//! (GS V) sampai ke printer thermal tanpa difilter/diterjemahkan oleh driver.
#![cfg(target_os = "windows")]

use std::ptr::null_mut;

use windows_sys::Win32::Foundation::HANDLE;
use windows_sys::Win32::Graphics::Printing::{
    ClosePrinter, EndDocPrinter, EndPagePrinter, GetDefaultPrinterW, OpenPrinterW,
    StartDocPrinterW, StartPagePrinter, WritePrinter, DOC_INFO_1W,
};

use crate::error::{AppError, AppResult};

fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

fn default_printer_name() -> AppResult<String> {
    let mut len: u32 = 0;
    unsafe {
        // Panggilan pertama gagal dengan sengaja untuk mendapatkan panjang buffer.
        GetDefaultPrinterW(null_mut(), &mut len);
    }
    if len == 0 {
        return Err(AppError::Other("Tidak ada printer default di sistem ini.".into()));
    }
    let mut buf: Vec<u16> = vec![0; len as usize];
    let ok = unsafe { GetDefaultPrinterW(buf.as_mut_ptr(), &mut len) };
    if ok == 0 {
        return Err(AppError::Other("Gagal membaca nama printer default.".into()));
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Ok(String::from_utf16_lossy(&buf[..end]))
}

/// Kirim `data` mentah ke `printer` (atau printer default bila `None`).
pub fn print_raw(printer: Option<&str>, data: &[u8]) -> AppResult<()> {
    let name = match printer {
        Some(p) if !p.trim().is_empty() => p.to_string(),
        _ => default_printer_name()?,
    };
    let printer_w = to_wide(&name);
    let mut handle: HANDLE = null_mut();

    let opened = unsafe { OpenPrinterW(printer_w.as_ptr() as *mut u16, &mut handle, null_mut()) };
    if opened == 0 {
        return Err(AppError::Other(format!("Gagal membuka printer '{name}'.")));
    }

    let mut doc_name = to_wide("GALAXYAS Struk");
    let mut datatype = to_wide("RAW");
    let doc_info = DOC_INFO_1W {
        pDocName: doc_name.as_mut_ptr(),
        pOutputFile: null_mut(),
        pDatatype: datatype.as_mut_ptr(),
    };

    let result = (|| -> AppResult<()> {
        let job = unsafe { StartDocPrinterW(handle, 1, &doc_info as *const DOC_INFO_1W) };
        if job == 0 {
            return Err(AppError::Other("Gagal memulai print job.".into()));
        }
        if unsafe { StartPagePrinter(handle) } == 0 {
            unsafe { EndDocPrinter(handle) };
            return Err(AppError::Other("Gagal memulai halaman print.".into()));
        }
        let mut written: u32 = 0;
        let ok = unsafe {
            WritePrinter(
                handle,
                data.as_ptr() as *const core::ffi::c_void,
                data.len() as u32,
                &mut written,
            )
        };
        unsafe { EndPagePrinter(handle) };
        unsafe { EndDocPrinter(handle) };
        if ok == 0 || written as usize != data.len() {
            return Err(AppError::Other("Gagal menulis data ke printer.".into()));
        }
        Ok(())
    })();

    unsafe { ClosePrinter(handle) };
    result
}
