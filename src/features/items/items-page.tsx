import { useMemo, useRef, useState } from "react";
import { useVirtualizer } from "@tanstack/react-virtual";
import {
  Copy,
  Download,
  FileSpreadsheet,
  Loader2,
  Pencil,
  Plus,
  Search,
  Trash2,
  Upload,
} from "lucide-react";
import { toast } from "sonner";
import type { Item } from "@/types";
import { useAuthStore } from "@/stores/auth-store";
import { formatCurrency } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { ItemFormDialog } from "./item-form-dialog";
import {
  downloadImportTemplate,
  exportItemsExcel,
  parseItemsExcel,
} from "./excel";
import {
  useItemMutations,
  useItemsInfinite,
  type ItemFilters,
} from "./use-items";

export function ItemsPage() {
  const [filters, setFilters] = useState<ItemFilters>({
    search: "",
    jenis: null,
    merek: null,
  });
  const [dialogOpen, setDialogOpen] = useState(false);
  const [editing, setEditing] = useState<Item | null>(null);
  const fileRef = useRef<HTMLInputElement>(null);
  const canManage = useAuthStore((s) => s.hasRole("admin", "supervisor"));

  const {
    data,
    fetchNextPage,
    hasNextPage,
    isFetchingNextPage,
    isLoading,
  } = useItemsInfinite(filters);
  const { remove, duplicate, bulkUpsert } = useItemMutations();

  const items = useMemo(
    () => data?.pages.flatMap((p) => p.items) ?? [],
    [data],
  );
  const total = data?.pages[0]?.total ?? 0;

  const parentRef = useRef<HTMLDivElement>(null);
  const rowVirtualizer = useVirtualizer({
    count: items.length,
    getScrollElement: () => parentRef.current,
    estimateSize: () => 44,
    overscan: 12,
  });

  const handleScroll = () => {
    const el = parentRef.current;
    if (!el || !hasNextPage || isFetchingNextPage) return;
    if (el.scrollHeight - el.scrollTop - el.clientHeight < 300) {
      void fetchNextPage();
    }
  };

  const handleImport = async (file: File) => {
    const { valid, errors } = await parseItemsExcel(file);
    if (errors.length) {
      toast.warning(`${errors.length} baris tidak valid`, {
        description: errors.slice(0, 3).join("\n"),
      });
    }
    if (valid.length) {
      const res = await bulkUpsert.mutateAsync(valid);
      toast.success(
        `Import selesai: ${res.inserted} baru, ${res.updated} diperbarui`,
      );
    }
  };

  return (
    <div className="flex h-full flex-col p-4">
      <div className="mb-3 flex items-center justify-between gap-2">
        <div>
          <h1 className="text-lg font-semibold">Master Barang</h1>
          <p className="text-xs text-muted-foreground">
            {total.toLocaleString("id-ID")} item
          </p>
        </div>
        <div className="flex items-center gap-2">
          <input
            ref={fileRef}
            type="file"
            accept=".xlsx,.xls,.csv"
            className="hidden"
            onChange={(e) => {
              const f = e.target.files?.[0];
              if (f) void handleImport(f);
              e.target.value = "";
            }}
          />
          <Button
            variant="outline"
            size="sm"
            onClick={() => downloadImportTemplate()}
          >
            <FileSpreadsheet /> Template
          </Button>
          <Button
            variant="outline"
            size="sm"
            disabled={!canManage}
            onClick={() => fileRef.current?.click()}
          >
            <Upload /> Import
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => exportItemsExcel(items)}
          >
            <Download /> Export
          </Button>
          <Button
            size="sm"
            disabled={!canManage}
            onClick={() => {
              setEditing(null);
              setDialogOpen(true);
            }}
          >
            <Plus /> Tambah
          </Button>
        </div>
      </div>

      <div className="mb-3 flex items-center gap-2">
        <div className="relative max-w-sm flex-1">
          <Search className="absolute left-2.5 top-2.5 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder="Cari nama / barcode / kode…"
            className="pl-8"
            value={filters.search}
            onChange={(e) =>
              setFilters((f) => ({ ...f, search: e.target.value }))
            }
          />
        </div>
      </div>

      <div className="flex min-h-0 flex-1 flex-col rounded-lg border">
        <div className="grid grid-cols-[1fr_120px_120px_110px_110px_90px_120px] gap-2 border-b bg-muted/40 px-3 py-2 text-xs font-medium text-muted-foreground">
          <div>Nama</div>
          <div>Kode</div>
          <div>Barcode</div>
          <div className="text-right">Harga Jual</div>
          <div className="text-right">Stok</div>
          <div className="text-center">Sync</div>
          <div className="text-right">Aksi</div>
        </div>

        {isLoading ? (
          <div className="flex flex-1 items-center justify-center">
            <Loader2 className="h-6 w-6 animate-spin text-primary" />
          </div>
        ) : (
          <div
            ref={parentRef}
            onScroll={handleScroll}
            className="flex-1 overflow-auto"
          >
            <div
              style={{ height: rowVirtualizer.getTotalSize(), position: "relative" }}
            >
              {rowVirtualizer.getVirtualItems().map((vr) => {
                const item = items[vr.index];
                return (
                  <div
                    key={item.id}
                    className="absolute left-0 top-0 grid w-full grid-cols-[1fr_120px_120px_110px_110px_90px_120px] items-center gap-2 border-b px-3 text-sm hover:bg-muted/30"
                    style={{
                      height: vr.size,
                      transform: `translateY(${vr.start}px)`,
                    }}
                  >
                    <div className="truncate font-medium">{item.nama_item}</div>
                    <div className="truncate text-muted-foreground">
                      {item.kode_item ?? "-"}
                    </div>
                    <div className="truncate text-muted-foreground">
                      {item.barcode ?? "-"}
                    </div>
                    <div className="text-right">
                      {formatCurrency(item.harga_jual)}
                    </div>
                    <div className="text-right">{item.stok}</div>
                    <div className="text-center">
                      <Badge
                        variant={
                          item.sync_status === "synced"
                            ? "success"
                            : item.sync_status === "conflict"
                              ? "destructive"
                              : "warning"
                        }
                      >
                        {item.sync_status}
                      </Badge>
                    </div>
                    <div className="flex justify-end gap-1">
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-7 w-7"
                        onClick={() => {
                          setEditing(item);
                          setDialogOpen(true);
                        }}
                      >
                        <Pencil className="h-3.5 w-3.5" />
                      </Button>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-7 w-7"
                        onClick={async () => {
                          await duplicate.mutateAsync(item.id);
                          toast.success("Barang diduplikasi");
                        }}
                      >
                        <Copy className="h-3.5 w-3.5" />
                      </Button>
                      <Button
                        size="icon"
                        variant="ghost"
                        className="h-7 w-7 text-destructive"
                        disabled={!canManage}
                        onClick={async () => {
                          if (confirm(`Hapus ${item.nama_item}?`)) {
                            await remove.mutateAsync(item.id);
                            toast.success("Barang dihapus");
                          }
                        }}
                      >
                        <Trash2 className="h-3.5 w-3.5" />
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
            {isFetchingNextPage && (
              <div className="flex justify-center py-2">
                <Loader2 className="h-4 w-4 animate-spin text-primary" />
              </div>
            )}
          </div>
        )}
      </div>

      <ItemFormDialog
        open={dialogOpen}
        onOpenChange={setDialogOpen}
        item={editing}
      />
    </div>
  );
}
