<script lang="ts">
  import { t, type Key } from "../i18n.svelte";
  import type { TriggerEffect } from "../types";
  import Segmented from "./Segmented.svelte";
  import Slider from "./Slider.svelte";

  let {
    title,
    effect,
    onchange,
  }: { title: string; effect: TriggerEffect; onchange: (e: TriggerEffect) => void } = $props();

  type Mode = "off" | "feedback" | "weapon" | "vibration";
  const modes = $derived(
    (["off", "feedback", "weapon", "vibration"] as Mode[]).map((m) => ({ value: m, label: t(`trigger.${m}` as Key) })),
  );

  function setMode(m: Mode) {
    const effects: Record<Mode, TriggerEffect> = {
      off: { mode: "off" },
      feedback: { mode: "feedback", position: 2, strength: 5 },
      weapon: { mode: "weapon", start: 3, end: 6, strength: 7 },
      vibration: { mode: "vibration", position: 0, amplitude: 5, frequency: 30 },
    };
    onchange(effects[m]);
  }

  // Positions 0-9 shown as percent of the trigger travel.
  const zone = (v: number) => `${v * 10} %`;
</script>

<div class="stack">
  <h3>{title}</h3>
  <Segmented options={modes} value={effect.mode === "raw" ? "off" : effect.mode} onchange={setMode} />
  {#if effect.mode === "feedback"}
    {@const e = effect}
    <Slider label={t("trigger.position")} value={e.position} min={0} max={9} step={1} format={zone} onchange={(v) => onchange({ ...e, position: v })} />
    <Slider label={t("trigger.strength")} value={e.strength} min={1} max={8} step={1} onchange={(v) => onchange({ ...e, strength: v })} />
  {:else if effect.mode === "weapon"}
    {@const e = effect}
    <Slider label={t("trigger.start")} value={e.start} min={2} max={7} step={1} format={zone} onchange={(v) => onchange({ ...e, start: v, end: Math.max(e.end, v + 1) })} />
    <Slider label={t("trigger.end")} value={e.end} min={3} max={8} step={1} format={zone} onchange={(v) => onchange({ ...e, end: Math.max(v, e.start + 1) })} />
    <Slider label={t("trigger.strength")} value={e.strength} min={1} max={8} step={1} onchange={(v) => onchange({ ...e, strength: v })} />
  {:else if effect.mode === "vibration"}
    {@const e = effect}
    <Slider label={t("trigger.position")} value={e.position} min={0} max={9} step={1} format={zone} onchange={(v) => onchange({ ...e, position: v })} />
    <Slider label={t("trigger.amplitude")} value={e.amplitude} min={1} max={8} step={1} onchange={(v) => onchange({ ...e, amplitude: v })} />
    <Slider label={t("trigger.frequency")} value={e.frequency} min={1} max={255} step={1} onchange={(v) => onchange({ ...e, frequency: v })} />
  {/if}
</div>
