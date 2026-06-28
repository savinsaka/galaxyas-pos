import { Loader2 } from "lucide-react";
import { useHeldSales } from "./use-pos";
import { formatCurrency, formatDateTime } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onResume: (saleId: string) => void;
}

export function HeldSalesDialog({ open, onOpenChange, onResume }: Props) {
  const { data: held, isLoading } = useHeldSales();

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-lg">
        <DialogHeader>
          <DialogTitle>Transaksi Ditahan</DialogTitle>
        </DialogHeader>
        {isLoading ? (
          <div className="flex justify-center py-8">
            <Loader2 className="h-6 w-6 animate-spin text-primary" />
          </div>
        ) : !held?.length ? (
          <p className="py-8 text-center text-sm text-muted-foreground">
            Tidak ada transaksi yang ditahan.
          </p>
        ) : (
          <div className="max-h-80 space-y-2 overflow-auto">
            {held.map((s) => (
              <div
                key={s.id}
                className="flex items-center justify-between rounded-md border p-3"
              >
                <div>
                  <div className="font-medium">
                    {formatCurrency(s.total || s.subtotal)}
                  </div>
                  <div className="text-xs text-muted-foreground">
                    {formatDateTime(s.created_at)}
                  </div>
                </div>
                <Button size="sm" onClick={() => onResume(s.id)}>
                  Lanjutkan
                </Button>
              </div>
            ))}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
