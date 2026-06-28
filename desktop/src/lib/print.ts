/** Cetak isi workspace aktif sebagai laporan (menyembunyikan ribbon/tab). */
export function printReport() {
  document.body.classList.add("printing-report");
  const cleanup = () => {
    document.body.classList.remove("printing-report");
    window.removeEventListener("afterprint", cleanup);
  };
  window.addEventListener("afterprint", cleanup);
  setTimeout(() => window.print(), 60);
}
