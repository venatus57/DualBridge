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

<div class="seg" role="radiogroup" aria-label={label}>
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
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
    padding: 3px;
    gap: 3px;
  }
  button {
    border: none;
    background: transparent;
    padding: 6px 12px;
    border-radius: 7px;
    color: var(--muted);
  }
  button:hover {
    background: var(--panel-2);
    color: var(--text);
  }
  button.active {
    background: var(--accent);
    color: #fff;
  }
</style>
