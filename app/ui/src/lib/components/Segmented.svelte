<script lang="ts" generics="T extends string">
  let {
    options,
    value,
    onchange,
    label = "",
  }: {
    options: { value: T; label: string }[];
    value: T;
    onchange: (v: T) => void;
    label?: string;
  } = $props();
</script>

<div class="seg bevel" role="radiogroup" aria-label={label}>
  {#each options as o (o.value)}
    <button
      type="button"
      role="radio"
      aria-checked={o.value === value}
      class:active={o.value === value}
      onclick={() => onchange(o.value)}>{o.label}</button
    >
  {/each}
</div>

<style>
  .seg {
    display: inline-flex;
    flex-wrap: wrap;
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    border-radius: var(--cut-sm);
    padding: 3px;
    gap: 2px;
  }
  button {
    border: 1px solid transparent;
    background: transparent;
    padding: 6px 13px;
    border-radius: 4px;
    color: var(--muted);
  }
  button:hover {
    background: var(--panel-2);
    border-color: transparent;
    color: var(--text);
  }
  button.active {
    background: var(--accent-soft);
    border-color: color-mix(in srgb, var(--accent) 55%, transparent);
    color: var(--accent);
    box-shadow: inset 0 -2px 0 var(--accent);
  }
</style>
