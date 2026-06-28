import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { ipc } from "@/lib/ipc/commands";
import type { SaleInput } from "@/types";

export function useStore() {
  return useQuery({ queryKey: ["store"], queryFn: () => ipc.getStore() });
}

export function usePrinterSettings() {
  return useQuery({
    queryKey: ["printer"],
    queryFn: () => ipc.getPrinterSettings(),
  });
}

export function useHeldSales() {
  return useQuery({
    queryKey: ["held-sales"],
    queryFn: () => ipc.listHeldSales(),
  });
}

export function useCreateSale() {
  const qc = useQueryClient();
  return useMutation({
    mutationFn: (input: SaleInput) => ipc.createSale(input),
    onSuccess: () => {
      qc.invalidateQueries({ queryKey: ["held-sales"] });
      qc.invalidateQueries({ queryKey: ["items"] });
      qc.invalidateQueries({ queryKey: ["sync-status"] });
    },
  });
}
