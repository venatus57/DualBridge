<script lang="ts">
  import { api } from "../lib/api";
  import ColorInput from "../lib/components/ColorInput.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import PadGraphic from "../lib/components/PadGraphic.svelte";
  import { toHex } from "../lib/color";
  import { SLOT_COLORS } from "../lib/defaults";
  import { t } from "../lib/i18n.svelte";
  import { app, change, refreshOverview, saveProfileSoon, flushProfile } from "../lib/store.svelte";
  import type { Rgb } from "../lib/types";

  let step = $state(0);
  const STEPS = 5;
  const o = $derived(app.overview);

  let slotColors = $state(true);
  let color: Rgb = $state({ r: 0, g: 64, b: 255 });
  const found = $derived(app.controllers[0]);
  const previewColor = $derived(slotColors ? toHex(SLOT_COLORS[0]) : toHex(color));

  async function finish() {
    const p = app.settings.profiles[0];
    p.lighting.effect = slotColors ? { type: "slot_color" } : { type: "static", color };
    saveProfileSoon(p);
    await flushProfile();
    await change(api.setPreferences({ first_run_done: true }));
    app.wizard = false;
  }

  async function skip() {
    await change(api.setPreferences({ first_run_done: true }));
    app.wizard = false;
  }
</script>

<div class="overlay">
  <div class="wizard card">
    <div class="lightbar" aria-hidden="true"></div>
    <div class="top">
      <div class="progress">
        {#each Array(STEPS) as _, i (i)}
          <span class:done={i <= step}></span>
        {/each}
      </div>
      <span class="counter mono">0{step + 1} / 0{STEPS}</span>
    </div>

    {#key step}<div class="body">
      {#if step === 0}
        <PadGraphic color="#3ef0c4" width={250} playerLeds={0b00100} />
        <h1>{t("welcome.step1.title")}</h1>
        <p class="muted">{t("welcome.step1.text")}</p>
      {:else if step === 1}
        <h1>{t("welcome.step2.title")}</h1>
        {#if o?.platform === "windows"}
          {#if o.virtual_status === "ready"}
            <p class="status ok"><Icon name="check" size={20} /> {t("welcome.step2.ok")}</p>
          {:else}
            <p class="status bad"><Icon name="alert" size={20} /> {t("welcome.step2.missing")}</p>
            <div class="row">
              <button class="primary" onclick={() => api.installDriver("vigembus")}>{t("welcome.step2.install")}</button>
              <button onclick={refreshOverview}>{t("welcome.step2.recheck")}</button>
            </div>
          {/if}
          {#if o.hidhide_installed}
            <p class="status ok"><Icon name="check" size={20} /> {t("welcome.step2.double.ok")}</p>
          {:else}
            <p class="muted">{t("welcome.step2.double.missing")}</p>
            <div class="row">
              <button class="primary" onclick={() => api.installDriver("hidhide")}>{t("welcome.step2.double.install")}</button>
              <button onclick={refreshOverview}>{t("welcome.step2.recheck")}</button>
            </div>
          {/if}
        {:else}
          <p class="muted">{t("welcome.step2.macos")}</p>
        {/if}
      {:else if step === 2}
        <h1>{t("welcome.step3.title")}</h1>
        <PadGraphic color={found ? found.lightbar : "#39414f"} dualsense={found ? found.model !== "dual_shock4" : true} switchPro={found?.model === "switch_pro"} width={220} />
        {#if found}
          <p class="status ok"><Icon name="check" size={20} /> {t("welcome.step3.found", { name: found.model_name })}</p>
        {:else}
          <p class="muted waiting">{t("welcome.step3.waiting")}</p>
          <ul class="muted small">
            <li>{t("controllers.empty.ds5")}</li>
            <li>{t("controllers.empty.ds4")}</li>
            <li>{t("controllers.empty.switch")}</li>
          </ul>
        {/if}
      {:else if step === 3}
        <h1>{t("welcome.step4.title")}</h1>
        <PadGraphic color={previewColor} width={220} playerLeds={0b00100} />
        <p class="muted">{t("welcome.step4.text")}</p>
        <div class="choices">
          <button class="choice bevel" class:active={slotColors} onclick={() => (slotColors = true)}>
            <span class="rainbow"></span>{t("welcome.step4.slot")}
          </button>
          <div class="choice bevel" class:active={!slotColors}>
            <ColorInput value={color} onchange={(c) => { color = c; slotColors = false; }} />
          </div>
        </div>
      {:else}
        <PadGraphic color={previewColor} width={220} playerLeds={0b00100} />
        <h1>{t("welcome.step5.title")}</h1>
        <p class="muted">{t("welcome.step5.text")}</p>
      {/if}
    </div>{/key}

    <div class="row footer">
      {#if step < STEPS - 1}
        <button class="ghost" onclick={skip}>{t("welcome.skip")}</button>
      {/if}
      <div class="spacer"></div>
      {#if step > 0}
        <button onclick={() => step--}>{t("welcome.back")}</button>
      {/if}
      {#if step < STEPS - 1}
        <button class="primary" onclick={() => step++}>{t("welcome.next")}</button>
      {:else}
        <button class="primary" onclick={finish}>{t("welcome.finish")}</button>
      {/if}
    </div>
  </div>
</div>

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background:
      radial-gradient(ellipse at 50% 0%, rgba(62, 240, 196, 0.16), transparent 55%),
      radial-gradient(circle at 1px 1px, rgba(255, 255, 255, 0.05) 1px, transparent 0) 0 0 / 22px 22px,
      rgba(5, 6, 8, 0.94);
    backdrop-filter: blur(6px);
  }
  .wizard {
    width: min(640px, 92vw);
    min-height: 500px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    padding: 26px 32px;
    box-shadow: var(--shadow);
    overflow: hidden;
    animation: rise 0.4s cubic-bezier(0.2, 0.7, 0.2, 1);
  }
  .lightbar {
    position: absolute;
    top: 0;
    left: 25%;
    right: 25%;
    height: 3px;
    background: var(--accent);
    box-shadow: 0 0 20px 2px var(--accent);
  }
  .top {
    display: flex;
    align-items: center;
    gap: 16px;
  }
  .progress {
    flex: 1;
    display: flex;
    gap: 6px;
  }
  .progress span {
    flex: 1;
    height: 4px;
    background: var(--panel-3);
    transition: background 0.3s, box-shadow 0.3s;
  }
  .progress span.done {
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
  }
  .counter {
    font-size: 11px;
    letter-spacing: 0.1em;
    color: var(--muted);
  }
  .body {
    flex: 1;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    text-align: center;
    gap: 16px;
    animation: rise 0.35s cubic-bezier(0.2, 0.7, 0.2, 1);
  }
  .body p {
    max-width: 460px;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    font-weight: 600;
  }
  .status.ok {
    color: var(--ok);
  }
  .status.bad {
    color: var(--bad);
  }
  .waiting {
    animation: blink 1.6s ease-in-out infinite;
  }
  ul {
    text-align: left;
    margin: 0;
  }
  .choices {
    display: flex;
    gap: 12px;
    flex-wrap: wrap;
    justify-content: center;
  }
  .choice {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 14px;
    border: 1px solid var(--line-2);
    border-radius: var(--cut-sm);
    background: var(--bg-2);
  }
  .choice.active {
    border-color: var(--accent);
    box-shadow: 0 0 18px -6px var(--accent);
  }
  .rainbow {
    width: 28px;
    height: 28px;
    background: conic-gradient(#0040ff, #ff1010, #00dc3c, #ff28a0, #0040ff);
  }
  @keyframes blink {
    50% {
      opacity: 0.4;
    }
  }
</style>
