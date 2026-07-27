//! GALAXYAS Relay Admin — alat pemilik untuk mengelola toko di relay.
//!
//! Sebelum ada alat ini, mendaftarkan toko berarti SSH ke VPS dan menjalankan
//! skrip Python — pekerjaan yang tidak masuk akal dibebankan ke pemilik toko.
//!
//! **Bukan bagian dari installer GALAXYAS POS.** Ini exe portabel yang dipegang
//! pemilik di satu PC saja: ia memegang kunci admin relay, yang boleh membuat
//! dan menghapus toko. Kasir tidak perlu — dan tidak boleh — punya ini.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::Duration;

use eframe::egui;
use serde::{Deserialize, Serialize};

const APP_NAME: &str = "GALAXYAS Relay Admin";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Config {
    relay_url: String,
    /// Disimpan apa adanya di profil user PC ini. Kunci ini setara kata sandi
    /// admin relay, jadi exe ini tidak untuk dibagikan ke kasir.
    admin_key: String,
}

impl Config {
    fn path() -> Option<std::path::PathBuf> {
        Some(dirs::config_dir()?.join("galaxyas-relay-admin").join("config.json"))
    }

    fn load() -> Self {
        Self::path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save(&self) -> Result<(), String> {
        let path = Self::path().ok_or("folder konfigurasi tidak ditemukan")?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        }
        let body = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(path, body).map_err(|e| e.to_string())
    }

    /// Terima ketikan `relay.jjapps.net` maupun `https://relay.jjapps.net/`.
    fn base_url(&self) -> String {
        let raw = self.relay_url.trim().trim_end_matches('/');
        if raw.starts_with("http://") || raw.starts_with("https://") {
            raw.to_string()
        } else {
            format!("https://{raw}")
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct Store {
    id: String,
    name: String,
    #[serde(default)]
    online: bool,
}

#[derive(Debug, Clone, Deserialize)]
struct NewStore {
    id: String,
    name: String,
    agent_key: String,
}

enum Msg {
    Stores(Result<Vec<Store>, String>),
    Created(Result<NewStore, String>),
    Deleted(Result<(), String>),
}

// ---------- HTTP ----------

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(20))
        .build()
}

/// Ubah kegagalan ureq jadi pesan yang layak dibaca pemilik toko: pesan `error`
/// dari relay kalau ada, kalau tidak baru penjelasan jaringan.
fn explain(err: ureq::Error) -> String {
    match err {
        ureq::Error::Status(code, resp) => {
            let body = resp.into_string().unwrap_or_default();
            let from_relay = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string));
            match (code, from_relay) {
                (_, Some(msg)) => msg,
                (404, None) => {
                    "Relay menjawab 404. Kunci admin belum diaktifkan di server \
                     (RELAY_ADMIN_KEY belum diisi)."
                        .to_string()
                }
                (c, None) => format!("Relay menolak permintaan (kode {c})."),
            }
        }
        ureq::Error::Transport(t) => {
            format!("Tidak bisa menghubungi relay — periksa alamat dan koneksi internet. ({t})")
        }
    }
}

fn fetch_stores(cfg: Config, tx: Sender<Msg>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let result = agent()
            .get(&format!("{}/admin/stores", cfg.base_url()))
            .set("X-Admin-Key", &cfg.admin_key)
            .call()
            .map_err(explain)
            .and_then(|r| r.into_json::<Vec<Store>>().map_err(|e| e.to_string()));
        let _ = tx.send(Msg::Stores(result));
        ctx.request_repaint();
    });
}

fn create_store(cfg: Config, name: String, tx: Sender<Msg>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let result = agent()
            .post(&format!("{}/admin/stores", cfg.base_url()))
            .set("X-Admin-Key", &cfg.admin_key)
            .send_json(serde_json::json!({ "name": name }))
            .map_err(explain)
            .and_then(|r| r.into_json::<NewStore>().map_err(|e| e.to_string()));
        let _ = tx.send(Msg::Created(result));
        ctx.request_repaint();
    });
}

fn delete_store(cfg: Config, id: String, tx: Sender<Msg>, ctx: egui::Context) {
    std::thread::spawn(move || {
        let result = agent()
            .delete(&format!("{}/admin/stores/{}", cfg.base_url(), id))
            .set("X-Admin-Key", &cfg.admin_key)
            .call()
            .map_err(explain)
            .map(|_| ());
        let _ = tx.send(Msg::Deleted(result));
        ctx.request_repaint();
    });
}

