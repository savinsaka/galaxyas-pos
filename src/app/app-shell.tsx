import { useEffect } from "react";
import {
  Boxes,
  ClipboardList,
  LayoutGrid,
  Package,
  RefreshCw,
  Settings,
  ShoppingCart,
  Wallet,
} from "lucide-react";
import { useTabStore, type TabKind } from "@/stores/tab-store";
import { useSyncStore } from "@/stores/sync-store";
import { useAuthStore } from "@/stores/auth-store";
import { Button } from "@/components/ui/button";
import { cn } from "@/lib/utils";
import { TabBar } from "./tab-bar";
import { SyncIndicator } from "@/features/sync/sync-indicator";
import { UserMenu } from "@/features/auth/user-menu";
import { useGlobalShortcuts } from "./use-global-shortcuts";
import { useUpdater } from "./use-updater";

import { PosPage } from "@/features/pos/pos-page";
import { ItemsPage } from "@/features/items/items-page";
import { InventoryPage } from "@/features/inventory/inventory-page";
import { ReportsPage } from "@/features/reports/reports-page";
import { SyncPage } from "@/features/sync/sync-page";
import { ShiftsPage } from "@/features/shifts/shifts-page";
import { AuditPage } from "@/features/audit/audit-page";
import { SettingsPage } from "@/features/settings/settings-page";

interface NavItem {
  kind: TabKind;
  label: string;
  icon: React.ComponentType<{ className?: string }>;
}

const NAV: NavItem[] = [
  { kind: "pos", label: "Kasir", icon: ShoppingCart },
  { kind: "items", label: "Barang", icon: Package },
  { kind: "inventory", label: "Stok", icon: Boxes },
  { kind: "shifts", label: "Shift", icon: Wallet },
  { kind: "reports", label: "Laporan", icon: LayoutGrid },
  { kind: "sync", label: "Sinkron", icon: RefreshCw },
  { kind: "audit", label: "Audit", icon: ClipboardList },
  { kind: "settings", label: "Setelan", icon: Settings },
];

function TabContent({ kind, tabId }: { kind: TabKind; tabId: string }) {
  switch (kind) {
    case "pos":
      return <PosPage tabId={tabId} />;
    case "items":
      return <ItemsPage />;
    case "inventory":
      return <InventoryPage />;
    case "reports":
      return <ReportsPage />;
    case "sync":
      return <SyncPage />;
    case "shifts":
      return <ShiftsPage />;
    case "audit":
      return <AuditPage />;
    case "settings":
      return <SettingsPage />;
  }
}

export function AppShell() {
  const { tabs, activeId, openTab } = useTabStore();
  const refresh = useSyncStore((s) => s.refresh);
  const storeName = useAuthStore((s) => s.session?.user.store_id);
  useGlobalShortcuts();
  useUpdater();

  // Open a first POS tab on launch and poll sync status.
  useEffect(() => {
    if (useTabStore.getState().tabs.length === 0) openTab("pos");
  }, [openTab]);

  useEffect(() => {
    void refresh();
    const id = setInterval(() => void refresh(), 15_000);
    return () => clearInterval(id);
  }, [refresh]);

  const activeTab = tabs.find((t) => t.id === activeId) ?? null;

  return (
    <div className="flex h-full flex-col bg-background">
      {/* Top bar */}
      <header className="flex h-12 shrink-0 items-center justify-between border-b px-3">
        <div className="flex items-center gap-2 font-semibold">
          <div className="flex h-7 w-7 items-center justify-center rounded-md bg-primary text-primary-foreground">
            G
          </div>
          <span>GalaxyAS POS</span>
          <span className="text-xs text-muted-foreground">
            {storeName ? `· ${storeName}` : ""}
          </span>
        </div>
        <div className="flex items-center gap-3">
          <SyncIndicator />
          <UserMenu />
        </div>
      </header>

      <div className="flex min-h-0 flex-1">
        {/* Sidebar */}
        <nav className="flex w-16 shrink-0 flex-col items-center gap-1 border-r py-2">
          {NAV.map((n) => {
            const active = activeTab?.kind === n.kind;
            return (
              <button
                key={n.kind}
                onClick={() => openTab(n.kind)}
                className={cn(
                  "flex w-14 flex-col items-center gap-1 rounded-md py-2 text-[10px] transition-colors",
                  active
                    ? "bg-primary/10 text-primary"
                    : "text-muted-foreground hover:bg-accent",
                )}
              >
                <n.icon className="h-5 w-5" />
                {n.label}
              </button>
            );
          })}
        </nav>

        {/* Main area */}
        <div className="flex min-w-0 flex-1 flex-col">
          <TabBar />
          <main className="min-h-0 flex-1 overflow-hidden">
            {tabs.length === 0 ? (
              <div className="flex h-full flex-col items-center justify-center gap-3 text-muted-foreground">
                <p>Tidak ada tab terbuka.</p>
                <Button onClick={() => openTab("pos")}>Buka Kasir</Button>
              </div>
            ) : (
              tabs.map((t) => (
                <div
                  key={t.id}
                  className={cn("h-full", t.id === activeId ? "block" : "hidden")}
                >
                  <TabContent kind={t.kind} tabId={t.id} />
                </div>
              ))
            )}
          </main>
        </div>
      </div>
    </div>
  );
}
