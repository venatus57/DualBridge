<script lang="ts">
  let {
    label,
    value = $bindable(),
    min = 0,
    max = 1,
    step = 0.01,
    format = (v: number) => String(v),
    hint = "",
    left = "",
    right = "",
    onchange,
  }: {
    label: string;
    value: number;
    min?: number;
    max?: number;
    step?: number;
    format?: (v: number) => string;
    hint?: string;
    left?: string;
    right?: string;
    onchange?: (v: number) => void;
  } = $props();

  const pct = $derived(((value - min) / (max - min)) * 100);
</script>

<div class="slider">
  <div class="top">
    <span class="label">{label}</span>
    <span class="value">{format(value)}</span>
  </div>
  <input
    type="range"
    {min}
    {max}
    {step}
    bind:value
    style="--pct: {pct}%"
    oninput={() => onchange?.(value)}
    aria-label={label}
  />
  {#if left || right}
    <div class="ends small muted"><span>{left}</span><span>{right}</span></div>
  {/if}
  {#if hint}<p class="hint">{hint}</p>{/if}
</div>

<style>
  .slider {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .top {
    display: flex;
    justify-content: space-between;
  }
  .value {
    font-variant-numeric: tabular-nums;
    color: var(--muted);
  }
  .ends {
    display: flex;
    justify-content: space-between;
    margin-top: -4px;
  }
  input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 6px;
    border-radius: 6px;
    background: linear-gradient(90deg, var(--accent) var(--pct), var(--panel-2) var(--pct));
    outline: none;
    cursor: pointer;
  }
  input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: #fff;
    border: 3px solid var(--accent);
    box-shadow: 0 2px 6px rgba(0, 0, 0, 0.4);
  }
  input[type="range"]::-moz-range-thumb {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: #fff;
    border: 3px solid var(--accent);
  }
</style>