// ---------- UI ----------

struct App {
    cfg: Config,
    stores: Vec<Store>,
    status: String,
    error: Option<String>,
    busy: bool,
    new_name: String,
    /// Ditampilkan menyolok setelah toko dibuat: agent key hanya ada di sini,
    /// sekali seumur hidup toko itu.
    created: Option<NewStore>,
    confirm_delete: Option<String>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl Default for App {
    fn default() -> Self {
        let (tx, rx) = channel();
        Self {
            cfg: Config::load(),
            stores: Vec::new(),
            status: String::new(),
            error: None,
            busy: false,
            new_name: String::new(),
            created: None,
            confirm_delete: None,
            tx,
            rx,
        }
    }
}

impl App {
    fn siap(&self) -> bool {
        !self.cfg.relay_url.trim().is_empty() && !self.cfg.admin_key.trim().is_empty()
    }

    fn salin(&mut self, teks: &str) {
        match arboard::Clipboard::new().and_then(|mut c| c.set_text(teks.to_string())) {
            Ok(()) => self.status = "Disalin ke clipboard.".into(),
            Err(e) => self.error = Some(format!("Gagal menyalin: {e}")),
        }
    }

    fn muat(&mut self, ctx: &egui::Context) {
        if !self.siap() {
            self.error = Some("Alamat relay dan kunci admin wajib diisi.".into());
            return;
        }
        self.busy = true;
        self.error = None;
        self.status = "Memuat…".into();
        fetch_stores(self.cfg.clone(), self.tx.clone(), ctx.clone());
    }

    fn proses_pesan(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            self.busy = false;
            match msg {
                Msg::Stores(Ok(list)) => {
                    let online = list.iter().filter(|s| s.online).count();
                    self.status = format!("{} toko · {online} sedang online", list.len());
                    self.stores = list;
                    self.error = None;
                }
                Msg::Created(Ok(baru)) => {
                    self.status = format!("Toko \"{}\" dibuat.", baru.name);
                    self.new_name.clear();
                    self.created = Some(baru);
                    self.error = None;
                }
                Msg::Deleted(Ok(())) => {
                    self.status = "Toko dihapus.".into();
                    self.error = None;
                }
                Msg::Stores(Err(e)) | Msg::Created(Err(e)) | Msg::Deleted(Err(e)) => {
                    self.error = Some(e);
                    self.status.clear();
                }
            }
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.proses_pesan();

        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading(APP_NAME);
                ui.label(
                    egui::RichText::new(
                        "Alat pemilik. Simpan di PC Anda sendiri — kunci admin di sini \
                         boleh membuat dan menghapus toko.",
                    )
                    .small()
                    .weak(),
                );
                ui.add_space(10.0);

                // --- Koneksi ---
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Koneksi Relay").strong());
                    egui::Grid::new("cfg").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                        ui.label("Alamat relay");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cfg.relay_url)
                                .hint_text("relay.jjapps.net")
                                .desired_width(320.0),
                        );
                        ui.end_row();

