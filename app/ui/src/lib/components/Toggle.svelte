<script lang="ts">
  let {
    label,
    checked = $bindable(),
    hint = "",
    disabled = false,
    onchange,
  }: {
    label: string;
    checked: boolean;
    hint?: string;
    disabled?: boolean;
    onchange?: (v: boolean) => void;
  } = $props();
</script>

<label class="toggle" class:disabled>
  <span class="text">
    <span>{label}</span>
    {#if hint}<span class="hint">{hint}</span>{/if}
  </span>
  <input type="checkbox" bind:checked {disabled} onchange={() => onchange?.(checked)} />
  <span class="switch bevel" aria-hidden="true"></span>
</label>

<style>
  .toggle {
    display: flex;
    align-items: center;
    gap: 14px;
    cursor: pointer;
  }
  .toggle.disabled {
    opacity: 0.5;
    cursor: default;
  }
  .text {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  input {
    position: absolute;
    opacity: 0;
    pointer-events: none;
  }
  .switch {
    flex: none;
    width: 42px;
    height: 22px;
    border-radius: 5px;
    background: var(--bg-2);
    border: 1px solid var(--line-2);
    position: relative;
    transition: background 0.2s, border-color 0.2s, box-shadow 0.2s;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 3px;
    left: 3px;
    width: 14px;
    height: 14px;
    border-radius: 3px;
    background: var(--dim);
    transition: transform 0.2s cubic-bezier(0.3, 0.7, 0.3, 1.3), background 0.2s;
  }
  input:checked + .switch {
    background: var(--accent-soft);
    border-color: var(--accent);
    box-shadow: 0 0 14px -4px var(--accent);
  }
  input:checked + .switch::after {
    transform: translateX(20px);
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  input:focus-visible + .switch {
    outline: 1px solid var(--accent);
    outline-offset: 2px;
  }
</style>
