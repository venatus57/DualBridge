<script lang="ts">
  import type { InputView } from "../types";

  let { input, touchpad = [1920, 942] }: { input: InputView; touchpad?: [number, number] } = $props();

  const has = (b: string) => input.buttons.includes(b);
  const pos = (v: number) => ((v - 128) / 128) * 14;
</script>

<div class="live">
  <div class="trigger" title="L2">
    <div class="fill" style="height:{(input.l2 / 255) * 100}%"></div>
    <span>L2</span>
  </div>

  <svg viewBox="0 0 64 40" class="cluster" aria-hidden="true">
    <!-- D-pad -->
    <rect x="8" y="3" width="8" height="10" rx="2" class:on={has("dpad_up")} />
    <rect x="8" y="27" width="8" height="10" rx="2" class:on={has("dpad_down")} />
    <rect x="-2" y="15" width="10" height="8" rx="2" class:on={has("dpad_left")} />
    <rect x="16" y="15" width="10" height="8" rx="2" class:on={has("dpad_right")} />
    <!-- L1 / R1 -->
    <rect x="30" y="4" width="12" height="6" rx="3" class:on={has("l1")} />
    <rect x="46" y="4" width="12" height="6" rx="3" class:on={has("r1")} />
    <!-- Share / Options / PS -->
    <circle cx="36" cy="18" r="2.5" class:on={has("share")} />
    <circle cx="52" cy="18" r="2.5" class:on={has("options")} />
    <circle cx="44" cy="30" r="3" class:on={has("ps")} />
  </svg>

  <div class="stick" title="L">
    <div class="dot" class:on={has("l3")} style="transform:translate({pos(input.lx)}px,{pos(input.ly)}px)"></div>
  </div>

  <div class="touchpad" class:on={has("touchpad")}>
    {#each input.touch as f, i (i)}
      {#if f.active}
        <div class="finger" style="left:{(f.x / touchpad[0]) * 100}%;top:{(f.y / touchpad[1]) * 100}%"></div>
      {/if}
    {/each}
  </div>

  <div class="stick" title="R">
    <div class="dot" class:on={has("r3")} style="transform:translate({pos(input.rx)}px,{pos(input.ry)}px)"></div>
  </div>

  <div class="face">
    <span class="tri" class:on={has("triangle")}>△</span>
    <span class="sq" class:on={has("square")}>□</span>
    <span class="ci" class:on={has("circle")}>○</span>
    <span class="cr" class:on={has("cross")}>✕</span>
  </div>

  <div class="trigger" title="R2">
    <div class="fill" style="height:{(input.r2 / 255) * 100}%"></div>
    <span>R2</span>
  </div>
</div>

<style>
  .live {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 10px 12px;
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-radius: var(--radius-sm);
  }
  .trigger {
    position: relative;
    width: 14px;
    height: 44px;
    border-radius: 5px;
    background: var(--panel-2);
    overflow: hidden;
    flex: none;
  }
  .trigger .fill {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: linear-gradient(0deg, var(--accent), var(--accent-2));
  }
  .trigger span {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%) rotate(-90deg);
    font-size: 8px;
    font-weight: 700;
    color: #fff;
    mix-blend-mode: difference;
  }
  .cluster {
    width: 64px;
    flex: none;
  }
  .cluster :global(rect),
  .cluster :global(circle) {
    fill: var(--panel-2);
    stroke: var(--line);
  }
  .cluster :global(.on) {
    fill: var(--accent);
    stroke: var(--accent);
  }
  .stick {
    position: relative;
    width: 40px;
    height: 40px;
    border-radius: 50%;
    background: radial-gradient(circle, var(--panel-2) 0%, var(--bg-2) 100%);
    border: 1px solid var(--line);
    flex: none;
  }
  .dot {
    position: absolute;
    top: 50%;
    left: 50%;
    width: 12px;
    height: 12px;
    margin: -6px 0 0 -6px;
    border-radius: 50%;
    background: #d6dceb;
  }
  .dot.on {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .touchpad {
    position: relative;
    flex: 1;
    min-width: 60px;
    height: 40px;
    border-radius: 8px;
    background: var(--panel-2);
    border: 1px solid var(--line);
  }
  .touchpad.on {
    border-color: var(--accent);
    box-shadow: inset 0 0 12px rgba(77, 124, 255, 0.35);
  }
  .finger {
    position: absolute;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border-radius: 50%;
    background: var(--accent-2);
    box-shadow: 0 0 8px var(--accent-2);
  }
  .face {
    display: grid;
    grid-template-areas: ". t ." "s . c" ". x .";
    grid-template-columns: repeat(3, 13px);
    grid-template-rows: repeat(3, 13px);
    font-size: 11px;
    line-height: 13px;
    text-align: center;
    color: #5d6880;
    flex: none;
  }
  .tri {
    grid-area: t;
  }
  .sq {
    grid-area: s;
  }
  .ci {
    grid-area: c;
  }
  .cr {
    grid-area: x;
  }
  .face .on {
    font-weight: 800;
    text-shadow: 0 0 6px currentColor;
  }
  .tri.on {
    color: #3ee6b5;
  }
  .sq.on {
    color: #f08de0;
  }
  .ci.on {
    color: #ff6b6b;
  }
  .cr.on {
    color: #7aa2ff;
  }
</style>
