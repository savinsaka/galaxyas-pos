import { Cloud, CloudOff, Loader2, RefreshCw, TriangleAlert } from "lucide-react";
import { toast } from "sonner";
import { useSyncStore } from "@/stores/sync-store";
import { cn } from "@/lib/utils";

export function SyncIndicator() {
  const status = useSyncStore((s) => s.status);
  const refreshing = useSyncStore((s) => s.refreshing);
  const triggerSync = useSyncStore((s) => s.triggerSync);

  const state = refreshing ? "syncing" : status?.state ?? "idle";
  const pending = status?.pending_count ?? 0;
  const conflicts = status?.conflict_count ?? 0;

  const config = {
    idle: { icon: Cloud, label: "Tersinkron", cls: "text-success" },
    syncing: { icon: Loader2, label: "Menyinkron…", cls: "text-primary" },
    offline: { icon: CloudOff, label: "Offline", cls: "text-muted-foreground" },
    error: { icon: TriangleAlert, label: "Error sync", cls: "text-destructive" },
  }[state];

  const Icon = config.icon;

  return (
    <button
      onClick={async () => {
        try {
          await triggerSync();
          toast.success("Sinkronisasi selesai");
        } catch (e) {
          toast.error("Sinkronisasi gagal", { description: String(e) });
        }
      }}
      className="flex items-center gap-2 rounded-md border px-2.5 py-1 text-xs hover:bg-accent"
      title="Klik untuk sinkron manual"
    >
      <Icon className={cn("h-4 w-4", config.cls, state === "syncing" && "animate-spin")} />
      <span>{config.label}</span>
      {pending > 0 && (
        <span className="rounded-full bg-warning px-1.5 text-[10px] font-semibold text-black">
          {pending}
        </span>
      )}
      {conflicts > 0 && (
        <span className="rounded-full bg-destructive px-1.5 text-[10px] font-semibold text-white">
          {conflicts}
        </span>
      )}
      <RefreshCw className="h-3 w-3 opacity-40" />
    </button>
  );
}
