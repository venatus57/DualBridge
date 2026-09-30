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
    gap: 8px;
  }
  .top {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .value {
    font-family: var(--font-mono);
    font-size: 12.5px;
    font-variant-numeric: tabular-nums;
    color: var(--accent);
  }
  .ends {
    display: flex;
    justify-content: space-between;
    margin-top: -2px;
    font-size: 11px;
  }
  input[type="range"] {
    -webkit-appearance: none;
    appearance: none;
    width: 100%;
    height: 4px;
    margin: 6px 0;
    background:
      linear-gradient(90deg, var(--accent) var(--pct), transparent var(--pct)),
      repeating-linear-gradient(90deg, var(--line-2) 0 2px, transparent 2px 10%),
      var(--panel-2);
    outline: none;
    cursor: pointer;
  }
  input[type="range"]::-webkit-slider-thumb {
    -webkit-appearance: none;
    width: 14px;
    height: 14px;
    transform: rotate(45deg);
    background: var(--text);
    border: 2px solid var(--accent);
    box-shadow: 0 0 10px -1px var(--accent);
  }
  input[type="range"]:focus-visible::-webkit-slider-thumb {
    outline: 1px solid var(--accent);
    outline-offset: 3px;
  }
  input[type="range"]::-moz-range-thumb {
    width: 12px;
    height: 12px;
    transform: rotate(45deg);
    border-radius: 0;
    background: var(--text);
    border: 2px solid var(--accent);
  }
</style>
