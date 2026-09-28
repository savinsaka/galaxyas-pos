<script lang="ts">
  import { formatCashInput, onMoneyInput, parseCashInput } from "$lib/moneyInput";

  /**
   * Input nominal uang laci (modal awal / uang fisik tutup shift): tampil
   * "Rp 100.000" alih-alih "100000", tanpa desimal. `value` tetap angka biasa.
   */
  let {
    value = $bindable(0),
    placeholder = "0",
    autofocus = false,
    onkeydown,
  }: {
    value?: number;
    placeholder?: string;
    autofocus?: boolean;
    onkeydown?: (e: KeyboardEvent) => void;
  } = $props();

  function onInput(e: Event) {
    const input = e.currentTarget as HTMLInputElement;
    onMoneyInput(
      e,
      (n) => {
        value = n;
        // Tulis ulang langsung: kalau angkanya tidak berubah (mis. yang diketik
        // huruf), Svelte tidak me-render ulang dan hurufnya akan tertinggal.
        input.value = formatCashInput(n);
      },
      parseCashInput,
    );
  }
</script>

<div class="cash-input">
  <span class="cash-prefix">Rp</span>
  <!-- svelte-ignore a11y_autofocus -->
  <input
    class="mono"
    type="text"
    inputmode="numeric"
    {placeholder}
    {autofocus}
    value={formatCashInput(value)}
    oninput={onInput}
    {onkeydown}
  />
</div>

<style>
  .cash-input { position: relative; display: flex; align-items: center; }
  .cash-prefix {
    position: absolute;
    left: 0.7rem;
    color: var(--text-dim);
    font-weight: 600;
    pointer-events: none;
  }
  .cash-input input { width: 100%; padding-left: 2.4rem; text-align: right; font-weight: 600; }
</style>
