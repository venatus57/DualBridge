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
  <span class="switch" aria-hidden="true"></span>
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
    width: 40px;
    height: 23px;
    border-radius: 23px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    position: relative;
    transition: background 0.2s;
  }
  .switch::after {
    content: "";
    position: absolute;
    top: 2px;
    left: 2px;
    width: 17px;
    height: 17px;
    border-radius: 50%;
    background: #c9d2e3;
    transition: transform 0.2s, background 0.2s;
  }
  input:checked + .switch {
    background: var(--accent);
    border-color: var(--accent);
  }
  input:checked + .switch::after {
    transform: translateX(17px);
    background: #fff;
  }
  input:focus-visible + .switch {
    outline: 2px solid var(--accent);
    outline-offset: 2px;
  }
</style>
