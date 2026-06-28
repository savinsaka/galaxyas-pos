import { useHotkeys } from "react-hotkeys-hook";
import { useTabStore } from "@/stores/tab-store";
import { usePosStore } from "@/stores/pos-store";

/**
 * App-wide keyboard shortcuts (browser-like tab management). Feature-specific
 * shortcuts (F2 scan, F9 bayar, END, etc.) live inside the POS page so they only
 * fire when a cashier tab is active.
 */
export function useGlobalShortcuts() {
  const { openPosTab, closeTab, tabs, activeId, setActive } =
    useTabStore.getState();

  useHotkeys(
    "ctrl+t",
    (e) => {
      e.preventDefault();
      useTabStore.getState().openPosTab();
    },
    { enableOnFormTags: true },
    [openPosTab],
  );

  useHotkeys(
    "ctrl+w",
    (e) => {
      e.preventDefault();
      const { activeId: id, tabs: list } = useTabStore.getState();
      const tab = list.find((t) => t.id === id);
      if (tab?.closable) {
        if (tab.kind === "pos") usePosStore.getState().removeCart(tab.id);
        useTabStore.getState().closeTab(tab.id);
      }
    },
    { enableOnFormTags: true },
    [closeTab],
  );

  useHotkeys(
    "ctrl+tab",
    (e) => {
      e.preventDefault();
      const { tabs: list, activeId: id } = useTabStore.getState();
      if (list.length < 2) return;
      const idx = list.findIndex((t) => t.id === id);
      const next = list[(idx + 1) % list.length];
      useTabStore.getState().setActive(next.id);
    },
    { enableOnFormTags: true },
    [tabs, activeId, setActive],
  );
}
