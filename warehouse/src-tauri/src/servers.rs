//! Registry "Server Pusat": daftar server yang bisa dipilih di layar awal
//! sebelum toko. Selalu ada entry implisit "Server Lokal" (kind="local",
//! perilaku persis seperti sekarang — SQLite in-process), ditambah entry
//! "remote" untuk konek ke PC lain yang mengaktifkan Server Pusat. Disimpan di
//! `servers.json` di app data dir, pola sama seperti registry toko di
//! `stores.rs`.
//!
//! Satu entry "remote" bisa menyimpan DUA alamat sekaligus: alamat LAN (wifi
//! toko, cepat, jalan walau internet mati) dan alamat relay (bisa dipakai dari
//! mana saja). Yang dipakai ditentukan pilihan kasir di layar awal
//! (`ServerPath`), BUKAN ditebak otomatis — permintaan pemilik toko. Bentuknya
//! disengaja sebangun dengan `SavedServer` di app HP
//! (`galaxyas-mobilepos/.../data/ServerRegistry.kt`).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult};

pub const LOCAL_SERVER_ID: &str = "local";

/// Port Server Pusat. Tetap (tidak bisa diubah user) supaya pairing tidak
/// menambah satu isian lagi yang bisa salah.
pub const DEFAULT_LAN_PORT: u16 = 8899;

/// Jalur yang dipakai untuk menghubungi Server Pusat.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ServerPath {
    /// Wifi toko: `http://<ip>:8899`, langsung ke PC pusat.
    Lan,
    /// Internet: `https://<relay>/s/<store_id>`, diteruskan relay ke PC pusat.
    Online,
}

impl ServerPath {
    /// Nilai tak dikenal (registry rusak / lebih baru) dianggap LAN — jalur
    /// yang paling tidak mungkin mengejutkan kasir.
    pub fn parse(raw: &str) -> Self {
        if raw.trim().eq_ignore_ascii_case("online") {
            ServerPath::Online
        } else {
            ServerPath::Lan
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            ServerPath::Lan => "lan",
            ServerPath::Online => "online",
        }
    }

    pub fn is_online(self) -> bool {
        matches!(self, ServerPath::Online)
    }
}

/// Terima ketikan `relay.jjapps.net` maupun `https://relay.jjapps.net/`.
/// Aturannya sama seperti `relay::RelayConfig::ws_url` di sisi host.
pub fn normalize_relay_url(raw: &str) -> Option<String> {
    let raw = raw.trim().trim_end_matches('/');
    if raw.is_empty() {
        return None;
    }
    if raw.starts_with("http://") || raw.starts_with("https://") {
        Some(raw.to_string())
    } else {
        Some(format!("https://{raw}"))
    }
}

