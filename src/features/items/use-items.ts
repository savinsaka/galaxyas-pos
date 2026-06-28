import {
  useInfiniteQuery,
  useMutation,
  useQueryClient,
} from "@tanstack/react-query";
import { ipc } from "@/lib/ipc/commands";
import type { ItemInput, ItemPage } from "@/types";
import type { ItemFormValues } from "@/schemas";

const PAGE_SIZE = 100;

export interface ItemFilters {
  search: string;
  jenis: string | null;
  merek: string | null;
}

export function useItemsInfinite(filters: ItemFilters) {
  return useInfiniteQuery({
    queryKey: ["items", filters],
    initialPageParam: { cursor_name: null, cursor_id: null } as {
      cursor_name: string | null;
      cursor_id: string | null;
    },
    queryFn: ({ pageParam }) =>
      ipc.listItems({
        search: filters.search || null,
        jenis: filters.jenis,
        merek: filters.merek,
        limit: PAGE_SIZE,
        cursor_name: pageParam.cursor_name,
        cursor_id: pageParam.cursor_id,
      }),
    getNextPageParam: (last: ItemPage) =>
      last.next_cursor_id
        ? { cursor_name: last.next_cursor_name, cursor_id: last.next_cursor_id }
        : undefined,
  });
}

function toInput(values: ItemFormValues): ItemInput {
  return {
    kode_item: values.kode_item ?? null,
    barcode: values.barcode ?? null,
    nama_item: values.nama_item,
    jenis: values.jenis ?? null,
    merek: values.merek ?? null,
    satuan_dasar: values.satuan_dasar ?? null,
    harga_beli: values.harga_beli,
    harga_jual: values.harga_jual,
    diskon_persen: values.diskon_persen,
  };
}

export function useItemMutations() {
  const qc = useQueryClient();
  const invalidate = () => qc.invalidateQueries({ queryKey: ["items"] });

  const create = useMutation({
    mutationFn: (values: ItemFormValues) => ipc.createItem(toInput(values)),
    onSuccess: invalidate,
  });
  const update = useMutation({
    mutationFn: ({ id, values }: { id: string; values: ItemFormValues }) =>
      ipc.updateItem(id, toInput(values)),
    onSuccess: invalidate,
  });
  const remove = useMutation({
    mutationFn: (id: string) => ipc.deleteItem(id),
    onSuccess: invalidate,
  });
  const duplicate = useMutation({
    mutationFn: (id: string) => ipc.duplicateItem(id),
    onSuccess: invalidate,
  });
  const bulkUpsert = useMutation({
    mutationFn: (items: ItemInput[]) => ipc.bulkUpsertItems(items),
    onSuccess: invalidate,
  });
  const massUpdate = useMutation({
    mutationFn: (items: Array<{ id: string } & Partial<ItemInput>>) =>
      ipc.massUpdateItems(items),
    onSuccess: invalidate,
  });

  return { create, update, remove, duplicate, bulkUpsert, massUpdate };
}
