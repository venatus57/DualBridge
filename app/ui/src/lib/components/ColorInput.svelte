<script lang="ts">
  import { fromHex, toHex } from "../color";
  import type { Rgb } from "../types";

  let {
    value,
    onchange,
    label = "",
    presets = true,
  }: { value: Rgb; onchange: (c: Rgb) => void; label?: string; presets?: boolean } = $props();

  const PRESETS = [
    "#FF1010", "#FF7800", "#FFD200", "#00DC3C", "#00DCDC",
    "#0040FF", "#8C28FF", "#FF28A0", "#FFFFFF",
  ];
  const hex = $derived(toHex(value));
</script>

<div class="color-input">
  {#if label}<span class="label">{label}</span>{/if}
  <div class="row">
    <label class="swatch" style="background:{hex}; color:{hex}" title={hex}>
      <input type="color" value={hex} oninput={(e) => onchange(fromHex(e.currentTarget.value))} aria-label={label} />
    </label>
    {#if presets}
      <div class="presets">
        {#each PRESETS as p (p)}
          <button
            type="button"
            class="preset"
            class:active={p === hex}
            style="background:{p}"
            aria-label={p}
            onclick={() => onchange(fromHex(p))}
          ></button>
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .color-input {
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    flex-wrap: wrap;
  }
  .swatch {
    width: 44px;
    height: 44px;
    border-radius: 12px;
    border: 2px solid rgba(255, 255, 255, 0.25);
    cursor: pointer;
    position: relative;
    box-shadow: 0 0 18px -4px currentColor;
  }
  .swatch input {
    position: absolute;
    inset: 0;
    opacity: 0;
    cursor: pointer;
  }
  .presets {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .preset {
    width: 24px;
    height: 24px;
    padding: 0;
    border-radius: 50%;
    border: 2px solid transparent;
  }
  .preset.active {
    border-color: #fff;
    box-shadow: 0 0 0 2px var(--accent);
  }
</style>
