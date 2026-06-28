import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import * as XLSX from "xlsx";
import { Download, TrendingUp } from "lucide-react";
import { ipc } from "@/lib/ipc/commands";
import { formatCurrency } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";

const startOfDay = (d: string) => new Date(`${d}T00:00:00`).getTime();
const endOfDay = (d: string) => new Date(`${d}T23:59:59`).getTime();

export function ReportsPage() {
  const today = new Date().toISOString().slice(0, 10);
  const [from, setFrom] = useState(today);
  const [to, setTo] = useState(today);

  const fromMs = startOfDay(from);
  const toMs = endOfDay(to);

  const daily = useQuery({
    queryKey: ["report-daily", fromMs, toMs],
    queryFn: () => ipc.dailySalesReport(fromMs, toMs),
  });
  const top = useQuery({
    queryKey: ["report-top", fromMs, toMs],
    queryFn: () => ipc.topSellingProducts(fromMs, toMs, 10),
  });
  const stock = useQuery({
    queryKey: ["report-stock"],
    queryFn: () => ipc.stockReport(),
  });

  const exportSheet = (name: string, rows: object[]) => {
    const ws = XLSX.utils.json_to_sheet(rows);
    const wb = XLSX.utils.book_new();
    XLSX.utils.book_append_sheet(wb, ws, name);
    XLSX.writeFile(wb, `${name}-${today}.xlsx`);
  };

  return (
    <div className="space-y-4 overflow-auto p-4">
      <div className="flex items-end gap-3">
        <div>
          <label className="text-xs text-muted-foreground">Dari</label>
          <Input type="date" value={from} onChange={(e) => setFrom(e.target.value)} />
        </div>
        <div>
          <label className="text-xs text-muted-foreground">Sampai</label>
          <Input type="date" value={to} onChange={(e) => setTo(e.target.value)} />
        </div>
      </div>

      <Card>
        <CardHeader className="flex-row items-center justify-between space-y-0">
          <CardTitle className="text-base">Penjualan Harian</CardTitle>
          <Button
            size="sm"
            variant="outline"
            onClick={() => exportSheet("penjualan-harian", daily.data ?? [])}
          >
            <Download /> Excel
          </Button>
        </CardHeader>
        <CardContent>
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Tanggal</TableHead>
                <TableHead className="text-right">Transaksi</TableHead>
                <TableHead className="text-right">Diskon</TableHead>
                <TableHead className="text-right">Pajak</TableHead>
                <TableHead className="text-right">Pendapatan</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {daily.data?.map((r) => (
                <TableRow key={r.date}>
                  <TableCell>{r.date}</TableCell>
                  <TableCell className="text-right">{r.total_transactions}</TableCell>
                  <TableCell className="text-right">{formatCurrency(r.total_discount)}</TableCell>
                  <TableCell className="text-right">{formatCurrency(r.total_tax)}</TableCell>
                  <TableCell className="text-right font-medium">
                    {formatCurrency(r.total_revenue)}
                  </TableCell>
                </TableRow>
              ))}
              {!daily.data?.length && (
                <TableRow>
                  <TableCell colSpan={5} className="py-6 text-center text-muted-foreground">
                    Tidak ada data pada periode ini.
                  </TableCell>
                </TableRow>
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>

      <div className="grid gap-4 md:grid-cols-2">
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2 text-base">
              <TrendingUp className="h-4 w-4" /> Produk Terlaris
            </CardTitle>
          </CardHeader>
          <CardContent>
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Barang</TableHead>
                  <TableHead className="text-right">Qty</TableHead>
                  <TableHead className="text-right">Omzet</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {top.data?.map((r) => (
                  <TableRow key={r.item_id}>
                    <TableCell className="truncate">{r.nama_item}</TableCell>
                    <TableCell className="text-right">{r.total_qty}</TableCell>
                    <TableCell className="text-right">
                      {formatCurrency(r.total_revenue)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>

        <Card>
          <CardHeader className="flex-row items-center justify-between space-y-0">
            <CardTitle className="text-base">Laporan Stok</CardTitle>
            <Button
              size="sm"
              variant="outline"
              onClick={() => exportSheet("laporan-stok", stock.data ?? [])}
            >
              <Download /> Excel
            </Button>
          </CardHeader>
          <CardContent className="max-h-80 overflow-auto">
            <Table>
              <TableHeader>
                <TableRow>
                  <TableHead>Barang</TableHead>
                  <TableHead className="text-right">Stok</TableHead>
                  <TableHead className="text-right">Nilai</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {stock.data?.map((r) => (
                  <TableRow key={r.item_id}>
                    <TableCell className="truncate">{r.nama_item}</TableCell>
                    <TableCell className="text-right">{r.stok}</TableCell>
                    <TableCell className="text-right">
                      {formatCurrency(r.nilai_stok)}
                    </TableCell>
                  </TableRow>
                ))}
              </TableBody>
            </Table>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
