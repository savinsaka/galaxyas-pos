// Peta key → komponen layar untuk stack navigasi (padanan mobile dari
// desktop views/registry.ts). Layar dasar tiap tab dirender langsung oleh
// shell; registry ini untuk sub-layar yang di-push (nav.push) — bertambah
// di Phase 2-4 (detail transaksi, form barang, opname, dst).
import type { Component } from "svelte";

export const SCREENS: Record<string, Component<any>> = {};
