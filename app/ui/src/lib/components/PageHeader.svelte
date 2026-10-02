<script lang="ts">
  import type { Snippet } from "svelte";
  import Glyph, { type Shape } from "./Glyph.svelte";

  let {
    index,
    glyph,
    color,
    title,
    subtitle = "",
    children,
  }: { index: number; glyph: Shape; color: string; title: string; subtitle?: string; children?: Snippet } = $props();
</script>

<div class="page-header" style="--c:{color}">
  <div>
    <span class="kicker" style="color:{color}">
      <Glyph shape={glyph} size={11} stroke={3} />
      <span>0{index}</span>
    </span>
    <h1>{title}</h1>
    {#if subtitle}<p class="muted">{subtitle}</p>{/if}
  </div>
  {#if children}
    <div class="spacer"></div>
    {@render children()}
  {/if}
</div>

<style>
  .kicker {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .page-header::after {
    background: var(--c);
    box-shadow: 0 0 12px var(--c);
  }
</style>
