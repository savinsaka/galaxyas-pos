import { useQuery, useQueryClient } from "@tanstack/react-query";
import { CheckCircle2, RefreshCw, TriangleAlert } from "lucide-react";
import { toast } from "sonner";
import { ipc } from "@/lib/ipc/commands";
import { useSyncStore } from "@/stores/sync-store";
import { useAuthStore } from "@/stores/auth-store";
import { formatDateTime } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import type { SyncConflict } from "@/types";

export function SyncPage() {
  const qc = useQueryClient();
  const status = useSyncStore((s) => s.status);
  const triggerSync = useSyncStore((s) => s.triggerSync);
  const refreshing = useSyncStore((s) => s.refreshing);
  const canResolve = useAuthStore((s) => s.hasRole("admin", "supervisor"));

  const { data: conflicts } = useQuery({
    queryKey: ["conflicts"],
    queryFn: () => ipc.listConflicts(),
  });

  const resolve = async (
    c: SyncConflict,
    resolution: "kept_local" | "kept_server",
  ) => {
    await ipc.resolveConflict(c.id, resolution);
    toast.success("Konflik diselesaikan");
    qc.invalidateQueries({ queryKey: ["conflicts"] });
  };

  const pending = conflicts?.filter((c) => c.resolution === "pending") ?? [];

  return (
    <div className="mx-auto max-w-3xl space-y-4 overflow-auto p-6">
      <Card>
        <CardHeader className="flex-row items-center justify-between space-y-0">
          <div>
            <CardTitle className="text-base">Status Sinkronisasi</CardTitle>
            <CardDescription>
              Terakhir sinkron: {formatDateTime(status?.last_sync_at)}
            </CardDescription>
          </div>
          <Button
            onClick={async () => {
              await triggerSync();
              toast.success("Sinkronisasi dijalankan");
            }}
            disabled={refreshing}
          >
            <RefreshCw className={refreshing ? "animate-spin" : ""} /> Sinkron
            Sekarang
          </Button>
        </CardHeader>
        <CardContent className="grid grid-cols-3 gap-4">
          <Stat label="Antrian" value={status?.pending_count ?? 0} />
          <Stat label="Konflik" value={status?.conflict_count ?? 0} />
          <Stat label="Status" value={status?.state ?? "-"} />
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2 text-base">
            <TriangleAlert className="h-4 w-4 text-warning" /> Konflik Data (
            {pending.length})
          </CardTitle>
          <CardDescription>
            Tinjau dan pilih versi mana yang dipertahankan.
          </CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {pending.length === 0 ? (
            <div className="flex items-center gap-2 text-sm text-muted-foreground">
              <CheckCircle2 className="h-4 w-4 text-success" />
              Tidak ada konflik yang perlu ditinjau.
            </div>
          ) : (
            pending.map((c) => (
              <div key={c.id} className="rounded-lg border p-3">
                <div className="mb-2 flex items-center justify-between">
                  <Badge variant="outline">
                    {c.entity_type} · {c.conflict_field ?? "data"}
                  </Badge>
                  <span className="text-xs text-muted-foreground">
                    {formatDateTime(c.created_at)}
                  </span>
                </div>
                <div className="grid grid-cols-2 gap-2 text-xs">
                  <pre className="max-h-32 overflow-auto rounded bg-muted p-2">
                    Lokal: {c.local_payload}
                  </pre>
                  <pre className="max-h-32 overflow-auto rounded bg-muted p-2">
                    Server: {c.server_payload}
                  </pre>
                </div>
                <div className="mt-2 flex justify-end gap-2">
                  <Button
                    size="sm"
                    variant="outline"
                    disabled={!canResolve}
                    onClick={() => resolve(c, "kept_local")}
                  >
                    Pakai Lokal
                  </Button>
                  <Button
                    size="sm"
                    disabled={!canResolve}
                    onClick={() => resolve(c, "kept_server")}
                  >
                    Pakai Server
                  </Button>
                </div>
              </div>
            ))
          )}
        </CardContent>
      </Card>
    </div>
  );
}

function Stat({ label, value }: { label: string; value: string | number }) {
  return (
    <div className="rounded-lg border p-3 text-center">
      <div className="text-2xl font-bold">{value}</div>
      <div className="text-xs text-muted-foreground">{label}</div>
    </div>
  );
}
