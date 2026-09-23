; Hook installer NSIS: buat folder template laporan (.Greport) di Dokumen user
; saat instalasi. Aplikasi juga memastikan folder ini ada saat startup (lihat
; lib.rs) — jadi ini sekadar menyiapkannya lebih awal untuk instalasi baru.
!macro NSIS_HOOK_POSTINSTALL
  CreateDirectory "$DOCUMENTS\GalaxyAS POS\Reports\kasir"
  CreateDirectory "$DOCUMENTS\GalaxyAS POS\Reports\penjualan"
  CreateDirectory "$DOCUMENTS\GalaxyAS POS\Reports\item"
  CreateDirectory "$DOCUMENTS\GalaxyAS POS\Reports\persediaan"
  CreateDirectory "$DOCUMENTS\GalaxyAS POS\Reports\umum"
!macroend
