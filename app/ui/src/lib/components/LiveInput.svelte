<script lang="ts">
  import type { InputView } from "../types";
  import Glyph from "./Glyph.svelte";

  let { input, touchpad = [1920, 942] }: { input: InputView; touchpad?: [number, number] } = $props();

  const has = (b: string) => input.buttons.includes(b);
  const pos = (v: number) => ((v - 128) / 128) * 14;
</script>

<div class="live bevel">
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

  <div class="touchpad bevel" class:on={has("touchpad")}>
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
    <span class="tri" class:on={has("triangle")}><Glyph shape="triangle" size={11} stroke={3} /></span>
    <span class="sq" class:on={has("square")}><Glyph shape="square" size={11} stroke={3} /></span>
    <span class="ci" class:on={has("circle")}><Glyph shape="circle" size={11} stroke={3} /></span>
    <span class="cr" class:on={has("cross")}><Glyph shape="cross" size={11} stroke={3} /></span>
  </div>

  <div class="trigger" title="R2">
    <div class="fill" style="height:{(input.r2 / 255) * 100}%"></div>
    <span>R2</span>
  </div>
</div>

<style>
  /* Pressed inputs light up in the controller's own color when there is one. */
  .live {
    --hi: var(--light, var(--accent));
    position: relative;
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 12px 14px;
    background:
      linear-gradient(90deg, rgba(255, 255, 255, 0.02) 1px, transparent 1px) 0 0 / 8px 100%,
      #0a0c10;
    border: 1px solid var(--line);
    border-radius: var(--cut-sm);
  }
  .trigger {
    position: relative;
    width: 14px;
    height: 46px;
    background: var(--panel-2);
    border: 1px solid var(--line);
    overflow: hidden;
    flex: none;
  }
  .trigger .fill {
    position: absolute;
    bottom: 0;
    left: 0;
    right: 0;
    background: var(--hi);
    box-shadow: 0 0 10px var(--hi);
  }
  .trigger span {
    position: absolute;
    top: 50%;
    left: 50%;
    transform: translate(-50%, -50%) rotate(-90deg);
    font-family: var(--font-mono);
    font-size: 8px;
    font-weight: 500;
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
    stroke: var(--line-2);
  }
  .cluster :global(.on) {
    fill: var(--hi);
    stroke: var(--hi);
    filter: drop-shadow(0 0 3px var(--hi));
  }
  .stick {
    position: relative;
    width: 42px;
    height: 42px;
    border-radius: 50%;
    background:
      linear-gradient(var(--line) 0 0) center / 1px 100% no-repeat,
      linear-gradient(var(--line) 0 0) center / 100% 1px no-repeat,
      radial-gradient(circle, var(--panel-2) 0%, #0a0c10 100%);
    border: 1px solid var(--line-2);
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
    background: #dfe3ec;
    box-shadow: 0 0 6px rgba(255, 255, 255, 0.35);
  }
  .dot.on {
    background: var(--hi);
    box-shadow: 0 0 10px var(--hi);
  }
  .touchpad {
    position: relative;
    flex: 1;
    min-width: 60px;
    height: 42px;
    background: var(--panel-2);
    border: 1px solid var(--line-2);
    border-radius: 4px;
  }
  .touchpad.on {
    border-color: var(--hi);
    box-shadow: inset 0 0 14px color-mix(in srgb, var(--hi) 40%, transparent);
  }
  .finger {
    position: absolute;
    width: 10px;
    height: 10px;
    margin: -5px 0 0 -5px;
    border-radius: 50%;
    background: #fff;
    box-shadow: 0 0 10px var(--hi);
  }
  .face {
    display: grid;
    grid-template-areas: ". t ." "s . c" ". x .";
    grid-template-columns: repeat(3, 14px);
    grid-template-rows: repeat(3, 14px);
    place-items: center;
    color: var(--dim);
    flex: none;
  }
  .face span {
    line-height: 0;
    transition: color 0.05s;
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
    filter: drop-shadow(0 0 4px currentColor);
  }
  .tri.on {
    color: var(--tri);
  }
  .sq.on {
    color: var(--sqr);
  }
  .ci.on {
    color: var(--cir);
  }
  .cr.on {
    color: var(--crs);
  }
</style>
