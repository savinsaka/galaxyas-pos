import { Plus, X } from "lucide-react";
import { useTabStore } from "@/stores/tab-store";
import { usePosStore } from "@/stores/pos-store";
import { cn } from "@/lib/utils";

export function TabBar() {
  const { tabs, activeId, setActive, closeTab, openPosTab } = useTabStore();
  const removeCart = usePosStore((s) => s.removeCart);

  const handleClose = (id: string, kind: string) => {
    if (kind === "pos") removeCart(id);
    closeTab(id);
  };

  return (
    <div className="flex h-10 shrink-0 items-center gap-1 border-b bg-card px-2">
      <div className="flex min-w-0 flex-1 items-center gap-1 overflow-x-auto">
        {tabs.map((t) => (
          <div
            key={t.id}
            onClick={() => setActive(t.id)}
            className={cn(
              "group flex h-8 max-w-48 shrink-0 cursor-pointer items-center gap-2 rounded-md border px-3 text-sm",
              t.id === activeId
                ? "border-primary/40 bg-primary/10 text-foreground"
                : "border-transparent text-muted-foreground hover:bg-accent",
            )}
          >
            <span className="truncate">{t.title}</span>
            {t.closable && (
              <button
                onClick={(e) => {
                  e.stopPropagation();
                  handleClose(t.id, t.kind);
                }}
                className="rounded p-0.5 opacity-60 hover:bg-background hover:opacity-100"
                aria-label="Tutup tab"
              >
                <X className="h-3.5 w-3.5" />
              </button>
            )}
          </div>
        ))}
      </div>
      <button
        onClick={() => openPosTab()}
        className="flex h-7 w-7 items-center justify-center rounded-md text-muted-foreground hover:bg-accent"
        title="Tab kasir baru (Ctrl+T)"
      >
        <Plus className="h-4 w-4" />
      </button>
    </div>
  );
}
