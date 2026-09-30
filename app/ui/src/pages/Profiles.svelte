<script lang="ts">
  import { api } from "../lib/api";
  import Icon from "../lib/components/Icon.svelte";
  import PageHeader from "../lib/components/PageHeader.svelte";
  import Segmented from "../lib/components/Segmented.svelte";
  import Slider from "../lib/components/Slider.svelte";
  import StickTest from "../lib/components/StickTest.svelte";
  import Toggle from "../lib/components/Toggle.svelte";
  import TriggerEffectEditor from "../lib/components/TriggerEffectEditor.svelte";
  import { DEFAULT_TARGETS, defaultMapping, defaultProfile } from "../lib/defaults";
  import { t, type Key } from "../lib/i18n.svelte";
  import { app, change, editedProfile, flushProfile, saveProfileSoon, toast } from "../lib/store.svelte";
  import type { ButtonTarget, Profile, StickConfig } from "../lib/types";

  type Tab = "general" | "buttons" | "sticks" | "triggers" | "vibration";
  let tab: Tab = $state("general");

  const profile = $derived(editedProfile());
  const isDefault = $derived(app.settings.profiles[0]?.name === profile.name);

  let nameDraft = $state("");
  let gameDraft = $state("");
  $effect(() => {
    nameDraft = profile.name;
  });

  function save() {
    saveProfileSoon(profile);
  }

  async function select(name: string) {
    await flushProfile();
    app.editing = name;
  }

  function uniqueName(base: string): string {
    const names = new Set(app.settings.profiles.map((p) => p.name));
    if (!names.has(base)) return base;
    for (let i = 2; ; i++) if (!names.has(`${base} ${i}`)) return `${base} ${i}`;
  }

  async function create(from?: Profile) {
    await flushProfile();
    const name = uniqueName(from ? from.name : t("profiles.new"));
    const p: Profile = from ? { ...($state.snapshot(from) as Profile), name, games: [] } : defaultProfile(name);
    if (await change(api.saveProfile(p, null))) app.editing = name;
  }

  async function remove() {
    if (isDefault) return;
    if (!confirm(t("profiles.deleteConfirm", { name: profile.name }))) return;
    await flushProfile();
    await change(api.deleteProfile(profile.name));
  }

  function rename() {
    const name = nameDraft.trim();
    if (!name || name === profile.name) {
      nameDraft = profile.name;
      return;
    }
    if (app.settings.profiles.some((p) => p.name === name)) {
      toast(t("common.error", { msg: `"${name}"` }), true);
      nameDraft = profile.name;
      return;
    }
    const old = profile.name;
    profile.name = name;
    app.editing = name;
    saveProfileSoon(profile, old, 0);
  }

  function addGame() {
    const g = gameDraft.trim();
    if (!g) return;
    const exe = /\.[a-z0-9]+$/i.test(g) ? g : `${g}.exe`;
    if (!profile.games.some((x) => x.toLowerCase() === exe.toLowerCase())) profile.games.push(exe);
    gameDraft = "";
    save();
  }

  // Buttons ----------------------------------------------------------------
  const BUTTONS = [
    "cross", "circle", "square", "triangle", "l1", "r1", "l3", "r3",
    "share", "options", "ps", "touchpad", "dpad_up", "dpad_down", "dpad_left", "dpad_right",
    "mute", "left_paddle", "right_paddle",
  ];
  const TARGETS: ButtonTarget[] = [
    "a", "b", "x", "y", "left_shoulder", "right_shoulder", "left_trigger", "right_trigger",
    "back", "start", "guide", "left_thumb", "right_thumb",
    "dpad_up", "dpad_down", "dpad_left", "dpad_right", "none",
  ];

  function targetOf(button: string): ButtonTarget {
    const o = [...profile.mapping.buttons].reverse().find(([b]) => b === button);
    return o ? o[1] : ((DEFAULT_TARGETS[button] ?? "none") as ButtonTarget);
  }

  function setTarget(button: string, target: ButtonTarget) {
    profile.mapping.buttons = profile.mapping.buttons.filter(([b]) => b !== button);
    if ((DEFAULT_TARGETS[button] ?? "none") !== target) profile.mapping.buttons.push([button, target]);
    save();
  }

  function resetButtons() {
    profile.mapping.buttons = [];
    save();
  }

  const pct = (v: number) => `${Math.round(v * 100)} %`;
  const tabs = $derived(
    (["general", "buttons", "sticks", "triggers", "vibration"] as Tab[]).map((v) => ({
      value: v,
      label: t(`profiles.section.${v}` as Key),
    })),
  );
  const firstController = $derived(app.controllers[0]);
</script>