fn non_empty(value: Option<&String>) -> Option<&str> {
    value.map(|s| s.trim()).filter(|s| !s.is_empty())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerInfo {
    pub id: String,
    /// "local" | "remote"
    pub kind: String,
    pub name: String,
    /// Alamat LAN (wifi toko).
    #[serde(default)]
    pub lan_host: Option<String>,
    #[serde(default)]
    pub lan_port: Option<u16>,
    /// Alamat relay, mis. `relay.jjapps.net`.
    #[serde(default)]
    pub relay_url: Option<String>,
    /// Store ID di relay (32 karakter hex).
    #[serde(default)]
    pub store_id: Option<String>,
    /// Token per-perangkat dari `/pair` (64 hex); sah di kedua jalur. Entry
    /// lama bisa berisi kode pairing 6 karakter — masih sah di jalur LAN saja.
    #[serde(default)]
    pub device_token: Option<String>,

    // --- Bentuk lama (sebelum jalur internet ada). Hanya dibaca sekali oleh
    // `migrate()`, tidak pernah ditulis lagi. ---
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<u16>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
}

impl ServerInfo {
    pub fn local() -> Self {
        ServerInfo {
            id: LOCAL_SERVER_ID.to_string(),
            kind: "local".to_string(),
            name: "Server Lokal".to_string(),
            lan_host: None,
            lan_port: None,
            relay_url: None,
            store_id: None,
            device_token: None,
            host: None,
            port: None,
            token: None,
        }
    }

    /// Entry remote baru dengan id acak (dipakai layar "+ Tambah Server").
    pub fn new_remote(
        name: String,
        lan_host: Option<String>,
        lan_port: Option<u16>,
        relay_url: Option<String>,
        store_id: Option<String>,
        device_token: String,
    ) -> Self {
        ServerInfo {
            id: Uuid::new_v4().to_string(),
            kind: "remote".to_string(),
            name,
            lan_host,
            lan_port,
            relay_url,
            store_id,
            device_token: Some(device_token),
            host: None,
            port: None,
            token: None,
        }
    }

    pub fn is_remote(&self) -> bool {
        self.kind == "remote"
    }

    pub fn has_lan(&self) -> bool {
        non_empty(self.lan_host.as_ref()).is_some()
    }

    pub fn has_online(&self) -> bool {
        non_empty(self.relay_url.as_ref()).is_some() && non_empty(self.store_id.as_ref()).is_some()
    }

    pub fn supports(&self, path: ServerPath) -> bool {
        match path {
            ServerPath::Lan => self.has_lan(),
            ServerPath::Online => self.has_online(),
        }
    }

    /// Alamat dasar untuk jalur ini. Rute di bawahnya identik di kedua jalur
    /// (`/health`, `/pair`, `/rpc/<cmd>`) — itulah yang membuat satu transport
    /// (`lan::call`) cukup untuk keduanya.
    pub fn base_url(&self, path: ServerPath) -> Option<String> {
        match path {
            ServerPath::Lan => {
                let host = non_empty(self.lan_host.as_ref())?;
                Some(format!("http://{host}:{}", self.lan_port.unwrap_or(DEFAULT_LAN_PORT)))
            }
            ServerPath::Online => {
                let relay = normalize_relay_url(self.relay_url.as_deref().unwrap_or_default())?;
                let store = non_empty(self.store_id.as_ref())?;
                Some(format!("{relay}/s/{store}"))
            }
        }
    }

    /// Konfigurasi transport untuk jalur ini. Satu-satunya tempat base URL
    /// klien disusun, dipakai `select_server` maupun startup di `lib.rs`.
    pub fn remote_config(&self, path: ServerPath) -> Option<crate::lan::RemoteConfig> {
        if !self.is_remote() {
            return None;
        }
        Some(crate::lan::RemoteConfig {
            base_url: self.base_url(path)?,
            token: self.device_token.clone().unwrap_or_default(),
            online: path.is_online(),
        })
    }

    /// Pindahkan bentuk lama (`host`/`port`/`token`, LAN saja) ke field baru
    /// supaya instalasi yang sudah jalan tidak perlu pairing ulang. Balik
    /// `true` bila ada yang berubah (pemanggil menyimpan registry sekali).
    fn migrate(&mut self) -> bool {
        if self.host.is_none() && self.port.is_none() && self.token.is_none() {
            return false;
        }
        if !self.has_lan() {
            self.lan_host = self.host.take();
        }
        if self.lan_port.is_none() {
            self.lan_port = self.port;
        }
        if non_empty(self.device_token.as_ref()).is_none() {
            self.device_token = self.token.take();
        }
        self.host = None;
        self.port = None;
        self.token = None;
        true
    }
}

/// Server aktif + jalur yang dipilih. Dikirim ke frontend saat boot supaya
/// layar awal tahu harus menampilkan "Wifi" atau "Internet".
#[derive(Debug, Clone, Serialize)]
pub struct ActiveServer {
    pub server: ServerInfo,
    pub path: ServerPath,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Registry {
    active_id: String,
    #[serde(default = "default_path")]
    active_path: String,
    servers: Vec<ServerInfo>,
}

fn default_path() -> String {
    ServerPath::Lan.as_str().to_string()
}

fn registry_path(data_dir: &Path) -> PathBuf {
    data_dir.join("servers.json")
}

fn default_registry() -> Registry {
    Registry {
        active_id: LOCAL_SERVER_ID.to_string(),
        active_path: default_path(),
        servers: vec![ServerInfo::local()],
    }
}

fn load_registry(data_dir: &Path) -> AppResult<Registry> {
    let path = registry_path(data_dir);
    if !path.exists() {
        let reg = default_registry();
        save_registry(data_dir, &reg)?;
        return Ok(reg);
    }
    let bytes = std::fs::read(&path)?;
    let mut reg: Registry = serde_json::from_slice(&bytes)
        .map_err(|e| AppError::Other(format!("Registry server rusak: {e}")))?;
    let mut dirty = false;
    // Registry lama (sebelum fitur ini ada) mungkin belum punya entry lokal.
    if !reg.servers.iter().any(|s| s.id == LOCAL_SERVER_ID) {
        reg.servers.insert(0, ServerInfo::local());
        dirty = true;
    }
    for server in reg.servers.iter_mut() {
        if server.migrate() {
            dirty = true;
        }
    }
    if dirty {
        save_registry(data_dir, &reg)?;
    }
    Ok(reg)
}

fn save_registry(data_dir: &Path, reg: &Registry) -> AppResult<()> {
    let bytes = serde_json::to_vec_pretty(reg)
        .map_err(|e| AppError::Other(format!("Gagal menyimpan registry server: {e}")))?;
    std::fs::write(registry_path(data_dir), bytes)?;
    Ok(())
}

pub fn list_servers(data_dir: &Path) -> AppResult<Vec<ServerInfo>> {
    Ok(load_registry(data_dir)?.servers)
}

/// Server aktif + jalurnya. Jalur yang tersimpan diturunkan ke LAN kalau entry
/// aktif ternyata tidak punya alamat relay (mis. dihapus lewat edit manual).
pub fn current_active(data_dir: &Path) -> AppResult<ActiveServer> {
    let reg = load_registry(data_dir)?;
    let server = reg
        .servers
        .iter()
        .find(|s| s.id == reg.active_id)
        .cloned()
        .unwrap_or_else(ServerInfo::local);
    let mut path = ServerPath::parse(&reg.active_path);
    if server.is_remote() && !server.supports(path) {
        path = ServerPath::Lan;
    }
    Ok(ActiveServer { server, path })
}

/// Simpan server remote baru (setelah pairing berhasil).
pub fn add_server(data_dir: &Path, info: ServerInfo) -> AppResult<ServerInfo> {
    let mut reg = load_registry(data_dir)?;
    reg.servers.push(info.clone());
    save_registry(data_dir, &reg)?;
    Ok(info)
}

/// Tandai `id` sebagai server aktif (dipanggil sebelum swap mode di AppState).
/// `path = None` mempertahankan jalur yang tersimpan — dipakai pemanggil yang
/// tidak peduli jalur, mis. tombol "Putuskan Koneksi" yang balik ke lokal.
pub fn set_active(data_dir: &Path, id: &str, path: Option<ServerPath>) -> AppResult<ActiveServer> {
    let mut reg = load_registry(data_dir)?;
    let server = reg
        .servers
        .iter()
        .find(|s| s.id == id)
        .cloned()
        .ok_or_else(|| AppError::Other("Server tidak ditemukan.".into()))?;
    let path = path.unwrap_or_else(|| ServerPath::parse(&reg.active_path));
    if server.is_remote() && !server.supports(path) {
        return Err(AppError::Other(match path {
            ServerPath::Lan => "Server ini belum punya alamat wifi (IP LAN).".into(),
            ServerPath::Online => {
                "Server ini belum punya alamat internet — daftarkan ulang dengan Kode Setup dari PC pusat yang Akses Online-nya aktif.".to_string()
            }
        }));
    }
    reg.active_id = id.to_string();
    reg.active_path = path.as_str().to_string();
    save_registry(data_dir, &reg)?;
    Ok(ActiveServer { server, path })
}

pub fn remove_server(data_dir: &Path, id: &str) -> AppResult<()> {
    if id == LOCAL_SERVER_ID {
        return Err(AppError::Other("Server Lokal tidak bisa dihapus.".into()));
    }
    let mut reg = load_registry(data_dir)?;
    reg.servers.retain(|s| s.id != id);
    if reg.active_id == id {
        reg.active_id = LOCAL_SERVER_ID.to_string();
        reg.active_path = default_path();
    }
    save_registry(data_dir, &reg)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote(lan: Option<&str>, relay: Option<&str>, store: Option<&str>) -> ServerInfo {
        ServerInfo::new_remote(
            "Kasir Pusat".into(),
            lan.map(str::to_string),
            Some(DEFAULT_LAN_PORT),
            relay.map(str::to_string),
            store.map(str::to_string),
            "a".repeat(64),
        )
    }

    #[test]
    fn base_url_lan_dan_online() {
        let s = remote(Some("192.168.18.11"), Some("relay.jjapps.net"), Some("ed2a445d"));
        assert_eq!(s.base_url(ServerPath::Lan).unwrap(), "http://192.168.18.11:8899");
        assert_eq!(
            s.base_url(ServerPath::Online).unwrap(),
            "https://relay.jjapps.net/s/ed2a445d"
        );
    }

    /// Isian relay di PC pusat sengaja permisif — bentuk apa pun yang diketik
    /// pemilik toko harus menghasilkan base URL yang sama.
    #[test]
    fn url_relay_dinormalkan_dari_berbagai_bentuk_isian() {
        for raw in ["relay.jjapps.net", "https://relay.jjapps.net", "https://relay.jjapps.net/"] {
            let s = remote(None, Some(raw), Some("toko1"));
            assert_eq!(
                s.base_url(ServerPath::Online).unwrap(),
                "https://relay.jjapps.net/s/toko1",
                "gagal untuk isian {raw}"
            );
        }
        // Uji lokal tanpa TLS harus tetap bisa (lihat relay/README.md).
        let s = remote(None, Some("http://localhost:9010"), Some("toko1"));
        assert_eq!(s.base_url(ServerPath::Online).unwrap(), "http://localhost:9010/s/toko1");
    }

    #[test]
    fn jalur_tanpa_alamat_tidak_didukung_dan_tidak_punya_base_url() {
        let lan_only = remote(Some("192.168.1.5"), None, None);
        assert!(lan_only.supports(ServerPath::Lan));
        assert!(!lan_only.supports(ServerPath::Online));
        assert!(lan_only.base_url(ServerPath::Online).is_none());

        let online_only = remote(None, Some("relay.jjapps.net"), Some("toko1"));
        assert!(online_only.supports(ServerPath::Online));
        assert!(!online_only.supports(ServerPath::Lan));
        assert!(online_only.base_url(ServerPath::Lan).is_none());

        // Store ID kosong = alamat internet belum lengkap, jangan dianggap ada.
        let half = remote(None, Some("relay.jjapps.net"), Some("  "));
        assert!(!half.supports(ServerPath::Online));
    }

    #[test]
    fn remote_config_membawa_penanda_jalur() {
        let s = remote(Some("192.168.1.5"), Some("relay.jjapps.net"), Some("toko1"));
        let lan = s.remote_config(ServerPath::Lan).expect("config lan");
        assert!(!lan.online);
        assert_eq!(lan.token.len(), 64);
        let online = s.remote_config(ServerPath::Online).expect("config online");
        assert!(online.online);
        assert!(ServerInfo::local().remote_config(ServerPath::Lan).is_none());
    }

    /// Entry klien versi lama menyimpan `host`/`port`/`token` (kode pairing 6
    /// karakter) dan hanya punya jalur wifi. Instalasi yang sudah jalan tidak
    /// boleh diminta pairing ulang.
    #[test]
    fn entry_lama_dipindahkan_ke_field_baru() {
        let dir = tempdir();
        std::fs::write(
            registry_path(&dir),
            r#"{"active_id":"abc","servers":[
                {"id":"local","kind":"local","name":"Server Lokal"},
                {"id":"abc","kind":"remote","name":"Kasir 1","host":"192.168.1.7","port":8899,"token":"A1B2C3"}
            ]}"#,
        )
        .unwrap();

        let active = current_active(&dir).expect("baca registry");
        assert_eq!(active.server.id, "abc");
        assert_eq!(active.path, ServerPath::Lan, "entry lama = wifi saja");
        assert_eq!(active.server.base_url(ServerPath::Lan).unwrap(), "http://192.168.1.7:8899");
        assert_eq!(active.server.device_token.as_deref(), Some("A1B2C3"));
        assert!(!active.server.has_online());

        // Migrasi ikut tersimpan, jadi field lama tidak lagi ada di file.
        let saved = std::fs::read_to_string(registry_path(&dir)).unwrap();
        assert!(!saved.contains("\"host\""), "field lama harus hilang: {saved}");
        assert!(saved.contains("\"lan_host\""));
    }

    /// Bentuk `servers.json` yang benar-benar ada di instalasi sekarang: hanya
    /// entry lokal, field lama ditulis eksplisit `null`, tanpa `active_path`.
    /// Membacanya tidak boleh dianggap "kotor" (jangan menulis ulang file) dan
    /// tidak boleh error.
    #[test]
    fn registry_instalasi_lama_dibaca_apa_adanya() {
        let dir = tempdir();
        let asli = "{\n  \"active_id\": \"local\",\n  \"servers\": [\n    {\n      \"id\": \"local\",\n      \"kind\": \"local\",\n      \"name\": \"Server Lokal\",\n      \"host\": null,\n      \"port\": null,\n      \"token\": null\n    }\n  ]\n}";
        std::fs::write(registry_path(&dir), asli).unwrap();

        let active = current_active(&dir).expect("baca registry lama");
        assert_eq!(active.server.id, LOCAL_SERVER_ID);
        assert_eq!(active.path, ServerPath::Lan);
        assert_eq!(
            std::fs::read_to_string(registry_path(&dir)).unwrap(),
            asli,
            "membaca registry yang sudah bersih tidak boleh menulisnya ulang"
        );
    }

    #[test]
    fn jalur_aktif_tersimpan_dan_bisa_diganti() {
        let dir = tempdir();
        let info = add_server(
            &dir,
            remote(Some("192.168.1.7"), Some("relay.jjapps.net"), Some("toko1")),
        )
        .expect("tambah");

        let active = set_active(&dir, &info.id, Some(ServerPath::Online)).expect("pilih online");
        assert_eq!(active.path, ServerPath::Online);
        assert_eq!(current_active(&dir).unwrap().path, ServerPath::Online);

        // `None` = pertahankan jalur tersimpan (tombol "Putuskan Koneksi").
        assert_eq!(set_active(&dir, &info.id, None).unwrap().path, ServerPath::Online);
        assert_eq!(set_active(&dir, &info.id, Some(ServerPath::Lan)).unwrap().path, ServerPath::Lan);
    }

    #[test]
    fn memilih_jalur_yang_belum_diatur_ditolak_dengan_pesan_jelas() {
        let dir = tempdir();
        let info = add_server(&dir, remote(Some("192.168.1.7"), None, None)).expect("tambah");
        let err = set_active(&dir, &info.id, Some(ServerPath::Online)).unwrap_err().to_string();
        assert!(err.contains("alamat internet"), "pesan kurang jelas: {err}");
        // Registry tidak boleh ikut berubah saat penolakan.
        assert_eq!(current_active(&dir).unwrap().server.id, LOCAL_SERVER_ID);
    }

    /// Registry yang jalurnya "online" tapi entry-nya kehilangan alamat relay
    /// (mis. diedit manual) tidak boleh membuat app mencoba URL yang tidak ada.
    #[test]
    fn jalur_online_diturunkan_ke_lan_bila_alamat_relay_hilang() {
        let dir = tempdir();
        std::fs::write(
            registry_path(&dir),
            r#"{"active_id":"abc","active_path":"online","servers":[
                {"id":"abc","kind":"remote","name":"Kasir 1","lan_host":"192.168.1.7","lan_port":8899,"device_token":"x"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(current_active(&dir).unwrap().path, ServerPath::Lan);
    }

    #[test]
    fn hapus_server_aktif_kembali_ke_lokal_dan_jalur_lan() {
        let dir = tempdir();
        let info = add_server(
            &dir,
            remote(Some("192.168.1.7"), Some("relay.jjapps.net"), Some("toko1")),
        )
        .expect("tambah");
        set_active(&dir, &info.id, Some(ServerPath::Online)).expect("pilih");
        remove_server(&dir, &info.id).expect("hapus");

        let active = current_active(&dir).expect("baca");
        assert_eq!(active.server.id, LOCAL_SERVER_ID);
        assert_eq!(active.path, ServerPath::Lan);
        assert!(remove_server(&dir, LOCAL_SERVER_ID).is_err(), "Server Lokal tidak boleh dihapus");
    }

    #[test]
    fn jalur_diparse_permisif() {
        assert_eq!(ServerPath::parse("online"), ServerPath::Online);
        assert_eq!(ServerPath::parse("ONLINE"), ServerPath::Online);
        assert_eq!(ServerPath::parse("lan"), ServerPath::Lan);
        assert_eq!(ServerPath::parse("entah-apa"), ServerPath::Lan);
        assert_eq!(ServerPath::parse(""), ServerPath::Lan);
    }

    /// Direktori sementara sendiri: `tempfile` bukan dependency di crate ini.
    fn tempdir() -> PathBuf {
        let dir = std::env::temp_dir().join(format!("galaxyas-servers-{}", Uuid::new_v4()));
        std::fs::create_dir_all(&dir).expect("buat dir uji");
        dir
    }
}
