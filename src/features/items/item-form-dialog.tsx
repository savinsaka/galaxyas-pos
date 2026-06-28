import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { toast } from "sonner";
import { itemSchema, type ItemFormValues } from "@/schemas";
import type { Item } from "@/types";
import { useItemMutations } from "./use-items";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface Props {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  item?: Item | null;
}

const FIELDS: Array<{
  name: keyof ItemFormValues;
  label: string;
  type?: string;
}> = [
  { name: "kode_item", label: "Kode Item" },
  { name: "barcode", label: "Barcode" },
  { name: "nama_item", label: "Nama Item" },
  { name: "jenis", label: "Jenis" },
  { name: "merek", label: "Merek" },
  { name: "satuan_dasar", label: "Satuan" },
  { name: "harga_beli", label: "Harga Beli", type: "number" },
  { name: "harga_jual", label: "Harga Jual", type: "number" },
  { name: "diskon_persen", label: "Diskon (%)", type: "number" },
];

export function ItemFormDialog({ open, onOpenChange, item }: Props) {
  const { create, update } = useItemMutations();
  const {
    register,
    handleSubmit,
    reset,
    formState: { errors },
  } = useForm<ItemFormValues>({
    resolver: zodResolver(itemSchema),
  });

  useEffect(() => {
    if (open) {
      reset({
        kode_item: item?.kode_item ?? "",
        barcode: item?.barcode ?? "",
        nama_item: item?.nama_item ?? "",
        jenis: item?.jenis ?? "",
        merek: item?.merek ?? "",
        satuan_dasar: item?.satuan_dasar ?? "PCS",
        harga_beli: item?.harga_beli ?? 0,
        harga_jual: item?.harga_jual ?? 0,
        diskon_persen: item?.diskon_persen ?? 0,
      });
    }
  }, [open, item, reset]);

  const onSubmit = async (values: ItemFormValues) => {
    try {
      if (item) {
        await update.mutateAsync({ id: item.id, values });
        toast.success("Barang diperbarui");
      } else {
        await create.mutateAsync(values);
        toast.success("Barang ditambahkan");
      }
      onOpenChange(false);
    } catch (e) {
      toast.error("Gagal menyimpan", { description: String(e) });
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle>{item ? "Edit Barang" : "Tambah Barang"}</DialogTitle>
        </DialogHeader>
        <form onSubmit={handleSubmit(onSubmit)} className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            {FIELDS.map((f) => (
              <div key={f.name} className="space-y-1.5">
                <Label htmlFor={f.name}>{f.label}</Label>
                <Input
                  id={f.name}
                  type={f.type ?? "text"}
                  step={f.type === "number" ? "any" : undefined}
                  {...register(f.name)}
                />
                {errors[f.name] && (
                  <p className="text-xs text-destructive">
                    {errors[f.name]?.message as string}
                  </p>
                )}
              </div>
            ))}
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
            >
              Batal
            </Button>
            <Button type="submit">Simpan</Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
