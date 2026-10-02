<script lang="ts">
  import { api } from "../api";
  import { contrastText } from "../color";
  import { t } from "../i18n.svelte";
  import { app, change, toast } from "../store.svelte";
  import type { ControllerView, PowerOffError } from "../types";
  import Icon from "./Icon.svelte";
  import LiveInput from "./LiveInput.svelte";

  let { c }: { c: ControllerView } = $props();

  let renaming = $state(false);
  let newName = $state("");

  const touchpad = $derived<[number, number] | null>(
    c.model === "switch_pro" ? null : c.model === "dual_shock4" ? [1920, 942] : [1920, 1080],
  );
  const battery = $derived(c.battery);
  const batteryClass = $derived(
    battery.charging === "error" ? "bad" : battery.percent <= 15 && battery.charging === "discharging" ? "bad" : battery.percent <= 30 ? "warn" : "ok",
  );
  const cells = $derived(Math.max(battery.percent > 0 ? 1 : 0, Math.ceil(battery.percent / 20)));
  const ms = (ns: number) => (ns / 1e6).toFixed(ns < 1e6 ? 3 : 2);
  const gameProfile = $derived(app.overview?.active_game_profile ?? null);

  const wired = $derived(c.transport === "usb");

  async function powerOff() {
    try {
      await api.powerOff(c.slot);
    } catch (e) {
      const err = e as PowerOffError;
      if (err.kind === "usb") toast(t("controller.powerOff.usb"), true);
      else if (err.kind === "unsupported") toast(t("controller.powerOff.unsupported"), true);
      else if (err.kind === "failed") toast(t("controller.powerOff.failed", { msg: err.message }), true);
    }
  }

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
  <div class="lightbar" aria-hidden="true"></div>

  <header>
    <div class="slot bevel" style="background:{c.lightbar};color:{contrastText(c.lightbar)}" title={t("controller.slot", { n: c.slot })}>
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
          <span class="name">{c.name}</span>
          <button class="ghost icon rename" onclick={startRename} title={t("controller.rename")} aria-label={t("controller.rename")}>
            <Icon name="edit" size={14} />
          </button>
        </h2>
      {/if}
      <div class="meta">
        <span class="badge">{c.model_name}</span>
        <span class="badge">
          <Icon name={c.transport === "usb" ? "usb" : "bluetooth"} size={13} />
          {c.transport === "usb" ? "USB" : "Bluetooth"}
        </span>
        {#if c.custom_lighting}
          <span class="badge"><Icon name="light" size={13} />{t("controller.customLighting")}</span>
        {/if}
        {#if c.mic_muted}
          <span class="badge warn"><Icon name="mic" size={13} />{t("controller.micMuted")}</span>
        {/if}
      </div>
    </div>
    <div class="side">
      <div class="battery {batteryClass}" title="{battery.percent} %">
        {#if battery.charging === "charging" || battery.charging === "full"}<Icon name="bolt" size={13} />{/if}
        <span class="mono">{battery.percent}%</span>
        <div class="cells">
          {#each [0, 1, 2, 3, 4] as i (i)}
            <span class:on={i < cells}></span>
          {/each}
        </div>
      </div>
      <div class="arrows">
        <button class="icon ghost" disabled={c.slot <= 1} onclick={() => api.swapSlots(c.slot, c.slot - 1)} title={t("controller.moveLeft")} aria-label={t("controller.moveLeft")}>
          <Icon name="left" size={16} />
        </button>
        <button class="icon ghost" disabled={c.slot >= 8} onclick={() => api.swapSlots(c.slot, c.slot + 1)} title={t("controller.moveRight")} aria-label={t("controller.moveRight")}>
          <Icon name="right" size={16} />
        </button>
      </div>
    </div>
  </header>

  <LiveInput input={c.input} {touchpad} />

  <div class="controls">
    <label class="profile">
      <span class="tag mono">{t("controller.profile")}</span>
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
    <button
      onclick={() => {
        app.lightingTarget = c.identity;
        app.page = "lighting";
      }}
    >
      <Icon name="light" size={15} />
      {t("controller.editLighting")}
    </button>
    <button onclick={() => api.identify(c.slot)} title={t("controller.identify.hint")}>
      <Icon name="pulse" size={15} />
      {t("controller.identify")}
    </button>
    <button
      class="power"
      disabled={wired}
      onclick={powerOff}
      title={wired ? t("controller.powerOff.usb") : t("controller.powerOff.hint")}
    >
      <Icon name="power" size={15} />
      {t("controller.powerOff")}
    </button>
  </div>

  <footer>
    {#if c.has_virtual}
      <span class="status ok"><span class="led"></span>{t("controller.virtual.ok")}</span>
    {:else if c.virtual_error}
      <span class="status bad" title={c.virtual_error}><span class="led"></span>{t("controller.virtual.error")}</span>
    {:else}
      <span class="status off"><span class="led"></span>{t("controller.virtual.off")}</span>
    {/if}
    <div class="spacer"></div>
    <span class="latency" title={t("controller.latency.hint")}>
      <span class="tag mono">{t("controller.latency")}</span>
      <b class="num">{ms(c.latency.avg_ns)}</b><span class="num muted"> / {ms(c.latency.max_ns)} ms</span>
    </span>
  </footer>
</article>

<style>
  .controller {
    overflow: hidden;
    display: flex;
    flex-direction: column;
    gap: 16px;
    padding: 22px 22px 16px;
    border-color: color-mix(in srgb, var(--light) 22%, var(--line));
    background:
      radial-gradient(120% 60% at 50% -10%, color-mix(in srgb, var(--light) 16%, transparent), transparent 70%),
      var(--panel);
  }
  .lightbar {
    position: absolute;
    top: 0;
    left: 18%;
    right: 18%;
    height: 3px;
    background: var(--light);
    box-shadow:
      0 0 14px 1px var(--light),
      0 0 40px 4px color-mix(in srgb, var(--light) 50%, transparent);
    border-radius: 0 0 3px 3px;
  }
  header {
    position: relative;
    display: flex;
    align-items: flex-start;
    gap: 14px;
  }
  .slot {
    width: 42px;
    height: 42px;
    border-radius: 8px;
    display: grid;
    place-items: center;
    font-family: var(--font-display);
    font-weight: 700;
    font-size: 20px;
    box-shadow: 0 0 24px -2px var(--light);
    flex: none;
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 7px;
  }
  .title h2 {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 18px;
    letter-spacing: 0.04em;
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .rename {
    opacity: 0;
    transition: opacity 0.15s;
  }
  .controller:hover .rename {
    opacity: 0.7;
  }
  .meta {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .side {
    flex: none;
    display: flex;
    flex-direction: column;
    align-items: flex-end;
    gap: 6px;
  }
  .arrows {
    display: flex;
    margin-right: -6px;
  }
  .battery {
    display: flex;
    align-items: center;
    gap: 7px;
    font-size: 12px;
    white-space: nowrap;
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
  .cells {
    display: flex;
    gap: 2px;
    padding: 2px;
    border: 1px solid color-mix(in srgb, currentColor 45%, transparent);
  }
  .cells span {
    width: 5px;
    height: 11px;
    background: color-mix(in srgb, currentColor 14%, transparent);
  }
  .cells span.on {
    background: currentColor;
    box-shadow: 0 0 5px currentColor;
  }
  .controls {
    position: relative;
    display: flex;
    align-items: center;
    gap: 8px;
    flex-wrap: wrap;
  }
  .profile {
    display: flex;
    align-items: center;
    gap: 9px;
  }
  .tag {
    font-size: 10px;
    letter-spacing: 0.14em;
    text-transform: uppercase;
    color: var(--dim);
  }
  .controls button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
  .controls .power:not(:disabled):hover {
    color: var(--bad);
    border-color: color-mix(in srgb, var(--bad) 50%, transparent);
  }
  footer {
    display: flex;
    align-items: center;
    gap: 10px;
    font-size: 12.5px;
    margin: 0 -22px;
    padding: 12px 22px 0;
    border-top: 1px dashed var(--line-2);
  }
  .status {
    display: inline-flex;
    align-items: center;
    gap: 8px;
  }
  .led {
    width: 7px;
    height: 7px;
    background: currentColor;
    box-shadow: 0 0 8px currentColor;
  }
  .status.ok {
    color: var(--ok);
  }
  .status.ok .led {
    animation: beat 2.4s ease-in-out infinite;
  }
  .status.bad {
    color: var(--bad);
  }
  .status.off {
    color: var(--muted);
  }
  .status.off .led {
    box-shadow: none;
    background: var(--dim);
  }
  .latency {
    display: inline-flex;
    align-items: baseline;
    gap: 6px;
  }
  .num {
    color: var(--text);
    font-weight: 500;
  }
  .num.muted {
    color: var(--muted);
    font-weight: 400;
  }
  @keyframes beat {
    50% {
      opacity: 0.35;
    }
  }
</style>