{#snippet stickEditor(title: string, s: StickConfig, which: "left" | "right")}
  <div class="card stack">
    <div class="row">
      <h2>{title}</h2>
      <div class="spacer"></div>
      <button class="ghost small" onclick={() => { Object.assign(s, defaultMapping().left_stick); save(); }}>↺</button>
    </div>
    <div class="stick-grid">
      <StickTest config={s} x={which === "left" ? firstController?.input.lx : firstController?.input.rx} y={which === "left" ? firstController?.input.ly : firstController?.input.ry} />
      <div class="stack">
        <Slider label={t("profiles.deadzone")} hint={t("profiles.deadzone.hint")} bind:value={s.deadzone} min={0} max={0.5} step={0.01} format={pct} onchange={save} />
        <Slider label={t("profiles.antiDeadzone")} hint={t("profiles.antiDeadzone.hint")} bind:value={s.anti_deadzone} min={0} max={0.5} step={0.01} format={pct} onchange={save} />
        <Slider label={t("profiles.outer")} bind:value={s.outer} min={0.5} max={1} step={0.01} format={pct} onchange={save} />
        <Slider label={t("profiles.curve")} hint={t("profiles.curve.hint")} bind:value={s.curve} min={0.3} max={3} step={0.05} format={(v) => v.toFixed(2)} onchange={save} />
        <div class="row">
          <Toggle label={t("profiles.invertX")} bind:checked={s.invert_x} onchange={save} />
          <Toggle label={t("profiles.invertY")} bind:checked={s.invert_y} onchange={save} />
        </div>
      </div>
    </div>
  </div>
{/snippet}

<section class="page">
  <PageHeader index={3} glyph="square" color="var(--sqr)" title={t("profiles.title")} subtitle={t("profiles.subtitle")} />

  <div class="layout">
    <aside class="card list">
      {#each app.settings.profiles as p, i (p.name)}
        <button class="item" class:active={p.name === profile.name} onclick={() => select(p.name)}>
          <span class="idx mono">{String(i + 1).padStart(2, "0")}</span>
          <span class="name">{p.name}</span>
          <span class="muted small">
            {#if i === 0}{t("profiles.default")}{:else if p.games.length}<Icon name="game" size={12} /> {p.games.length}{/if}
          </span>
        </button>
      {/each}
      <div class="divider"></div>
      <button onclick={() => create()}><Icon name="plus" size={14} /> {t("profiles.new")}</button>
      <button onclick={() => create(profile)}><Icon name="copy" size={14} /> {t("profiles.duplicate")}</button>
      <button class="danger" disabled={isDefault} onclick={remove}><Icon name="trash" size={14} /> {t("profiles.delete")}</button>
    </aside>

    <div class="stack editor">
      <Segmented options={tabs} value={tab} onchange={(v) => (tab = v)} />

      {#if tab === "general"}
        <div class="card stack">
          <div class="field">
            <label for="profile-name">{t("profiles.name")}</label>
            <input id="profile-name" type="text" bind:value={nameDraft} onblur={rename} onkeydown={(e) => e.key === "Enter" && rename()} />
            {#if isDefault}<span class="hint">{t("profiles.defaultHint")}</span>{/if}
          </div>
          <div class="field">
            <span class="label">{t("profiles.virtual")}</span>
            <Segmented
              options={[
                { value: "xbox360", label: t("profiles.virtual.xbox360") },
                { value: "none", label: t("profiles.virtual.none") },
              ]}
              value={profile.virtual_kind}
              onchange={(v) => { profile.virtual_kind = v; save(); }}
            />
            <span class="hint">{t("profiles.virtual.hint")}</span>
          </div>
        </div>
        <div class="card stack">
          <div class="field">
            <span class="label">{t("profiles.games")}</span>
            <span class="hint">{t("profiles.games.hint")}</span>
          </div>
          <div class="row">
            <input type="text" placeholder={t("profiles.games.placeholder")} bind:value={gameDraft} onkeydown={(e) => e.key === "Enter" && addGame()} />
            <button onclick={addGame}>{t("profiles.games.add")}</button>
          </div>
          {#if profile.games.length === 0}
            <p class="muted small">{t("profiles.games.none")}</p>
          {:else}
            <div class="chips">
              {#each profile.games as g, i (g)}
                <span class="chip bevel"><Icon name="game" size={13} />{g}<button class="ghost icon" aria-label="remove" onclick={() => { profile.games.splice(i, 1); save(); }}><Icon name="x" size={12} /></button></span>
              {/each}
            </div>
          {/if}
        </div>
      {:else if tab === "buttons"}
        <div class="card stack">
          <div class="row">
            <p class="hint">{t("profiles.buttons.hint")}</p>
            <div class="spacer"></div>
            <button onclick={resetButtons}>{t("profiles.buttons.reset")}</button>
          </div>
          <div class="buttons">
            {#each BUTTONS as b (b)}
              {@const target = targetOf(b)}
              <div class="button-row" class:pressed={firstController?.input.buttons.includes(b)}>
                <span>{t(`button.${b}` as Key)}</span>
                <select class:changed={target !== (DEFAULT_TARGETS[b] ?? "none")} value={target} onchange={(e) => setTarget(b, e.currentTarget.value as ButtonTarget)}>
                  {#each TARGETS as tg (tg)}
                    <option value={tg}>{t(`target.${tg}` as Key)}</option>
                  {/each}
                </select>
              </div>
            {/each}
          </div>
        </div>
      {:else if tab === "sticks"}
        {@render stickEditor(t("profiles.leftStick"), profile.mapping.left_stick, "left")}
        {@render stickEditor(t("profiles.rightStick"), profile.mapping.right_stick, "right")}
        <div class="card">
          <Toggle label={t("profiles.swapSticks")} bind:checked={profile.mapping.swap_sticks} onchange={save} />
        </div>
      {:else if tab === "triggers"}
        <div class="grid-triggers">
          {#each [["l2", t("profiles.l2")], ["r2", t("profiles.r2")]] as [key, title] (key)}
            {@const tc = key === "l2" ? profile.mapping.l2 : profile.mapping.r2}
            {@const live = key === "l2" ? firstController?.input.l2 : firstController?.input.r2}
            <div class="card stack">
              <h2>{title}</h2>
              {#if live !== undefined}
                <div class="meter"><div style="width:{(live / 255) * 100}%"></div>
                  <span class="mark" style="left:{(tc.deadzone / 255) * 100}%"></span>
                  <span class="mark" style="left:{(tc.max / 255) * 100}%"></span>
                </div>
              {/if}
              <Slider label={t("profiles.triggerDeadzone")} bind:value={tc.deadzone} min={0} max={200} step={1} format={(v) => pct(v / 255)} onchange={save} />
              <Slider label={t("profiles.triggerMax")} bind:value={tc.max} min={50} max={255} step={1} format={(v) => pct(v / 255)} onchange={save} />
            </div>
          {/each}
        </div>
        <div class="card stack">
          <div class="field">
            <h2>{t("profiles.adaptive")}</h2>
            <span class="hint">{t("profiles.adaptive.hint")}</span>
          </div>
          <div class="grid-triggers">
            <TriggerEffectEditor title="L2" effect={profile.left_trigger} onchange={(e) => { profile.left_trigger = e; save(); }} />
            <TriggerEffectEditor title="R2" effect={profile.right_trigger} onchange={(e) => { profile.right_trigger = e; save(); }} />
          </div>
        </div>
      {:else if tab === "vibration"}
        <div class="card stack">
          <Toggle label={t("profiles.rumble")} bind:checked={profile.rumble} onchange={save} />
          <Slider label={t("profiles.rumbleStrength")} bind:value={profile.rumble_strength} min={0} max={1} step={0.05} format={pct} onchange={save} />
        </div>
      {/if}
    </div>
  </div>
</section>

<style>
  .layout {
    display: grid;
    grid-template-columns: 230px 1fr;
    gap: 18px;
    align-items: start;
  }
  .list {
    display: flex;
    flex-direction: column;
    gap: 6px;
    padding: 10px;
    position: sticky;
    top: 0;
  }
  .list > button {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .item {
    display: flex;
    align-items: center;
    gap: 10px;
    background: transparent;
    border-color: transparent;
    text-align: left;
    position: relative;
  }
  .item.active {
    background: linear-gradient(90deg, color-mix(in srgb, var(--sqr) 14%, transparent), transparent);
    border-color: color-mix(in srgb, var(--sqr) 35%, transparent);
  }
  .item.active::before {
    content: "";
    position: absolute;
    left: -11px;
    top: 7px;
    bottom: 7px;
    width: 2px;
    background: var(--sqr);
    box-shadow: 0 0 10px var(--sqr);
  }
  .idx {
    font-size: 10px;
    color: var(--dim);
  }
  .item.active .idx {
    color: var(--sqr);
  }
  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .editor {
    min-width: 0;
  }
  .chips {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
  }
  .chip {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 3px 4px 3px 10px;
    border-radius: 4px;
    background: var(--panel-2);
    border: 1px solid var(--line);
  }
  .row input[type="text"] {
    flex: 1;
  }
  .buttons {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(300px, 1fr));
    gap: 8px 18px;
  }
  .button-row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 4px 8px;
    border-left: 2px solid transparent;
    transition: background 0.1s;
  }
  .button-row:hover {
    background: var(--bg-2);
  }
  .button-row.pressed {
    background: var(--accent-soft);
    border-left-color: var(--accent);
  }
  .button-row select {
    width: 170px;
  }
  select.changed {
    border-color: var(--sqr);
    color: var(--sqr);
  }
  .stick-grid {
    display: grid;
    grid-template-columns: 170px 1fr;
    gap: 22px;
    align-items: start;
  }
  .grid-triggers {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 18px;
  }
  .meter {
    position: relative;
    height: 10px;
    background: repeating-linear-gradient(90deg, var(--line) 0 1px, transparent 1px 10%), var(--bg-2);
    border: 1px solid var(--line-2);
  }
  .meter > div {
    height: 100%;
    background: var(--accent);
    box-shadow: 0 0 10px var(--accent);
  }
  .mark {
    position: absolute;
    top: -4px;
    bottom: -4px;
    width: 2px;
    background: var(--warn);
  }
</style>
