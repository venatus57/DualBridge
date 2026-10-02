<script lang="ts">
  import ControllerCard from "../lib/components/ControllerCard.svelte";
  import PadGraphic from "../lib/components/PadGraphic.svelte";
  import Icon from "../lib/components/Icon.svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import { api } from "../lib/api";
  import { t } from "../lib/i18n.svelte";
  import { app, toast } from "../lib/store.svelte";

  const wireless = $derived(app.controllers.some((c) => c.transport !== "usb"));

  async function powerOffAll() {
    const n = await api.powerOffAll();
    toast(t("controllers.powerOffAll.done", { n }));
  }
</script>

<section class="page">
  <PageHeader index={1} glyph="cross" color="var(--crs)" title={t("controllers.title")} subtitle={t("controllers.subtitle")}>
    {#if wireless}
      <button class="off-all" onclick={powerOffAll} title={t("controllers.powerOffAll.hint")}>
        <Icon name="power" size={15} />
        {t("controllers.powerOffAll")}
      </button>
    {/if}
  </PageHeader>

  {#if app.controllers.length === 0}
    <div class="card empty">
      <div class="scan">
        <span class="ring"></span>
        <span class="ring r2"></span>
        <span class="ring r3"></span>
        <div class="pad"><PadGraphic color="#5c8dff" dualsense={true} width={230} /></div>
      </div>
      <div class="stack steps">
        <h2>{t("controllers.empty.title")}</h2>
        <p>{t("controllers.empty.usb")}</p>
        <ol>
          <li><span class="n mono">01</span>{t("controllers.empty.ds5")}</li>
          <li><span class="n mono">02</span>{t("controllers.empty.ds4")}</li>
          <li><span class="n mono">03</span>{t("controllers.empty.switch")}</li>
        </ol>
        <p class="muted">{t("controllers.empty.then")}</p>
      </div>
    </div>
  {:else}
    <div class="grid-2">
      {#each app.controllers as c (c.identity)}
        <ControllerCard {c} />
      {/each}
    </div>
  {/if}
</section>

<style>
  .off-all {
    display: inline-flex;
    align-items: center;
    gap: 7px;
  }
  .off-all:hover {
    color: var(--bad);
    border-color: color-mix(in srgb, var(--bad) 50%, transparent);
  }
  .empty {
    display: flex;
    gap: 44px;
    align-items: center;
    padding: 40px;
    overflow: hidden;
  }
  .scan {
    position: relative;
    flex: none;
    width: 280px;
    height: 240px;
    display: grid;
    place-items: center;
  }
  .ring {
    position: absolute;
    width: 180px;
    height: 180px;
    border-radius: 50%;
    border: 1px solid var(--crs);
    opacity: 0;
    animation: ping 3.6s cubic-bezier(0.2, 0.6, 0.3, 1) infinite;
  }
  .r2 {
    animation-delay: 1.2s;
  }
  .r3 {
    animation-delay: 2.4s;
  }
  .pad {
    position: relative;
    animation: float 4s ease-in-out infinite;
  }
  .steps {
    max-width: 520px;
  }
  ol {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  li {
    display: flex;
    gap: 12px;
    align-items: baseline;
    padding: 10px 12px;
    background: var(--bg-2);
    border-left: 2px solid var(--crs);
  }
  .n {
    color: var(--crs);
    font-size: 11px;
  }
  @keyframes ping {
    0% {
      transform: scale(0.6);
      opacity: 0.7;
    }
    100% {
      transform: scale(1.6);
      opacity: 0;
    }
  }
  @keyframes float {
    50% {
      transform: translateY(-8px);
    }
  }
  @media (max-width: 900px) {
    .empty {
      flex-direction: column;
    }
  }
</style>
