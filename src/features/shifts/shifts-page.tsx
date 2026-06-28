import { useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Lock, Unlock, Wallet } from "lucide-react";
import { toast } from "sonner";
import { ipc } from "@/lib/ipc/commands";
import { formatCurrency, formatDateTime } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

export function ShiftsPage() {
  const qc = useQueryClient();
  const { data: shift, isLoading } = useQuery({
    queryKey: ["current-shift"],
    queryFn: () => ipc.currentShift(),
  });
  const [cash, setCash] = useState(0);

  const open = async () => {
    await ipc.openShift(cash);
    toast.success("Shift dibuka");
    setCash(0);
    qc.invalidateQueries({ queryKey: ["current-shift"] });
  };

  const close = async () => {
    const closed = await ipc.closeShift(cash);
    toast.success("Shift ditutup", {
      description: `Selisih: ${formatCurrency(
        (closed.closing_cash ?? 0) - (closed.expected_cash ?? 0),
      )}`,
    });
    setCash(0);
    qc.invalidateQueries({ queryKey: ["current-shift"] });
  };

  if (isLoading) return null;

  return (
    <div className="mx-auto max-w-md p-6">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Wallet className="h-5 w-5" /> Manajemen Shift Kas
          </CardTitle>
          <CardDescription>
            {shift
              ? `Shift terbuka sejak ${formatDateTime(shift.opened_at)}`
              : "Tidak ada shift yang sedang berjalan"}
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {shift ? (
            <>
              <div className="space-y-1 rounded-lg bg-muted p-3 text-sm">
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Kas Awal</span>
                  <span>{formatCurrency(shift.opening_cash)}</span>
                </div>
                <div className="flex justify-between">
                  <span className="text-muted-foreground">Penjualan</span>
                  <span>{formatCurrency(shift.total_sales)}</span>
                </div>
              </div>
              <div className="space-y-1.5">
                <Label>Kas Akhir (hitung fisik)</Label>
                <Input
                  type="number"
                  value={cash || ""}
                  onChange={(e) => setCash(Number(e.target.value))}
                />
              </div>
              <Button className="w-full" variant="destructive" onClick={close}>
                <Lock /> Tutup Shift
              </Button>
            </>
          ) : (
            <>
              <div className="space-y-1.5">
                <Label>Kas Awal (modal laci)</Label>
                <Input
                  type="number"
                  value={cash || ""}
                  onChange={(e) => setCash(Number(e.target.value))}
                />
              </div>
              <Button className="w-full" onClick={open}>
                <Unlock /> Buka Shift
              </Button>
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
