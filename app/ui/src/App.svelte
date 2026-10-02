<script lang="ts">
  import { onMount } from "svelte";
  import Glyph, { type Shape } from "./lib/components/Glyph.svelte";
  import { t, type Key } from "./lib/i18n.svelte";
  import { app, init, type Page } from "./lib/store.svelte";
  import Controllers from "./pages/Controllers.svelte";
  import Lighting from "./pages/Lighting.svelte";
  import Profiles from "./pages/Profiles.svelte";
  import Settings from "./pages/Settings.svelte";
  import Welcome from "./pages/Welcome.svelte";

  let error = $state("");
  onMount(() => {
    init().catch((e) => (error = String(e)));
  });

  // Each page gets a face button and its color.
  const NAV: { page: Page; glyph: Shape; color: string }[] = [
    { page: "controllers", glyph: "cross", color: "var(--crs)" },
    { page: "lighting", glyph: "triangle", color: "var(--tri)" },
    { page: "profiles", glyph: "square", color: "var(--sqr)" },
    { page: "settings", glyph: "circle", color: "var(--cir)" },
  ];

  const SLOTS = [1, 2, 3, 4, 5, 6, 7, 8];
  const bySlot = $derived(new Map(app.controllers.map((c) => [c.slot, c])));

  // The room is lit by the first controller's lightbar. A dark or switched-off
  // light falls back to the accent color.
  const ambient = $derived.by(() => {
    const hex = app.controllers[0]?.lightbar;
    if (!hex || !/^#[0-9a-f]{6}$/i.test(hex)) return "#3ef0c4";
    const n = parseInt(hex.slice(1), 16);
    const max = Math.max(n >> 16, (n >> 8) & 255, n & 255);
    return max < 40 ? "#3ef0c4" : hex;
  });
</script>

<div class="shell" style="--ambient:{ambient}">
  <nav>
    <div class="brand">
      <svg class="mark" viewBox="0 0 40 28" aria-hidden="true">
        <path d="M3 22 C 10 6, 30 6, 37 22" />
        <path d="M3 22h34" class="deck" />
      </svg>
      <div class="wordmark">
        <span>Dual<b>Bridge</b></span>
        <span class="ver mono">{app.overview?.version ? `v${app.overview.version}` : ""} · PS4 / PS5</span>
      </div>
    </div>

    <div class="nav-list">
      {#each NAV as n, i (n.page)}
        <button class="nav-item" class:active={app.page === n.page} style="--c:{n.color}" onclick={() => (app.page = n.page)}>
          <span class="glyph"><Glyph shape={n.glyph} size={15} /></span>
          <span class="label">{t(`nav.${n.page}` as Key)}</span>
          {#if n.page === "controllers" && app.controllers.length}
            <span class="count mono">{app.controllers.length}</span>
          {:else}
            <span class="idx mono">0{i + 1}</span>
          {/if}
        </button>
      {/each}
    </div>

    <div class="spacer"></div>

    <div class="ports">
      <span class="ports-title mono">{t("nav.ports")}</span>
      <div class="ports-grid">
        {#each SLOTS as s (s)}
          {@const c = bySlot.get(s)}
          <button
            class="port"
            class:on={!!c}
            style={c ? `--l:${c.lightbar}` : ""}
            title={c ? `${s}. ${c.name}` : String(s)}
            disabled={!c}
            onclick={() => (app.page = "controllers")}
          >
            <span class="mono">{s}</span>
          </button>
        {/each}
      </div>
    </div>

    {#if app.overview?.demo}
      <div class="demo small">{t("demo.banner")}</div>
    {/if}
  </nav>

  <main>
    <div class="ambient" aria-hidden="true"></div>
    {#if app.overview?.conflicts?.length}
      <div class="conflict" role="alert">
        <b>{t("conflict.title", { names: app.overview.conflicts.join(", ") })}</b>
        <span>{t("conflict.text")}</span>
      </div>
    {/if}
    {#if error}
      <div class="page"><div class="card">{t("common.error", { msg: error })}</div></div>
    {:else if app.ready}
      {#if app.page === "controllers"}
        <Controllers />
      {:else if app.page === "lighting"}
        <Lighting />
      {:else if app.page === "profiles"}
        <Profiles />
      {:else}
        <Settings />
      {/if}
    {/if}
  </main>

  {#if app.ready && app.wizard}
    <Welcome />
  {/if}

  {#if app.toast}
    <div class="toast" class:error={app.toast.error} role="status">{app.toast.text}</div>
  {/if}
  <div class="grain" aria-hidden="true"></div>
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 236px 1fr;
    height: 100%;
    position: relative;
  }

  /* Sidebar ------------------------------------------------------------- */
  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 22px 14px 16px;
    background: linear-gradient(180deg, #0b0d11, #08090c);
    border-right: 1px solid var(--line);
    position: relative;
    z-index: 1;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 11px;
    padding: 0 8px 26px;
  }
  .mark {
    width: 38px;
    flex: none;
    fill: none;
    stroke: var(--ambient);
    stroke-width: 3;
    stroke-linecap: round;
    filter: drop-shadow(0 0 6px var(--ambient));
    transition: stroke 0.6s, filter 0.6s;
  }
  .mark .deck {
    stroke: var(--text);
    filter: none;
  }
  .wordmark {
    display: flex;
    flex-direction: column;
    line-height: 1.1;
  }
  .wordmark > span:first-child {
    font-family: var(--font-display);
    font-size: 19px;
    font-weight: 700;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .wordmark b {
    font-weight: 700;
    color: var(--ambient);
    transition: color 0.6s;
  }
  .ver {
    font-size: 10px;
    color: var(--dim);
    letter-spacing: 0.08em;
    margin-top: 3px;
  }

  .nav-list {
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    border: 1px solid transparent;
    background: transparent;
    color: var(--muted);
    padding: 10px 12px;
    text-align: left;
    font-family: var(--font-display);
    font-size: 14px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.08em;
    position: relative;
  }
  .nav-item:hover {
    background: var(--panel);
    border-color: var(--line);
    color: var(--text);
  }
  .glyph {
    display: grid;
    place-items: center;
    width: 26px;
    height: 26px;
    color: var(--dim);
    transition: color 0.2s, filter 0.2s;
  }
  .nav-item:hover .glyph {
    color: var(--c);
  }
  .nav-item.active {
    color: var(--text);
    background: linear-gradient(90deg, color-mix(in srgb, var(--c) 16%, transparent), transparent 85%);
    border-color: color-mix(in srgb, var(--c) 30%, transparent);
  }
  .nav-item.active::before {
    content: "";
    position: absolute;
    left: -15px;
    top: 8px;
    bottom: 8px;
    width: 3px;
    background: var(--c);
    box-shadow: 0 0 12px var(--c);
  }
  .nav-item.active .glyph {
    color: var(--c);
    filter: drop-shadow(0 0 6px var(--c));
  }
  .label {
    flex: 1;
  }
  .idx {
    font-size: 10px;
    color: var(--dim);
    letter-spacing: 0.05em;
  }
  .count {
    font-size: 11px;
    font-weight: 500;
    color: var(--accent-ink);
    background: var(--crs);
    padding: 0 6px;
    min-width: 20px;
    text-align: center;
    clip-path: polygon(4px 0, 100% 0, 100% calc(100% - 4px), calc(100% - 4px) 100%, 0 100%, 0 4px);
  }

  .ports {
    padding: 12px 8px;
    border-top: 1px solid var(--line);
  }
  .ports-title {
    display: block;
    font-size: 10px;
    letter-spacing: 0.18em;
    text-transform: uppercase;
    color: var(--dim);
    margin-bottom: 9px;
  }
  .ports-grid {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 6px;
  }
  .port {
    height: 26px;
    padding: 0;
    display: grid;
    place-items: center;
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-radius: 4px;
    font-size: 11px;
    color: var(--dim);
  }
  .port:disabled {
    opacity: 1;
  }
  .port.on {
    color: #fff;
    border-color: var(--l);
    background: color-mix(in srgb, var(--l) 22%, var(--bg-2));
    box-shadow:
      0 0 14px -3px var(--l),
      inset 0 -2px 0 var(--l);
    text-shadow: 0 0 6px var(--l);
  }
  .demo {
    margin: 8px 4px 0;
    padding: 8px 10px;
    border-left: 2px solid var(--warn);
    background: rgba(255, 194, 61, 0.07);
    color: var(--warn);
  }

  /* Main area ----------------------------------------------------------- */
  main {
    overflow-y: auto;
    position: relative;
    isolation: isolate;
    background:
      radial-gradient(circle at 1px 1px, rgba(255, 255, 255, 0.045) 1px, transparent 0) 0 0 / 22px 22px,
      var(--bg);
  }
  .ambient {
    position: fixed;
    top: -30vh;
    right: -10vw;
    width: 70vw;
    height: 80vh;
    background: radial-gradient(closest-side, var(--ambient), transparent);
    opacity: 0.1;
    pointer-events: none;
    z-index: -1;
    transition: background 0.8s;
  }
  .conflict {
    position: sticky;
    top: 0;
    z-index: 5;
    display: flex;
    flex-direction: column;
    gap: 3px;
    margin: 0 0 -8px;
    padding: 12px 40px;
    background: color-mix(in srgb, var(--bad) 16%, var(--bg));
    border-bottom: 1px solid color-mix(in srgb, var(--bad) 50%, transparent);
    color: var(--text);
  }
  .conflict b {
    color: var(--bad);
  }
  .conflict span {
    color: var(--muted);
    font-size: 13px;
  }
  .grain {
    position: fixed;
    inset: 0;
    pointer-events: none;
    z-index: 200;
    opacity: 0.05;
    background-image: url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='160' height='160'><filter id='n'><feTurbulence type='fractalNoise' baseFrequency='0.9' numOctaves='2' stitchTiles='stitch'/></filter><rect width='100%' height='100%' filter='url(%23n)'/></svg>");
  }

  .toast {
    position: fixed;
    bottom: 24px;
    left: calc(50% + 118px);
    transform: translateX(-50%);
    background: var(--panel-2);
    border: 1px solid var(--line-2);
    border-left: 3px solid var(--accent);
    padding: 11px 18px;
    box-shadow: var(--shadow);
    z-index: 100;
    animation: toast 0.25s ease-out;
  }
  .toast.error {
    border-left-color: var(--bad);
    color: var(--bad);
  }
  @keyframes toast {
    from {
      opacity: 0;
      transform: translate(-50%, 8px);
    }
  }
</style>
