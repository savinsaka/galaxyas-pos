const rupiah = new Intl.NumberFormat("id-ID", {
  style: "currency",
  currency: "IDR",
  minimumFractionDigits: 2,
  maximumFractionDigits: 2,
});

export const formatIDR = (n: number) => rupiah.format(n || 0);

export const formatQty = (n: number) =>
  Number.isInteger(n) ? n.toString() : n.toFixed(2);

export const formatDateTime = (iso: string) => {
  if (!iso) return "-";
  const d = new Date(iso);
  return d.toLocaleString("id-ID", {
    day: "2-digit",
    month: "short",
    year: "numeric",
    hour: "2-digit",
    minute: "2-digit",
    second: "2-digit",
  });
};

/** Jam:menit:detik saja, mis. untuk jam berjalan di header. */
export const formatTime = (d: Date) =>
  d.toLocaleTimeString("id-ID", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
