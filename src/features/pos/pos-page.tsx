import { useEffect, useMemo, useRef, useState } from "react";
import { useHotkeys } from "react-hotkeys-hook";
import { useQuery } from "@tanstack/react-query";
import { Barcode, Pause, Play, Trash2, Wallet } from "lucide-react";
import { toast } from "sonner";
import { ipc } from "@/lib/ipc/commands";
import type { Item } from "@/types";
import { useTabStore } from "@/stores/tab-store";
import { usePosStore, computeTotals } from "@/stores/pos-store";
import { useAuthStore } from "@/stores/auth-store";
import { formatCurrency, cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PaymentDialog } from "./payment-dialog";
import { HeldSalesDialog } from "./held-sales-dialog";
import {
  useCreateSale,
  usePrinterSettings,
  useStore,
} from "./use-pos";
import { printReceipt } from "./receipt";

export function PosPage({ tabId }: { tabId: string }) {
  const { data: store } = useStore();
  const { data: printer } = usePrinterSettings();
  const createSale = useCreateSale();
  const userId = useAuthStore((s) => s.session?.user.id);

  const ensureCart = usePosStore((s) => s.ensureCart);
  const cart = usePosStore((s) => s.carts[tabId]);
  const actions = usePosStore();

  const [scan, setScan] = useState("");
  const [payOpen, setPayOpen] = useState(false);
  const [resumeOpen, setResumeOpen] = useState(false);
  const scanRef = useRef<HTMLInputElement>(null);
  const isActive = useTabStore((s) => s.activeId === tabId);

  useEffect(() => {
    ensureCart(tabId, store?.tax_percent ?? 0);
  }, [tabId, store?.tax_percent, ensureCart]);

  // Quick name search suggestions while typing in the scan box.
  const { data: suggestions } = useQuery({
    queryKey: ["pos-search", scan],
    queryFn: () =>
      ipc.listItems({ search: scan, limit: 8, jenis: null, merek: null }),
    enabled: scan.length >= 2,
  });

  const totals = useMemo(
    () => (cart ? computeTotals(cart) : { subtotal: 0, pajak: 0, total: 0, kembali: 0 }),
    [cart],
  );

  const addItem = (item: Item) => {
    actions.addItem(tabId, item);
    setScan("");
    scanRef.current?.focus();
  };

  const handleScanEnter = async () => {
    const code = scan.trim();
    if (!code) return;
    const byBarcode = await ipc.findItemByBarcode(code);
    if (byBarcode) {
      addItem(byBarcode);
      return;
    }
    if (suggestions?.items.length) {
      addItem(suggestions.items[0]);
      return;
    }
    toast.error("Barang tidak ditemukan", { description: code });
  };

  const submit = async (status: "completed" | "held", bayar = 0) => {
    if (!cart || cart.lines.length === 0) {
      toast.error("Keranjang masih kosong");
      return;
    }
    const shift = await ipc.currentShift();
    try {
      const sale = await createSale.mutateAsync({
        shift_id: shift?.id ?? null,
        status,
        diskon: cart.diskon,
        pajak_persen: cart.pajak_persen,
        bayar: status === "completed" ? bayar : 0,
        items: cart.lines.map((l) => ({
          item_id: l.item_id,
          nama_item: l.nama_item,
          qty: l.qty,
          harga: l.harga,
          diskon: l.diskon,
        })),
      });
      if (status === "completed") {
        toast.success(`Transaksi ${sale.invoice_no} selesai`);
        if (store && printer) {
          const { items } = await ipc.getSale(sale.id);
          void printReceipt({ store, printer, sale, items });
        }
      } else {
        toast.success("Transaksi ditahan");
      }
      actions.clearCart(tabId);
      setPayOpen(false);
      scanRef.current?.focus();
    } catch (e) {
      toast.error("Gagal menyimpan transaksi", { description: String(e) });
    }
  };

  // Feature shortcuts only fire while this POS tab is the active one.
  const opts = { enabled: isActive, enableOnFormTags: true } as const;
  useHotkeys("f2", (e) => { e.preventDefault(); scanRef.current?.focus(); }, opts, [isActive]);
  useHotkeys("f9", (e) => { e.preventDefault(); if (cart?.lines.length) setPayOpen(true); }, opts, [isActive, cart]);
  useHotkeys("f5", (e) => { e.preventDefault(); void submit("held"); }, opts, [isActive, cart]);
  useHotkeys("f4", (e) => { e.preventDefault(); setResumeOpen(true); }, opts, [isActive]);
  useHotkeys("end", (e) => { e.preventDefault(); if (cart?.lines.length) setPayOpen(true); }, opts, [isActive, cart]);

  if (!cart) return null;

  return (
    <div className="flex h-full">
      {/* Left: cart */}
      <div className="flex min-w-0 flex-1 flex-col p-4">
        <div className="relative mb-3">
          <Barcode className="absolute left-3 top-3 h-5 w-5 text-muted-foreground" />
          <Input
            ref={scanRef}
            autoFocus
            placeholder="Scan barcode atau ketik nama barang… (F2)"
            className="h-11 pl-10 text-base"
            value={scan}
            onChange={(e) => setScan(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") void handleScanEnter();
            }}
          />
          {scan.length >= 2 && suggestions?.items.length ? (
            <div className="absolute z-20 mt-1 w-full overflow-hidden rounded-md border bg-popover shadow-lg">
              {suggestions.items.map((it) => (
                <button
                  key={it.id}
                  onClick={() => addItem(it)}
                  className="flex w-full items-center justify-between px-3 py-2 text-left text-sm hover:bg-accent"
                >
                  <span className="truncate">{it.nama_item}</span>
                  <span className="text-muted-foreground">
                    {formatCurrency(it.harga_jual)}
                  </span>
                </button>
              ))}
            </div>
          ) : null}
        </div>

        <div className="flex min-h-0 flex-1 flex-col rounded-lg border">
          <div className="grid grid-cols-[1fr_90px_120px_120px_40px] gap-2 border-b bg-muted/40 px-3 py-2 text-xs font-medium text-muted-foreground">
            <div>Barang</div>
            <div className="text-center">Qty</div>
            <div className="text-right">Harga</div>
            <div className="text-right">Subtotal</div>
            <div />
          </div>
          <div className="flex-1 overflow-auto">
            {cart.lines.length === 0 ? (
              <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
                Belum ada barang. Scan atau cari untuk menambah.
              </div>
            ) : (
              cart.lines.map((l, idx) => (
                <div
                  key={l.item_id}
                  className={cn(
                    "grid grid-cols-[1fr_90px_120px_120px_40px] items-center gap-2 border-b px-3 py-2 text-sm",
                    idx === cart.activeLine && "bg-primary/5",
                  )}
                  onClick={() => actions.setActiveLine(tabId, idx)}
                >
                  <div className="truncate font-medium">{l.nama_item}</div>
                  <Input
                    type="number"
                    value={l.qty}
                    onChange={(e) =>
                      actions.setQty(tabId, idx, Number(e.target.value))
                    }
                    className="h-8 text-center"
                  />
                  <div className="text-right">{formatCurrency(l.harga)}</div>
                  <div className="text-right font-medium">
                    {formatCurrency(l.harga * l.qty - l.diskon)}
                  </div>
                  <Button
                    size="icon"
                    variant="ghost"
                    className="h-7 w-7 text-destructive"
                    onClick={() => actions.removeLine(tabId, idx)}
                  >
                    <Trash2 className="h-3.5 w-3.5" />
                  </Button>
                </div>
              ))
            )}
          </div>
        </div>
      </div>

      {/* Right: totals */}
      <aside className="flex w-80 shrink-0 flex-col gap-3 border-l bg-card p-4">
        <div className="rounded-lg bg-primary/10 p-4">
          <div className="text-sm text-muted-foreground">Total Tagihan</div>
          <div className="text-4xl font-bold text-primary">
            {formatCurrency(totals.total)}
          </div>
        </div>

        <div className="space-y-2 text-sm">
          <div className="flex justify-between">
            <span className="text-muted-foreground">Subtotal</span>
            <span>{formatCurrency(totals.subtotal)}</span>
          </div>
          <div className="flex items-center justify-between">
            <span className="text-muted-foreground">Diskon (Rp)</span>
            <Input
              type="number"
              value={cart.diskon || ""}
              onChange={(e) =>
                actions.setDiskon(tabId, Number(e.target.value))
              }
              className="h-8 w-32 text-right"
            />
          </div>
          <div className="flex justify-between">
            <span className="text-muted-foreground">
              Pajak ({cart.pajak_persen}%)
            </span>
            <span>{formatCurrency(totals.pajak)}</span>
          </div>
        </div>

        <div className="mt-auto space-y-2">
          <Button
            className="h-14 w-full text-lg"
            disabled={cart.lines.length === 0}
            onClick={() => setPayOpen(true)}
          >
            <Wallet /> Bayar (F9)
          </Button>
          <div className="grid grid-cols-2 gap-2">
            <Button
              variant="outline"
              onClick={() => void submit("held")}
              disabled={cart.lines.length === 0}
            >
              <Pause /> Tahan (F5)
            </Button>
            <Button variant="outline" onClick={() => setResumeOpen(true)}>
              <Play /> Resume (F4)
            </Button>
          </div>
        </div>
      </aside>

      <PaymentDialog
        open={payOpen}
        onOpenChange={setPayOpen}
        total={totals.total}
        onConfirm={(bayar) => void submit("completed", bayar)}
      />
      <HeldSalesDialog
        open={resumeOpen}
        onOpenChange={setResumeOpen}
        onResume={(saleId) => void resumeHeld(saleId)}
      />
    </div>
  );

  async function resumeHeld(saleId: string) {
    const { sale, items } = await ipc.getSale(saleId);
    actions.clearCart(tabId);
    for (const it of items) {
      actions.addItem(
        tabId,
        {
          id: it.item_id,
          nama_item: it.nama_item,
          harga_jual: it.harga,
          diskon_persen: 0,
        } as Item,
        it.qty,
      );
    }
    actions.setDiskon(tabId, sale.diskon);
    await ipc.voidSale(saleId, "Resume ke keranjang");
    setResumeOpen(false);
    toast.success("Transaksi dilanjutkan");
    if (userId) scanRef.current?.focus();
  }
}
