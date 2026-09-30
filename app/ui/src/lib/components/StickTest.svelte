<script lang="ts">
  import type { StickConfig } from "../types";

  // Shows the live stick position over the configured dead zone and outer
  // radius, and where the game sees it after mapping.
  let { config, x, y }: { config: StickConfig; x?: number; y?: number } = $props();

  const R = 70;
  const axis = (v: number) => (v >= 128 ? (v - 128) / 127 : (v - 128) / 128);
  const raw = $derived(x === undefined || y === undefined ? null : { x: axis(x), y: axis(y) });

  // Same math as `mapping::map_stick` (screen coordinates: y down).
  const mapped = $derived.by(() => {
    if (!raw) return null;
    let vx = config.invert_x ? -raw.x : raw.x;
    let vy = config.invert_y ? -raw.y : raw.y;
    const mag = Math.hypot(vx, vy);
    const dz = Math.min(0.99, Math.max(0, config.deadzone));
    if (mag <= dz || mag === 0) return { x: 0, y: 0 };
    const outer = Math.min(1, Math.max(dz + 0.01, config.outer));
    let tt = Math.min(1, (mag - dz) / (outer - dz));
    tt = Math.pow(tt, Math.min(10, Math.max(0.1, config.curve)));
    const ad = Math.min(0.99, Math.max(0, config.anti_deadzone));
    const out = ad + (1 - ad) * tt;
    const k = out / mag;
    return { x: Math.max(-1, Math.min(1, vx * k)), y: Math.max(-1, Math.min(1, vy * k)) };
  });
</script>

<svg viewBox="-80 -80 160 160" width="170" height="170" aria-hidden="true">
  <circle r={R} class="base" />
  <circle r={R * config.outer} class="outer" />
  <circle r={R * config.deadzone} class="dead" />
  <line x1={-R} y1="0" x2={R} y2="0" class="axis" />
  <line x1="0" y1={-R} x2="0" y2={R} class="axis" />
  {#if raw}
    <circle cx={raw.x * R} cy={raw.y * R} r="6" class="raw" />
  {/if}
  {#if mapped}
    <circle cx={mapped.x * R} cy={mapped.y * R} r="4.5" class="mapped" />
  {/if}
</svg>

<style>
  svg {
    display: block;
  }
  .base {
    fill: var(--bg-2);
    stroke: var(--line);
  }
  .outer {
    fill: none;
    stroke: var(--accent);
    stroke-dasharray: 3 3;
    opacity: 0.7;
  }
  .dead {
    fill: rgba(255, 79, 98, 0.16);
    stroke: var(--bad);
  }
  .axis {
    stroke: var(--line);
  }
  .raw {
    fill: none;
    stroke: #d6dceb;
    stroke-width: 2;
  }
  .mapped {
    fill: var(--accent);
    filter: drop-shadow(0 0 4px var(--accent));
  }
</style>
