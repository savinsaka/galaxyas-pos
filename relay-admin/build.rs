//! Tempelkan ikon + metadata ke exe Windows.
//!
//! Tanpa ini exe-nya memakai ikon bawaan Windows yang polos, dan pemilik toko
//! wajar saja curiga itu program asing.

fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=assets/icon.ico");
        let mut res = winres::WindowsResource::new();
        res.set_icon("assets/icon.ico");
        res.set("FileDescription", "GALAXYAS Relay Admin");
        res.set("ProductName", "GALAXYAS Relay Admin");
        res.set("CompanyName", "GALAXYAS");
        res.set("LegalCopyright", "GALAXYAS");
        if let Err(e) = res.compile() {
            // Jangan gagalkan build hanya karena ikon — biarkan exe-nya tetap jadi.
            println!("cargo:warning=gagal menempelkan ikon: {e}");
        }
    }
}