                        ui.label("Kunci admin");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.cfg.admin_key)
                                .password(true)
                                .hint_text("dari RELAY_ADMIN_KEY di server")
                                .desired_width(320.0),
                        );
                        ui.end_row();
                    });
                    ui.horizontal(|ui| {
                        if ui.add_enabled(!self.busy, egui::Button::new("Simpan & Muat")).clicked() {
                            match self.cfg.save() {
                                Ok(()) => self.muat(ctx),
                                Err(e) => self.error = Some(format!("Gagal menyimpan: {e}")),
                            }
                        }
                        if ui.add_enabled(!self.busy, egui::Button::new("↻ Muat Ulang")).clicked() {
                            self.muat(ctx);
                        }
                    });
                });

                ui.add_space(8.0);
                if !self.status.is_empty() {
                    ui.label(egui::RichText::new(&self.status).weak());
                }
                if let Some(err) = self.error.clone() {
                    ui.colored_label(egui::Color32::from_rgb(200, 60, 60), err);
                }

                // --- Kredensial toko yang baru dibuat ---
                if let Some(baru) = self.created.clone() {
                    ui.add_space(8.0);
                    ui.group(|ui| {
                        ui.label(
                            egui::RichText::new(format!("Toko baru: {}", baru.name))
                                .strong()
                                .color(egui::Color32::from_rgb(30, 130, 60)),
                        );
                        ui.label(
                            egui::RichText::new(
                                "Agent Key hanya ditampilkan SEKARANG. Salin dulu sebelum \
                                 menutup — server cuma menyimpan sidik jarinya.",
                            )
                            .small(),
                        );
                        ui.add_space(4.0);
                        for (label, nilai) in
                            [("Store ID", baru.id.clone()), ("Agent Key", baru.agent_key.clone())]
                        {
                            ui.horizontal(|ui| {
                                ui.label(format!("{label}:"));
                                ui.add(
                                    egui::TextEdit::singleline(&mut nilai.clone())
                                        .desired_width(430.0)
                                        .font(egui::TextStyle::Monospace),
                                );
                                if ui.button("Salin").clicked() {
                                    self.salin(&nilai);
                                }
                            });
                        }
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new(
                                "Masukkan keduanya di PC kasir: Pengaturan → Server Pusat \
                                 → Akses Online.",
                            )
                            .small()
                            .weak(),
                        );
                        if ui.button("Sudah saya salin, tutup").clicked() {
                            self.created = None;
                            self.muat(ctx);
                        }
                    });
                }

                // --- Tambah toko ---
                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.label(egui::RichText::new("Tambah Toko").strong());
                    ui.horizontal(|ui| {
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_name)
                                .hint_text("Nama toko, mis. Toko Cabang 2")
                                .desired_width(280.0),
                        );
                        let boleh = !self.busy && self.siap() && !self.new_name.trim().is_empty();
                        if ui.add_enabled(boleh, egui::Button::new("Buat")).clicked() {
                            self.busy = true;
                            self.error = None;
                            create_store(
                                self.cfg.clone(),
                                self.new_name.trim().to_string(),
                                self.tx.clone(),
                                ctx.clone(),
                            );
                        }
                    });
                });

                // --- Daftar toko ---
                ui.add_space(8.0);
                ui.label(egui::RichText::new("Daftar Toko").strong());
                if self.stores.is_empty() {
                    ui.label(
                        egui::RichText::new("Belum ada toko, atau belum dimuat.").weak().small(),
                    );
                }

                let daftar = self.stores.clone();
                for store in daftar {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            let (warna, teks) = if store.online {
                                (egui::Color32::from_rgb(30, 150, 60), "● online")
                            } else {
                                (egui::Color32::GRAY, "○ PC kasir mati")
                            };
                            ui.colored_label(warna, teks);
                            ui.label(egui::RichText::new(&store.name).strong());
                        });
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(&store.id)
                                    .monospace()
                                    .small(),
                            );
                            if ui.small_button("Salin ID").clicked() {
                                self.salin(&store.id);
                            }
                            if self.confirm_delete.as_deref() == Some(store.id.as_str()) {
                                ui.colored_label(
                                    egui::Color32::from_rgb(200, 60, 60),
                                    "Yakin hapus?",
                                );
                                if ui.small_button("Ya, hapus").clicked() {
                                    self.confirm_delete = None;
                                    self.busy = true;
                                    delete_store(
                                        self.cfg.clone(),
                                        store.id.clone(),
                                        self.tx.clone(),
                                        ctx.clone(),
                                    );
                                }
                                if ui.small_button("Batal").clicked() {
                                    self.confirm_delete = None;
                                }
                            } else if ui.small_button("Hapus").clicked() {
                                self.confirm_delete = Some(store.id.clone());
                            }
                        });
                    });
                }

                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new(
                        "Menghapus toko memutus PC kasirnya dari relay saat itu juga. \
                         Data POS tidak ikut terhapus — relay memang tidak menyimpannya.",
                    )
                    .small()
                    .weak(),
                );
            });
        });
    }
}

/// Ikon jendela & taskbar. Di-embed ke exe, jadi tidak ada berkas pendamping
/// yang bisa tertinggal saat exe-nya disalin ke mana-mana.
fn icon() -> Option<egui::IconData> {
    let img = image::load_from_memory(include_bytes!("../assets/icon.png")).ok()?;
    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some(egui::IconData { rgba: rgba.into_raw(), width, height })
}

fn main() -> eframe::Result<()> {
    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([640.0, 720.0])
        .with_min_inner_size([520.0, 420.0])
        .with_title(APP_NAME);
    if let Some(icon) = icon() {
        viewport = viewport.with_icon(icon);
    }
    eframe::run_native(
        APP_NAME,
        eframe::NativeOptions { viewport, ..Default::default() },
        Box::new(|_cc| Ok(Box::<App>::default())),
    )
}
