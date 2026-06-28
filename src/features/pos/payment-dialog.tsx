import { useEffect, useRef, useState } from "react";
import { formatCurrency } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  total: number;
  onConfirm: (bayar: number) => void;
}

const QUICK = [50000, 100000, 150000, 200000];

export function PaymentDialog({ open, onOpenChange, total, onConfirm }: Props) {
  const [bayar, setBayar] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (open) {
      setBayar(total);
      setTimeout(() => inputRef.current?.select(), 50);
    }
  }, [open, total]);

  const kembali = Math.max(0, bayar - total);
  const kurang = bayar < total;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Pembayaran</DialogTitle>
        </DialogHeader>
        <div className="space-y-4">
          <div className="rounded-lg bg-muted p-4 text-center">
            <div className="text-sm text-muted-foreground">Total</div>
            <div className="text-3xl font-bold">{formatCurrency(total)}</div>
          </div>
          <div className="space-y-1.5">
            <label className="text-sm font-medium">Tunai diterima</label>
            <Input
              ref={inputRef}
              type="number"
              value={bayar || ""}
              onChange={(e) => setBayar(Number(e.target.value))}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !kurang) onConfirm(bayar);
              }}
              className="h-12 text-2xl"
            />
          </div>
          <div className="grid grid-cols-4 gap-2">
            <Button variant="outline" onClick={() => setBayar(total)}>
              Uang Pas
            </Button>
            {QUICK.map((q) => (
              <Button key={q} variant="outline" onClick={() => setBayar(q)}>
                {(q / 1000).toFixed(0)}k
              </Button>
            ))}
          </div>
          <div className="flex items-center justify-between rounded-lg border p-3">
            <span className="text-sm">Kembalian</span>
            <span
              className={`text-xl font-bold ${kurang ? "text-destructive" : "text-success"}`}
            >
              {kurang
                ? `Kurang ${formatCurrency(total - bayar)}`
                : formatCurrency(kembali)}
            </span>
          </div>
          <Button
            className="h-12 w-full text-base"
            disabled={kurang}
            onClick={() => onConfirm(bayar)}
          >
            Selesaikan Transaksi (Enter)
          </Button>
        </div>
      </DialogContent>
    </Dialog>
  );
}
