/** Jam berjalan otomatis (di-refresh tiap detik) untuk header Kasir POS, Item Masuk/Keluar, Opname. */
export function createLiveClock() {
  let now = $state(new Date());
  const timer = setInterval(() => {
    now = new Date();
  }, 1000);
  return {
    get now() {
      return now;
    },
    stop() {
      clearInterval(timer);
    },
  };
}
