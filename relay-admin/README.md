# GALAXYAS Relay Admin

Alat pemilik untuk mengelola toko di relay: lihat toko mana yang PC kasirnya
sedang online, daftarkan toko baru (dapat Store ID + Agent Key), dan cabut toko.

**Bukan bagian dari GALAXYAS POS.** Sengaja proyek terpisah di luar workspace
`desktop/`, jadi tidak pernah ikut terpasang di PC kasir. Exe ini memegang kunci
admin relay — yang bisa membuat dan menghapus toko — jadi simpan di PC pemilik
saja, jangan dibagikan ke kasir.

Sebelum ada alat ini, mendaftarkan toko berarti SSH ke VPS lalu menjalankan
skrip Python. Itu bukan pekerjaan yang masuk akal dibebankan ke pemilik toko.

## Pakai

```bash
cargo build --release
```

Hasilnya satu berkas portabel `target/release/galaxyas-relay-admin.exe` (±4,7 MB)
— tidak perlu installer, tinggal salin ke mana saja.

Isi sekali di jendelanya:

| Kolom | Isi |
|---|---|
| Alamat relay | `relay.jjapps.net` |
| Kunci admin | nilai `RELAY_ADMIN_KEY` di server |

Tersimpan di `%APPDATA%\galaxyas-relay-admin\config.json`, jadi lain kali tinggal
tekan **Muat Ulang**.

## Alur menambah toko

1. **Tambah Toko** → isi nama → **Buat**.
2. Muncul **Store ID** dan **Agent Key**. Salin keduanya sekarang — Agent Key
   hanya ditampilkan sekali, server cuma menyimpan hash-nya.
3. Di PC kasir toko itu: Pengaturan → Server Pusat → Akses Online → isi alamat
   relay + kedua nilai tadi → Simpan → Nyalakan.
4. Kembali ke sini, **Muat Ulang** — toko itu harus berubah jadi ● online.

## Mengaktifkan kunci admin di server

Rute `/admin/*` **mati total** kalau `RELAY_ADMIN_KEY` tidak diisi (menjawab 404,
bukan 403 — supaya keberadaannya tidak ketahuan). Di
`/etc/systemd/system/galaxyas-relay.service`:

```ini
Environment=RELAY_ADMIN_KEY=<acak panjang, mis. `openssl rand -hex 24`>
```

lalu `sudo systemctl daemon-reload && sudo systemctl restart galaxyas-relay`.

## Catatan

- Menghapus toko memutus PC kasirnya dari relay saat itu juga, dan kredensial
  lamanya langsung ditolak.
- Data POS tidak ikut terhapus — relay memang tidak pernah menyimpannya.
- Kalau Agent Key hilang, tidak ada cara memunculkannya lagi: buat toko baru dan
  isi ulang di PC kasir.
