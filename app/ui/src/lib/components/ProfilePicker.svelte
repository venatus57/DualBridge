<script lang="ts">
  import { t } from "../i18n.svelte";
  import { app, flushProfile } from "../store.svelte";
</script>

<label class="picker">
  <span class="muted">{t("lighting.profile")}</span>
  <select
    value={app.editing}
    onchange={async (e) => {
      const name = e.currentTarget.value;
      await flushProfile();
      app.editing = name;
    }}
  >
    {#each app.settings.profiles as p, i (p.name)}
      <option value={p.name}>{p.name}{i === 0 ? ` (${t("profiles.default")})` : ""}</option>
    {/each}
  </select>
</label>

<style>
  .picker {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  select {
    min-width: 200px;
  }
</style>
