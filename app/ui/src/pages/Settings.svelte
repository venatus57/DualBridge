<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../lib/api";
  import Icon from "../lib/components/Icon.svelte";
  import Toggle from "../lib/components/Toggle.svelte";
  import { LANGUAGES, t } from "../lib/i18n.svelte";
  import { app, change, refreshOverview } from "../lib/store.svelte";

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
  <div class="page-header">
    <h1>{t("settings.title")}</h1>
  </div>

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
        hint={t("settings.exclusive.hint")}
        checked={s.exclusive_mode}
        disabled={!o.hidhide_installed}
        onchange={(v) => change(api.setPreferences({ exclusive_mode: v }))}
      />
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
    <div class="row">
      <button onclick={() => api.openUrl(REPO)}><Icon name="link" size={14} /> {t("settings.github")}</button>
      <button onclick={() => (app.wizard = true)}>{t("settings.runWizard")}</button>
    </div>
  </div>
</section>

<style>
  .driver {
    display: flex;
    align-items: center;
    gap: 12px;
  }
  .stack-tight {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .row button {
    display: inline-flex;
    align-items: center;
    gap: 6px;
  }
</style>
