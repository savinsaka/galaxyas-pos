#!/usr/bin/env bash
# Pasang relay GALAXYAS POS di VPS jjapps.net: venv, systemd, daftar toko.
#
# Jalankan dari folder sumber yang sudah disalin ke VPS:
#     bash scripts/install_vps.sh
#
# Aman dijalankan berulang: paket di-update dan unit systemd ditulis ulang, tapi
# toko TIDAK didaftarkan ulang kalau relay.db sudah ada — supaya agent key yang
# sudah dipakai desktop tidak berubah diam-diam.
#
# Reverse proxy (Caddy) TIDAK disentuh skrip ini. VPS ini melayani jjapps.net dan
# app.jjapps.net dari Caddyfile yang sama; menimpanya lewat skrip lebih berbahaya
# daripada menambah satu blok secara sadar. Lihat DEPLOY.md langkah 4.
set -euo pipefail

APP_DIR=/home/galaxyas/galaxyas-relay
SERVICE=galaxyas-relay
RUN_USER=galaxyas
PORT="${RELAY_PORT:-9010}"
SRC="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> Menyalin berkas ke $APP_DIR"
mkdir -p "$APP_DIR/scripts"
cp "$SRC/app.py" "$SRC/requirements.txt" "$APP_DIR/"
cp "$SRC/scripts/add_store.py" "$APP_DIR/scripts/"

echo "==> Menyiapkan virtualenv"
[[ -x "$APP_DIR/.venv/bin/python" ]] || python3 -m venv "$APP_DIR/.venv"
"$APP_DIR/.venv/bin/pip" install --quiet --upgrade pip
"$APP_DIR/.venv/bin/pip" install --quiet -r "$APP_DIR/requirements.txt"
"$APP_DIR/.venv/bin/python" -c 'import app' >/dev/null

NEW_STORE=""
if [[ ! -f "$APP_DIR/relay.db" ]]; then
    echo "==> Mendaftarkan toko pertama"
    NEW_STORE=$(cd "$APP_DIR" && RELAY_DB="$APP_DIR/relay.db" .venv/bin/python \
        scripts/add_store.py "${STORE_NAME:-Toko Pusat}")
else
    echo "==> relay.db sudah ada — toko tidak didaftarkan ulang"
fi

echo "==> Menulis unit systemd"
sudo tee "/etc/systemd/system/$SERVICE.service" >/dev/null <<EOF
[Unit]
Description=GALAXYAS POS Relay (penerus permintaan HP kasir ke PC kasir)
After=network.target

[Service]
User=$RUN_USER
WorkingDirectory=$APP_DIR
Environment=RELAY_DB=$APP_DIR/relay.db
ExecStart=$APP_DIR/.venv/bin/uvicorn app:app --host 127.0.0.1 --port $PORT --proxy-headers --forwarded-allow-ips='127.0.0.1' --ws-ping-interval 20 --ws-ping-timeout 60
Restart=on-failure
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now "$SERVICE"
sudo systemctl restart "$SERVICE"
sleep 3

echo
echo "==> Status"
systemctl is-active "$SERVICE"
curl -sS "http://127.0.0.1:$PORT/health"
echo
echo "==> Service lain tidak terganggu"
systemctl is-active galaxyas-sync galaxyas-mobile caddy || true
echo

if [[ -n "$NEW_STORE" ]]; then
    echo "================ SIMPAN INI ================"
    echo "$NEW_STORE"
    echo "============================================"
    echo "Masukkan ke desktop: Pengaturan -> Server Pusat -> Akses Online."
else
    echo "Butuh toko baru? jalankan:"
    echo "  cd $APP_DIR && RELAY_DB=$APP_DIR/relay.db .venv/bin/python scripts/add_store.py \"Nama Toko\""
fi
echo
echo "Belum selesai: tambahkan blok Caddy untuk relay.jjapps.net (DEPLOY.md langkah 4)."
echo "A record HARUS sudah ada dulu, kalau tidak Caddy gagal ambil sertifikat."
