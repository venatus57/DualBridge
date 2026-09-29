<script lang="ts">
  import { api } from "../api";
  import { contrastText } from "../color";
  import { t } from "../i18n.svelte";
  import { app, change } from "../store.svelte";
  import type { ControllerView } from "../types";
  import Icon from "./Icon.svelte";
  import LiveInput from "./LiveInput.svelte";

  let { c }: { c: ControllerView } = $props();

  let renaming = $state(false);
  let newName = $state("");

  const dualsense = $derived(c.model !== "dual_shock4");
  const battery = $derived(c.battery);
  const batteryClass = $derived(
    battery.charging === "error" ? "bad" : battery.percent <= 15 && battery.charging === "discharging" ? "bad" : battery.percent <= 30 ? "warn" : "ok",
  );
  const ms = (ns: number) => (ns / 1e6).toFixed(ns < 1e6 ? 3 : 2);
  const gameProfile = $derived(app.overview?.active_game_profile ?? null);

  function startRename() {
    newName = c.name;
    renaming = true;
  }

  async function commitRename() {
    renaming = false;
    await change(api.renameController(c.identity, newName));
  }
</script>

<article class="card controller" style="--light:{c.lightbar}">
  <div class="glow"></div>
  <header class="row">
    <div class="slot" style="background:{c.lightbar};color:{contrastText(c.lightbar)}" title={t("controller.slot", { n: c.slot })}>
      {c.slot}
    </div>
    <div class="title">
      {#if renaming}
        <!-- svelte-ignore a11y_autofocus -->
        <input
          type="text"
          bind:value={newName}
          autofocus
          onblur={commitRename}
          onkeydown={(e) => e.key === "Enter" && commitRename()}
        />
      {:else}
        <h2>
          {c.name}
          <button class="ghost icon rename" onclick={startRename} title={t("controller.rename")} aria-label={t("controller.rename")}>
            <Icon name="edit" size={14} />
          </button>
        </h2>
      {/if}
      <div class="row meta">
        <span class="badge">{c.model_name}</span>
        <span class="badge">
          <Icon name={c.transport === "usb" ? "usb" : "bluetooth"} size={13} />
          {c.transport === "usb" ? "USB" : "Bluetooth"}
        </span>
        {#if c.mic_muted}
          <span class="badge warn"><Icon name="mic" size={13} />{t("controller.micMuted")}</span>
        {/if}
      </div>
    </div>
    <div class="spacer"></div>
    <div class="battery {batteryClass}" title="{battery.percent} %">
      <div class="cell"><div class="level" style="width:{battery.percent}%"></div></div>
      <span>{battery.percent} %</span>
      {#if battery.charging === "charging" || battery.charging === "full"}<Icon name="bolt" size={14} />{/if}
    </div>
  </header>

  <LiveInput input={c.input} touchpad={dualsense ? [1920, 1080] : [1920, 942]} />

  <div class="row controls">
    <label class="profile">
      <span class="muted small">{t("controller.profile")}</span>
      <select
        value={app.settings.assignments[c.identity] ?? app.settings.profiles[0].name}
        onchange={(e) => change(api.assignProfile(c.identity, e.currentTarget.value))}
      >
        {#each app.settings.profiles as p (p.name)}
          <option value={p.name}>{p.name}</option>
        {/each}
      </select>
    </label>
    {#if gameProfile}
      <span class="badge ok" title={t("controller.gameProfile")}><Icon name="game" size={13} />{gameProfile}</span>
    {/if}
    <div class="spacer"></div>
    <button class="icon ghost" disabled={c.slot <= 1} onclick={() => api.swapSlots(c.slot, c.slot - 1)} title={t("controller.moveLeft")} aria-label={t("controller.moveLeft")}>
      <Icon name="left" />
    </button>
    <button class="icon ghost" disabled={c.slot >= 8} onclick={() => api.swapSlots(c.slot, c.slot + 1)} title={t("controller.moveRight")} aria-label={t("controller.moveRight")}>
      <Icon name="right" />
    </button>
    <button onclick={() => api.identify(c.slot)} title={t("controller.identify.hint")}>
      <Icon name="pulse" size={15} />
      {t("controller.identify")}
    </button>
  </div>

  <footer class="row small">
    {#if c.has_virtual}
      <span class="status ok"><Icon name="check" size={14} />{t("controller.virtual.ok")}</span>
    {:else if c.virtual_error}
      <span class="status bad" title={c.virtual_error}><Icon name="alert" size={14} />{t("controller.virtual.error")}</span>
    {:else}
      <span class="status muted">{t("controller.virtual.off")}</span>
    {/if}
    <div class="spacer"></div>
    <span class="muted" title={t("controller.latency.hint")}>
      {t("controller.latency")} <b class="num">{ms(c.latency.avg_ns)}</b> / {ms(c.latency.max_ns)} ms
    </span>
  </footer>
</article>

<style>
  .controller {
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 14px;
  }
  .glow {
    position: absolute;
    inset: -60px -40px auto -40px;
    height: 120px;
    background: radial-gradient(ellipse at 50% 0%, var(--light), transparent 70%);
    opacity: 0.35;
    pointer-events: none;
    transition: background 0.1s;
  }
  header {
    position: relative;
    align-items: flex-start;
  }
  .slot {
    width: 38px;
    height: 38px;
    border-radius: 11px;
    display: grid;
    place-items: center;
    font-weight: 800;
    font-size: 17px;
    box-shadow: 0 0 22px -2px var(--light);
    flex: none;
  }
  .title {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  .title h2 {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .rename {
    opacity: 0;
    transition: opacity 0.15s;
  }
  .controller:hover .rename {
    opacity: 0.7;
  }
  .meta {
    gap: 6px;
  }
  .battery {
    display: flex;
    align-items: center;
    gap: 7px;
    font-variant-numeric: tabular-nums;
  }
  .battery.ok {
    color: var(--ok);
  }
  .battery.warn {
    color: var(--warn);
  }
  .battery.bad {
    color: var(--bad);
  }
  .cell {
    width: 30px;
    height: 14px;
    border: 2px solid currentColor;
    border-radius: 4px;
    padding: 1px;
    position: relative;
  }
  .cell::after {
    content: "";
    position: absolute;
    right: -5px;
    top: 3px;
    width: 3px;
    height: 4px;
    background: currentColor;
    border-radius: 0 2px 2px 0;
  }
  .level {
    height: 100%;
    background: currentColor;
    border-radius: 1px;
  }
  .controls {
    flex-wrap: wrap;
  }
  .profile {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .controls button:not(.icon) {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  footer {
    border-top: 1px solid var(--line);
    padding-top: 12px;
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .status.ok {
    color: var(--ok);
  }
  .status.bad {
    color: var(--bad);
  }
  .num {
    font-variant-numeric: tabular-nums;
    color: var(--text);
  }
</style>
