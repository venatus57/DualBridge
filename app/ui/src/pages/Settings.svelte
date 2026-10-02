<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import Icon from "../lib/components/Icon.svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Toggle from "../lib/components/Toggle.svelte";
  import { LANGUAGES, t } from "../lib/i18n.svelte";
  import { app, change, checkForUpdate, refreshOverview } from "../lib/store.svelte";

  const s = $derived(app.settings);
  const o = $derived(app.overview);
  let autostart = $state(false);

  onMount(async () => {
    autostart = await api.getAutostart().catch(() => false);
    await refreshOverview();
  });

  async function setAutostart(on: boolean) {
    try {
      await api.setAutostart(on);
    } catch {
      autostart = !on;
    }
  }

  const REPO = "https://github.com/venatus57/DualBridge";
</script>

<section class="page">
  <PageHeader index={4} glyph="circle" color="var(--cir)" title={t("settings.title")} />

  <div class="card stack">
    <h2>{t("settings.general")}</h2>
    <div class="row">
      <span class="label">{t("settings.language")}</span>
      <div class="spacer"></div>
      <select value={s.language ?? ""} onchange={(e) => change(api.setPreferences({ language: e.currentTarget.value || null }))}>
        <option value="">{t("settings.language.auto")}</option>
        {#each LANGUAGES as l (l.code)}
          <option value={l.code}>{l.name}</option>
        {/each}
      </select>
    </div>
    <Toggle label={t("settings.autostart")} bind:checked={autostart} onchange={setAutostart} />
    <Toggle label={t("settings.tray")} checked={s.minimize_to_tray} onchange={(v) => change(api.setPreferences({ minimize_to_tray: v }))} />
    <Toggle label={t("settings.startMinimized")} checked={s.start_minimized} onchange={(v) => change(api.setPreferences({ start_minimized: v }))} />
    <Toggle label={t("settings.autoProfile")} checked={s.auto_profile_switch} onchange={(v) => change(api.setPreferences({ auto_profile_switch: v }))} />
  </div>

  <div class="card stack">
    <h2>{t("settings.battery")}</h2>
    <Toggle
      label={t("settings.batterySaver")}
      hint={t("settings.batterySaver.hint")}
      checked={s.battery_saver}
      onchange={(v) => change(api.setPreferences({ battery_saver: v }))}
    />
    <div class="row">
      <div class="stack-tight">
        <span>{t("settings.idleOff")}</span>
        <span class="hint">{t("settings.idleOff.hint")}</span>
      </div>
      <select value={String(s.idle_off_minutes)} onchange={(e) => change(api.setPreferences({ idle_off_minutes: Number(e.currentTarget.value) }))}>
        {#each [0, 5, 10, 15, 30, 60] as m (m)}
          <option value={String(m)}>{m === 0 ? t("settings.idleOff.never") : t("settings.idleOff.minutes", { n: m })}</option>
        {/each}
      </select>
    </div>
  </div>

  <div class="card stack">
    <h2>{t("settings.drivers")}</h2>
    {#if o?.platform === "windows"}
      <div class="driver">
        <div class="stack-tight">
          <span class="label">{t("settings.vigem")}</span>
          <span class="hint">{t("settings.vigem.hint")}</span>
        </div>
        {#if o.virtual_status === "ready"}
          <span class="badge ok"><Icon name="check" size={13} />{t("settings.installed")}</span>
        {:else}
          <span class="badge bad">{t("settings.missing")}</span>
          <button class="primary" onclick={() => api.installDriver("vigembus")}>{t("settings.install")}</button>
        {/if}
      </div>
      <div class="driver">
        <div class="stack-tight">
          <span class="label">{t("settings.hidhide")}</span>
          <span class="hint">{t("settings.hidhide.hint")}</span>
        </div>
        {#if o.hidhide_installed}
          <span class="badge ok"><Icon name="check" size={13} />{t("settings.installed")}</span>
        {:else}
          <span class="badge">{t("settings.missing")}</span>
          <button onclick={() => api.installDriver("hidhide")}>{t("settings.install")}</button>
        {/if}
      </div>
      <Toggle
        label={t("settings.exclusive")}
        hint={o.hidhide_installed ? t("settings.exclusive.hint") : t("settings.exclusive.needsHidHide")}
        checked={s.exclusive_mode && o.hidhide_installed}
        disabled={!o.hidhide_installed}
        onchange={(v) => change(api.setPreferences({ exclusive_mode: v }))}
      />
      {#if o.hidhide_installed && s.exclusive_mode}
        <div class="row">
          <button onclick={async () => { await api.hideControllersNow(); setTimeout(refreshOverview, 3000); }}>{t("settings.hideNow")}</button>
          <span class="hint">{t("settings.hideNow.hint")}</span>
        </div>
      {/if}
      {#if o.hidhide_error}
        <p class="error small"><Icon name="alert" size={14} /> {t("settings.hidhide.error", { msg: o.hidhide_error })}</p>
      {/if}
      <p class="hint">{t("settings.steamTip")}</p>
    {:else if o?.platform === "macos"}
      <p class="muted">{t("settings.macos")}</p>
    {:else}
      <p class="muted">{t("settings.linux")}</p>
    {/if}
  </div>

  <div class="card stack">
    <h2>{t("settings.about")}</h2>
    <p>DualBridge — {t("settings.version", { v: o?.version ?? "" })}</p>
    <p class="muted small">{t("settings.license")}</p>
    <Toggle label={t("settings.checkUpdates")} checked={s.check_updates} onchange={(v) => change(api.setPreferences({ check_updates: v }))} />
    <div class="row">
      <button onclick={() => api.openUrl(REPO)}><Icon name="link" size={14} /> {t("settings.github")}</button>
      <button onclick={() => checkForUpdate(true)}>{t("settings.checkNow")}</button>
      <button onclick={() => (app.wizard = true)}>{t("settings.runWizard")}</button>
    </div>
  </div>
</section>

<style>
  .driver {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 12px 14px;
    background: var(--bg-2);
    border: 1px solid var(--line);
    border-left: 2px solid var(--line-2);
  }
  .driver:has(:global(.badge.ok)) {
    border-left-color: var(--ok);
  }
  .driver:has(:global(.badge.bad)) {
    border-left-color: var(--bad);
  }
  .stack-tight {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .error {
    display: flex;
    align-items: center;
    gap: 6px;
    color: var(--bad);
  }
  .row button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
