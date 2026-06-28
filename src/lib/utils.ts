import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}

/** Format a number as Indonesian Rupiah currency. */
export function formatCurrency(value: number): string {
  return new Intl.NumberFormat("id-ID", {
    style: "currency",
    currency: "IDR",
    maximumFractionDigits: 0,
  }).format(value || 0);
}

/** Format an epoch-ms timestamp using the Indonesian locale. */
export function formatDateTime(epochMs: number | null | undefined): string {
  if (!epochMs) return "-";
  return new Intl.DateTimeFormat("id-ID", {
    dateStyle: "medium",
    timeStyle: "short",
  }).format(new Date(epochMs));
}

export function formatDate(epochMs: number | null | undefined): string {
  if (!epochMs) return "-";
  return new Intl.DateTimeFormat("id-ID", { dateStyle: "medium" }).format(
    new Date(epochMs),
  );
}

/** Current epoch milliseconds. Centralized so it is easy to mock in tests. */
export function nowMs(): number {
  return Date.now();
}
