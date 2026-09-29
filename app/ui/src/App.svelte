<script lang="ts">
  import { onMount } from "svelte";
  import Icon from "./lib/components/Icon.svelte";
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

  const NAV: { page: Page; icon: string }[] = [
    { page: "controllers", icon: "gamepad" },
    { page: "lighting", icon: "light" },
    { page: "profiles", icon: "sliders" },
    { page: "settings", icon: "settings" },
  ];
</script>

<div class="shell">
  <nav>
    <div class="brand">
      <div class="logo"><Icon name="gamepad" size={20} /></div>
      <span>DualBridge</span>
    </div>
    {#each NAV as n (n.page)}
      <button class="nav-item" class:active={app.page === n.page} onclick={() => (app.page = n.page)}>
        <Icon name={n.icon} />
        <span>{t(`nav.${n.page}` as Key)}</span>
        {#if n.page === "controllers" && app.controllers.length}
          <span class="count">{app.controllers.length}</span>
        {/if}
      </button>
    {/each}
    <div class="spacer"></div>
    <div class="pads">
      {#each app.controllers as c (c.identity)}
        <span class="pad-dot" style="background:{c.lightbar}; box-shadow: 0 0 10px {c.lightbar}" title="{c.slot}. {c.name}"></span>
      {/each}
    </div>
    {#if app.overview?.demo}
      <div class="demo small">{t("demo.banner")}</div>
    {/if}
  </nav>

  <main>
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
</div>

<style>
  .shell {
    display: grid;
    grid-template-columns: 220px 1fr;
    height: 100%;
  }
  nav {
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 18px 12px;
    background: var(--bg-2);
    border-right: 1px solid var(--line);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 10px;
    font-weight: 700;
    font-size: 16px;
    padding: 0 8px 18px;
  }
  .logo {
    width: 34px;
    height: 34px;
    border-radius: 10px;
    display: grid;
    place-items: center;
    color: #fff;
    background: linear-gradient(135deg, var(--accent), var(--accent-2));
  }
  .nav-item {
    display: flex;
    align-items: center;
    gap: 12px;
    border: none;
    background: transparent;
    color: var(--muted);
    padding: 10px 12px;
    text-align: left;
  }
  .nav-item:hover {
    background: var(--panel);
    color: var(--text);
  }
  .nav-item.active {
    background: var(--panel-2);
    color: var(--text);
    box-shadow: inset 3px 0 0 var(--accent);
  }
  .count {
    margin-left: auto;
    font-size: 12px;
    background: var(--accent);
    color: #fff;
    border-radius: 999px;
    padding: 0 7px;
  }
  .spacer {
    flex: 1;
  }
  .pads {
    display: flex;
    gap: 8px;
    padding: 0 12px 10px;
    flex-wrap: wrap;
  }
  .pad-dot {
    width: 12px;
    height: 12px;
    border-radius: 50%;
  }
  .demo {
    margin: 0 4px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    background: rgba(251, 191, 36, 0.1);
    color: var(--warn);
    border: 1px solid rgba(251, 191, 36, 0.3);
  }
  main {
    overflow-y: auto;
  }
  .toast {
    position: fixed;
    bottom: 22px;
    left: 50%;
    transform: translateX(-50%);
    background: var(--panel-2);
    border: 1px solid var(--line);
    padding: 10px 18px;
    border-radius: 999px;
    box-shadow: var(--shadow);
    z-index: 100;
  }
  .toast.error {
    border-color: var(--bad);
    color: var(--bad);
  }
</style>
