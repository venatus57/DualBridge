<script lang="ts">
  import ControllerCard from "../lib/components/ControllerCard.svelte";
  import PadGraphic from "../lib/components/PadGraphic.svelte";
  import { t } from "../lib/i18n.svelte";
  import { app } from "../lib/store.svelte";
</script>

<section class="page">
  <div class="page-header">
    <div>
      <h1>{t("controllers.title")}</h1>
      <p class="muted">{t("controllers.subtitle")}</p>
    </div>
  </div>

  {#if app.controllers.length === 0}
    <div class="card empty">
      <div class="pads">
        <PadGraphic color="#4d7cff" dualsense={true} width={220} />
      </div>
      <div class="stack">
        <h2>{t("controllers.empty.title")}</h2>
        <p>{t("controllers.empty.usb")}</p>
        <ul>
          <li>{t("controllers.empty.ds5")}</li>
          <li>{t("controllers.empty.ds4")}</li>
        </ul>
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
  .empty {
    display: flex;
    gap: 32px;
    align-items: center;
    padding: 32px;
  }
  .pads {
    animation: float 4s ease-in-out infinite;
    flex: none;
  }
  ul {
    margin: 0;
    padding-left: 18px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }
  @keyframes float {
    50% {
      transform: translateY(-8px);
    }
  }
</style>
