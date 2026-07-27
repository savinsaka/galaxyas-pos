# Deploy Relay GALAXYAS POS

Target: satu subdomain di VPS, misal `relay.jjapps.net`.

## 1. DNS

Buat A record `relay.jjapps.net` → IP VPS. Tunggu sampai `ping relay.jjapps.net`
menjawab IP yang benar sebelum lanjut (Let's Encrypt akan gagal kalau DNS belum
menyebar).

## 2. Pasang aplikasi

VPS ini menaruh aplikasi di `/home/galaxyas/<app>` dan menjalankannya sebagai user
`galaxyas` (lihat `galaxyas-sync` :8000 dan `galaxyas-mobile` :8001 yang sudah
ada). Relay mengikuti pola yang sama, di port **9010**.

Dari PC pengembang, salin sumbernya (ganti `USER@VPS`):

```bash
ssh USER@VPS "mkdir -p /home/galaxyas/galaxyas-relay/scripts" && scp relay/app.py relay/requirements.txt USER@VPS:/home/galaxyas/galaxyas-relay/ && scp relay/scripts/add_store.py relay/scripts/install_vps.sh USER@VPS:/home/galaxyas/galaxyas-relay/scripts/
```

Lalu di VPS — satu perintah untuk venv, systemd, dan pendaftaran toko:

```bash
bash /home/galaxyas/galaxyas-relay/scripts/install_vps.sh
```

Skrip mencetak blok `SIMPAN INI` berisi **store_id** dan **agent_key**. Keduanya
diacak saat itu juga (bukan nilai yang bisa ditebak atau dicari belakangan) dan
`agent_key` hanya ditampilkan sekali — yang tersimpan di `relay.db` cuma
hash-nya. Kalau hilang, daftarkan toko baru.

## 3. Kalau ingin memasang manual

```bash
cd /home/galaxyas/galaxyas-relay
python3 -m venv .venv
.venv/bin/pip install -r requirements.txt
RELAY_DB=$PWD/relay.db .venv/bin/python scripts/add_store.py "Toko Pusat"
```

Unit systemd-nya (`/etc/systemd/system/galaxyas-relay.service`):

```ini
[Unit]
Description=GALAXYAS POS Relay
After=network.target

[Service]
User=galaxyas
WorkingDirectory=/home/galaxyas/galaxyas-relay
Environment=RELAY_DB=/home/galaxyas/galaxyas-relay/relay.db
ExecStart=/home/galaxyas/galaxyas-relay/.venv/bin/uvicorn app:app --host 127.0.0.1 --port 9010 --proxy-headers --forwarded-allow-ips='127.0.0.1' --ws-ping-interval 20 --ws-ping-timeout 60
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
```

> `--ws-ping-timeout 60` penting: PC kasir berdenyut tiap 15 detik, tapi jaringan
> toko kadang tersendat. Timeout bawaan (20 detik) bikin socket diputus-sambung
> tanpa perlu, dan setiap putus itu tampak sebagai "PC kasir mati" di HP.

> **Satu proses saja.** Registry socket agent dan rate limit disimpan di memori
> proses, jadi jangan pakai `--workers > 1` tanpa mengganti keduanya dengan
> penyimpanan bersama (mis. Redis). Satu worker cukup: beban relay hanya
> meneruskan JSON.

## 4. Caddy

VPS ini memakai **Caddy** (bukan nginx) untuk `jjapps.net` dan `app.jjapps.net`.
Tambahkan satu blok di `/etc/caddy/Caddyfile`:

```
relay.jjapps.net {
    reverse_proxy 127.0.0.1:9010
}
```

Caddy meneruskan WebSocket dan mengisi `X-Forwarded-For` tanpa konfigurasi
tambahan, dan mengurus sertifikat sendiri.

```bash
sudo cp /etc/caddy/Caddyfile /etc/caddy/Caddyfile.bak    # cadangan dulu
sudo caddy validate --config /etc/caddy/Caddyfile
sudo systemctl reload caddy                              # reload, bukan restart
```

> **Tambahkan blok ini hanya SETELAH A record hidup.** Kalau domainnya belum
> menunjuk ke VPS, Caddy gagal validasi ACME lalu mundur bertahap (backoff), jadi
> sertifikatnya bisa telat lama walau DNS-nya sudah benar belakangan.
> `reload` dipakai supaya jjapps.net & app.jjapps.net tidak ikut terputus.

## 5. Di PC kasir

1. Pengaturan → Server Pusat → centang **"Jadikan PC ini Server Pusat"** (jalur
   LOCAL tetap dipakai saat HP di wifi toko).
2. Di bagian **Akses Online**: isi URL Relay (`relay.jjapps.net`), Store ID, dan
   Agent Key dari langkah 2 → **Simpan** → **Nyalakan Akses Online**.
3. Indikator harus berubah jadi 🟢 **Terhubung ke relay** dalam beberapa detik.

**Setel PC supaya tidak pernah tidur.** Akses Online mati total begitu PC tidur —
itu memang desainnya (tidak ada antrian, supaya stok tidak pernah ketinggalan),
tapi bagi kasir di lapangan bentuknya "app HP tiba-tiba bilang PC kasir mati".

```powershell
powercfg /change standby-timeout-ac 0
powercfg /change hibernate-timeout-ac 0
```

Layar boleh tetap mati (`monitor-timeout-ac`) — yang tidak boleh cuma tidur/hibernasi.

## 6. Uji

```bash
curl https://relay.jjapps.net/health
```

Harus `{"status":"ok","stores_online":0}`. Setelah desktop dinyalakan dengan
Akses Online aktif, `stores_online` jadi `1`.

Cek PC kasir benar-benar terhubung (harus 200, bukan 503):

```bash
curl -i https://relay.jjapps.net/s/<store_id>/health
```

## Pemeliharaan

- Log: `journalctl -u galaxyas-relay -f`
- Cabut satu HP: dari **desktop** (Pengaturan → Perangkat Terhubung), bukan dari
  relay — token perangkat dimiliki PC kasir, relay hanya menyalin hash-nya.
- Cabut satu toko: hapus barisnya dari `relay.db` lalu restart service.
- Relay tidak menyimpan data POS. Backup `relay.db` (isinya cuma daftar toko)
  bersifat opsional; hilang = tinggal daftarkan ulang tokonya.
