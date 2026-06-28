import { useRef, useState } from "react";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import * as XLSX from "xlsx";
import { ArrowDownToLine, ArrowUpFromLine, ClipboardCheck, Upload } from "lucide-react";
import { toast } from "sonner";
import { ipc } from "@/lib/ipc/commands";
import type { Item, StockTxType } from "@/types";
import { formatDateTime } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

const TYPE_LABEL: Record<StockTxType, string> = {
  in: "Masuk",
  out: "Keluar",
  adjustment: "Penyesuaian",
  opname: "Opname",
};

export function InventoryPage() {
  const qc = useQueryClient();
  const [search, setSearch] = useState("");
  const [type, setType] = useState<StockTxType>("in");
  const [selected, setSelected] = useState<Item | null>(null);
  const [qty, setQty] = useState(0);
  const [ref, setRef] = useState("");
  const opnameRef = useRef<HTMLInputElement>(null);

  const { data: itemPage } = useQuery({
    queryKey: ["inv-items", search],
    queryFn: () =>
      ipc.listItems({ search, limit: 8, jenis: null, merek: null }),
    enabled: search.length >= 2,
  });

  const { data: history } = useQuery({
    queryKey: ["stock-tx", selected?.id ?? null],
    queryFn: () => ipc.listStockTransactions(selected?.id ?? null, 100),
  });

  const submitMovement = async () => {
    if (!selected) return toast.error("Pilih barang dulu");
    if (!qty) return toast.error("Qty tidak boleh 0");
    await ipc.recordStockMovement({
      item_id: selected.id,
      type,
      qty,
      ref_doc: ref || null,
    });
    toast.success("Pergerakan stok dicatat");
    setQty(0);
    setRef("");
    qc.invalidateQueries({ queryKey: ["stock-tx"] });
    qc.invalidateQueries({ queryKey: ["items"] });
  };

  const handleOpname = async (file: File) => {
    const buf = await file.arrayBuffer();
    const wb = XLSX.read(buf, { type: "array" });
    const rows = XLSX.utils.sheet_to_json<Record<string, unknown>>(
      wb.Sheets[wb.SheetNames[0]],
    );
    const mapped = rows
      .map((r) => ({
        item_id: String(r["item_id"] ?? r["id"] ?? ""),
        counted_qty: Number(r["counted_qty"] ?? r["stok"] ?? 0),
      }))
      .filter((r) => r.item_id);
    if (!mapped.length) return toast.error("Tidak ada baris valid");
    const n = await ipc.applyStockOpname(
      mapped,
      `OPNAME-${new Date().toISOString().slice(0, 10)}`,
    );
    toast.success(`Stock opname diterapkan untuk ${n} barang`);
    qc.invalidateQueries({ queryKey: ["items"] });
    qc.invalidateQueries({ queryKey: ["stock-tx"] });
  };

  return (
    <div className="grid h-full grid-cols-[360px_1fr] gap-4 overflow-auto p-4">
      <div className="space-y-4">
        <Card>
          <CardHeader>
            <CardTitle className="text-base">Catat Pergerakan Stok</CardTitle>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="relative">
              <Input
                placeholder="Cari barang…"
                value={selected ? selected.nama_item : search}
                onChange={(e) => {
                  setSelected(null);
                  setSearch(e.target.value);
                }}
              />
              {!selected && search.length >= 2 && itemPage?.items.length ? (
                <div className="absolute z-20 mt-1 w-full rounded-md border bg-popover shadow">
                  {itemPage.items.map((it) => (
                    <button
                      key={it.id}
                      className="block w-full px-3 py-2 text-left text-sm hover:bg-accent"
                      onClick={() => {
                        setSelected(it);
                        setSearch("");
                      }}
                    >
                      {it.nama_item}{" "}
                      <span className="text-muted-foreground">
                        (stok {it.stok})
                      </span>
                    </button>
                  ))}
                </div>
              ) : null}
            </div>

            <Select value={type} onValueChange={(v) => setType(v as StockTxType)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="in">Barang Masuk</SelectItem>
                <SelectItem value="out">Barang Keluar</SelectItem>
                <SelectItem value="adjustment">Penyesuaian (set stok)</SelectItem>
              </SelectContent>
            </Select>

            <Input
              type="number"
              placeholder="Qty"
              value={qty || ""}
              onChange={(e) => setQty(Number(e.target.value))}
            />
            <Input
              placeholder="No. dokumen (opsional)"
              value={ref}
              onChange={(e) => setRef(e.target.value)}
            />
            <Button className="w-full" onClick={submitMovement}>
              {type === "in" ? <ArrowDownToLine /> : <ArrowUpFromLine />}
              Simpan
            </Button>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">Stock Opname (Excel)</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <p className="text-xs text-muted-foreground">
              Upload file dengan kolom <code>item_id</code> dan{" "}
              <code>counted_qty</code>.
            </p>
            <input
              ref={opnameRef}
              type="file"
              accept=".xlsx,.xls,.csv"
              className="hidden"
              onChange={(e) => {
                const f = e.target.files?.[0];
                if (f) void handleOpname(f);
                e.target.value = "";
              }}
            />
            <Button
              variant="outline"
              className="w-full"
              onClick={() => opnameRef.current?.click()}
            >
              <Upload /> Upload Opname
            </Button>
          </CardContent>
        </Card>
      </div>

      <Card className="flex min-h-0 flex-col">
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <ClipboardCheck className="h-4 w-4" />
            Riwayat Pergerakan {selected ? `· ${selected.nama_item}` : "(semua)"}
          </CardTitle>
        </CardHeader>
        <CardContent className="min-h-0 flex-1 overflow-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Waktu</TableHead>
                <TableHead>Tipe</TableHead>
                <TableHead className="text-right">Qty</TableHead>
                <TableHead className="text-right">Sebelum</TableHead>
                <TableHead className="text-right">Sesudah</TableHead>
                <TableHead>Dok</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {history?.map((t) => (
                <TableRow key={t.id}>
                  <TableCell>{formatDateTime(t.created_at)}</TableCell>
                  <TableCell>{TYPE_LABEL[t.type]}</TableCell>
                  <TableCell className="text-right">{t.qty}</TableCell>
                  <TableCell className="text-right text-muted-foreground">
                    {t.qty_before}
                  </TableCell>
                  <TableCell className="text-right font-medium">
                    {t.qty_after}
                  </TableCell>
                  <TableCell>{t.ref_doc ?? "-"}</TableCell>
                </TableRow>
              ))}
              {!history?.length && (
                <TableRow>
                  <TableCell colSpan={6} className="py-8 text-center text-muted-foreground">
                    Belum ada riwayat.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}
