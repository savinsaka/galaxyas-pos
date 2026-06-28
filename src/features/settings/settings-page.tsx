import { useEffect } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import { Printer, Store as StoreIcon } from "lucide-react";
import { toast } from "sonner";
import { z } from "zod";
import { ipc } from "@/lib/ipc/commands";
import { printerSettingsSchema, storeSettingsSchema } from "@/schemas";
import { useAuthStore } from "@/stores/auth-store";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

type StoreValues = z.infer<typeof storeSettingsSchema>;
type PrinterValues = z.infer<typeof printerSettingsSchema>;

export function SettingsPage() {
  const qc = useQueryClient();
  const canEdit = useAuthStore((s) => s.hasRole("admin", "supervisor"));
  const { data: store } = useQuery({ queryKey: ["store"], queryFn: () => ipc.getStore() });
  const { data: printer } = useQuery({
    queryKey: ["printer"],
    queryFn: () => ipc.getPrinterSettings(),
  });

  const storeForm = useForm<StoreValues>({
    resolver: zodResolver(storeSettingsSchema),
  });
  const printerForm = useForm<PrinterValues>({
    resolver: zodResolver(printerSettingsSchema),
  });

  useEffect(() => {
    if (store)
      storeForm.reset({
        name: store.name,
        address: store.address ?? "",
        phone: store.phone ?? "",
        tax_percent: store.tax_percent,
      });
  }, [store, storeForm]);

  useEffect(() => {
    if (printer) printerForm.reset(printer);
  }, [printer, printerForm]);

  const saveStore = async (v: StoreValues) => {
    await ipc.updateStore(v);
    toast.success("Pengaturan toko disimpan");
    qc.invalidateQueries({ queryKey: ["store"] });
  };

  const savePrinter = async (v: PrinterValues) => {
    await ipc.updatePrinterSettings(v);
    toast.success("Pengaturan printer disimpan");
    qc.invalidateQueries({ queryKey: ["printer"] });
  };

  return (
    <div className="mx-auto grid max-w-4xl gap-4 p-6 md:grid-cols-2">
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <StoreIcon className="h-4 w-4" /> Pengaturan Toko
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={storeForm.handleSubmit(saveStore)}
            className="space-y-3"
          >
            <Field label="Nama Toko" error={storeForm.formState.errors.name?.message}>
              <Input {...storeForm.register("name")} disabled={!canEdit} />
            </Field>
            <Field label="Alamat">
              <Input {...storeForm.register("address")} disabled={!canEdit} />
            </Field>
            <Field label="Telepon">
              <Input {...storeForm.register("phone")} disabled={!canEdit} />
            </Field>
            <Field
              label="Pajak (%)"
              error={storeForm.formState.errors.tax_percent?.message}
            >
              <Input
                type="number"
                step="any"
                {...storeForm.register("tax_percent")}
                disabled={!canEdit}
              />
            </Field>
            <Button type="submit" disabled={!canEdit}>
              Simpan
            </Button>
          </form>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <Printer className="h-4 w-4" /> Pengaturan Printer
          </CardTitle>
        </CardHeader>
        <CardContent>
          <form
            onSubmit={printerForm.handleSubmit(savePrinter)}
            className="space-y-3"
          >
            <Field
              label="Nama Printer"
              error={printerForm.formState.errors.printer_name?.message}
            >
              <Input {...printerForm.register("printer_name")} disabled={!canEdit} />
            </Field>
            <Field
              label="Lebar Kertas (mm)"
              error={printerForm.formState.errors.paper_width?.message}
            >
              <Input
                type="number"
                {...printerForm.register("paper_width")}
                disabled={!canEdit}
              />
            </Field>
            <Field label="Teks Header">
              <Input {...printerForm.register("header_text")} disabled={!canEdit} />
            </Field>
            <Field label="Teks Footer">
              <Input {...printerForm.register("footer_text")} disabled={!canEdit} />
            </Field>
            <Button type="submit" disabled={!canEdit}>
              Simpan
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}

function Field({
  label,
  error,
  children,
}: {
  label: string;
  error?: string;
  children: React.ReactNode;
}) {
  return (
    <div className="space-y-1.5">
      <Label>{label}</Label>
      {children}
      {error && <p className="text-xs text-destructive">{error}</p>}
    </div>
  );
}
