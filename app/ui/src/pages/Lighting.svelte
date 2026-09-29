<script lang="ts">
  import { api } from "../lib/api";
  import ColorInput from "../lib/components/ColorInput.svelte";
  import PadGraphic from "../lib/components/PadGraphic.svelte";
  import Segmented from "../lib/components/Segmented.svelte";
  import Slider from "../lib/components/Slider.svelte";
  import Toggle from "../lib/components/Toggle.svelte";
  import ProfilePicker from "../lib/components/ProfilePicker.svelte";
  import { hsv } from "../lib/color";
  import { SLOT_COLORS } from "../lib/defaults";
  import { t, type Key } from "../lib/i18n.svelte";
  import { editedProfile, saveProfileSoon } from "../lib/store.svelte";
  import type { Brightness, Effect, EffectType, MicLedMode, Rgb } from "../lib/types";

  const profile = $derived(editedProfile());
  const lighting = $derived(profile.lighting);

  const EFFECTS: EffectType[] = [
    "slot_color",
    "static",
    "breathing",
    "rainbow",
    "color_cycle",
    "strobe",
    "battery_level",
    "off",
  ];

  function save() {
    saveProfileSoon(profile);
  }

  function currentColor(): Rgb {
    const e = lighting.effect;
    if ("color" in e) return e.color;
    return SLOT_COLORS[0];
  }

  function pick(type: EffectType) {
    const color = currentColor();
    const effects: Record<EffectType, Effect> = {
      off: { type: "off" },
      slot_color: { type: "slot_color" },
      static: { type: "static", color },
      breathing: { type: "breathing", color, period_ms: 3000, min_level: 0.05 },
      rainbow: { type: "rainbow", period_ms: 6000, saturation: 1 },
      color_cycle: {
        type: "color_cycle",
        colors: [color, { r: 255, g: 40, b: 160 }, { r: 0, g: 220, b: 220 }],
        step_ms: 1500,
        smooth: true,
      },
      strobe: { type: "strobe", color, on_ms: 80, off_ms: 420 },
      battery_level: { type: "battery_level", empty: { r: 255, g: 0, b: 0 }, full: { r: 0, g: 220, b: 60 } },
    };
    lighting.effect = effects[type];
    save();
  }

  // Speed 1 (slow) to 10 (fast), mapped to a cycle duration.
  const toSpeed = (period: number, base: number) => Math.min(10, Math.max(1, Math.round(base / period)));
  const fromSpeed = (speed: number, base: number) => Math.round(base / speed);

  // Tile swatches.
  function tileStyle(type: EffectType): string {
    switch (type) {
      case "slot_color":
        return `background: conic-gradient(${SLOT_COLORS.slice(0, 4).map((c) => `rgb(${c.r},${c.g},${c.b})`).join(",")}, rgb(0,64,255))`;
      case "rainbow":
        return `background: linear-gradient(90deg, ${[0, 60, 120, 180, 240, 300, 360].map((h) => { const c = hsv(h, 1, 1); return `rgb(${c.r},${c.g},${c.b})`; }).join(",")})`;
      case "color_cycle":
        return "background: linear-gradient(90deg, #4d7cff, #ff28a0, #00dcdc)";
      case "battery_level":
        return "background: linear-gradient(90deg, #ff1010, #ffd200, #00dc3c)";
      case "off":
        return "background: #111";
      case "strobe":
        return "background: repeating-linear-gradient(90deg, #fff 0 6px, #111 6px 14px)";
      case "breathing":
        return "background: radial-gradient(circle, #4d7cff, #0b1020)";
      default:
        return "background: #4d7cff";
    }
  }

  // Live preview: ask the backend to render a few seconds of the effect and
  // loop over the frames.
  const FPS = 30;
  let frames: string[] = $state(["#000000"]);
  let frame = $state(0);
  let previewBattery = $state(60);

  $effect(() => {
    const snapshot = $state.snapshot(lighting);
    const battery = previewBattery;
    const timer = setTimeout(async () => {
      frames = await api.previewLighting(snapshot, 1, battery, 12000, FPS);
    }, 60);
    return () => clearTimeout(timer);
  });

  $effect(() => {
    let raf = 0;
    const start = performance.now();
    const loop = (now: number) => {
      frame = Math.floor(((now - start) / 1000) * FPS) % Math.max(1, frames.length);
      raf = requestAnimationFrame(loop);
    };
    raf = requestAnimationFrame(loop);
    return () => cancelAnimationFrame(raf);
  });

  const previewColor = $derived(frames[frame] ?? "#000000");
  const previewLeds = $derived.by(() => {
    const m = lighting.player_leds;
    if (m.type === "slot_number") return 0b00100;
    if (m.type === "battery") return (1 << Math.ceil(previewBattery / 20)) - 1;
    if (m.type === "custom") return m.pattern;
    return 0;
  });

  const brightnessOptions = $derived(
    (["high", "medium", "low"] as Brightness[]).map((v) => ({ value: v, label: t(`brightness.${v}` as Key) })),
  );
  const micOptions = $derived(
    (["show_mute", "on", "pulse", "off"] as MicLedMode[]).map((v) => ({ value: v, label: t(`mic.${v}` as Key) })),
  );
  const ledOptions = $derived(
    (["slot_number", "battery", "custom", "off"] as const).map((v) => ({ value: v, label: t(`playerLeds.${v}` as Key) })),
  );
</script>

<section class="page">
  <div class="page-header">
    <div>
      <h1>{t("lighting.title")}</h1>
      <p class="muted">{t("lighting.subtitle")}</p>
    </div>
    <div class="spacer"></div>
    <ProfilePicker />
  </div>

  <div class="layout">
    <div class="card preview">
      <span class="label muted small">{t("lighting.preview")}</span>
      <PadGraphic color={previewColor} dualsense={true} playerLeds={previewLeds} width={300} />
      <div class="swatch" style="background:{previewColor}; box-shadow: 0 0 40px {previewColor}"></div>
      {#if lighting.effect.type === "battery_level" || lighting.low_battery.enabled || lighting.player_leds.type === "battery"}
        <div class="battery-sim">
          <Slider label="🔋" bind:value={previewBattery} min={0} max={100} step={1} format={(v) => `${v} %`} />
        </div>
      {/if}
    </div>

    <div class="stack">
      <div class="card stack">
        <h2>{t("lighting.effect")}</h2>
        <div class="tiles">
          {#each EFFECTS as type (type)}
            <button class="tile" class:active={lighting.effect.type === type} onclick={() => pick(type)}>
              <span class="tile-swatch" style={tileStyle(type)}></span>
              <span class="tile-name">{t(`effect.${type}` as Key)}</span>
              <span class="tile-desc">{t(`effect.${type}.desc` as Key)}</span>
            </button>
          {/each}
        </div>

        {#if lighting.effect.type !== "off" && lighting.effect.type !== "slot_color"}
          <div class="divider"></div>
        {/if}
        {#if lighting.effect.type === "static"}
          {@const e = lighting.effect}
          <ColorInput label={t("lighting.color")} value={e.color} onchange={(c) => { e.color = c; save(); }} />
        {:else if lighting.effect.type === "breathing"}
          {@const e = lighting.effect}
          <ColorInput label={t("lighting.color")} value={e.color} onchange={(c) => { e.color = c; save(); }} />
          <div class="params">
            <Slider label={t("lighting.speed")} value={toSpeed(e.period_ms, 12000)} min={1} max={10} step={1} left={t("lighting.slow")} right={t("lighting.fast")} onchange={(v) => { e.period_ms = fromSpeed(v, 12000); save(); }} />
            <Slider label={t("lighting.minLevel")} bind:value={e.min_level} min={0} max={0.9} step={0.05} format={(v) => `${Math.round(v * 100)} %`} onchange={save} />
          </div>
        {:else if lighting.effect.type === "rainbow"}
          {@const e = lighting.effect}
          <div class="params">
            <Slider label={t("lighting.speed")} value={toSpeed(e.period_ms, 20000)} min={1} max={10} step={1} left={t("lighting.slow")} right={t("lighting.fast")} onchange={(v) => { e.period_ms = fromSpeed(v, 20000); save(); }} />
            <Slider label={t("lighting.saturation")} bind:value={e.saturation} min={0.2} max={1} step={0.05} format={(v) => `${Math.round(v * 100)} %`} onchange={save} />
          </div>
        {:else if lighting.effect.type === "color_cycle"}
          {@const e = lighting.effect}
          <span class="label">{t("lighting.colors")}</span>
          <div class="cycle">
            {#each e.colors as c, i (i)}
              <div class="cycle-item">
                <ColorInput value={c} presets={false} onchange={(nc) => { e.colors[i] = nc; save(); }} />
                {#if e.colors.length > 2}
                  <button class="ghost icon" aria-label="remove" onclick={() => { e.colors.splice(i, 1); save(); }}>✕</button>
                {/if}
              </div>
            {/each}
            {#if e.colors.length < 8}
              <button onclick={() => { e.colors.push({ r: 255, g: 255, b: 255 }); save(); }}>+ {t("lighting.addColor")}</button>
            {/if}
          </div>
          <div class="params">
            <Slider label={t("lighting.speed")} value={toSpeed(e.step_ms, 8000)} min={1} max={10} step={1} left={t("lighting.slow")} right={t("lighting.fast")} onchange={(v) => { e.step_ms = fromSpeed(v, 8000); save(); }} />
            <Toggle label={t("lighting.smooth")} bind:checked={e.smooth} onchange={save} />
          </div>
        {:else if lighting.effect.type === "strobe"}
          {@const e = lighting.effect}
          <ColorInput label={t("lighting.color")} value={e.color} onchange={(c) => { e.color = c; save(); }} />
          <div class="params">
            <Slider label={t("lighting.onTime")} bind:value={e.on_ms} min={20} max={1000} step={10} format={(v) => `${v} ms`} onchange={save} />
            <Slider label={t("lighting.offTime")} bind:value={e.off_ms} min={20} max={2000} step={10} format={(v) => `${v} ms`} onchange={save} />
          </div>
        {:else if lighting.effect.type === "battery_level"}
          {@const e = lighting.effect}
          <div class="params">
            <ColorInput label={t("lighting.empty")} value={e.empty} presets={false} onchange={(c) => { e.empty = c; save(); }} />
            <ColorInput label={t("lighting.full")} value={e.full} presets={false} onchange={(c) => { e.full = c; save(); }} />
          </div>
        {/if}
      </div>

      <div class="card stack">
        <h2>{t("lighting.general")}</h2>
        <Slider label={t("lighting.brightness")} bind:value={lighting.brightness} min={0} max={1} step={0.01} format={(v) => `${Math.round(v * 100)} %`} onchange={save} />
        <Toggle label={t("lighting.lowBattery")} bind:checked={lighting.low_battery.enabled} onchange={save} />
        {#if lighting.low_battery.enabled}
          <div class="params indent">
            <Slider label={t("lighting.lowBattery.threshold")} bind:value={lighting.low_battery.threshold} min={5} max={50} step={5} format={(v) => `${v} %`} onchange={save} />
            <ColorInput label={t("lighting.lowBattery.color")} value={lighting.low_battery.color} presets={false} onchange={(c) => { lighting.low_battery.color = c; save(); }} />
          </div>
        {/if}
        <Toggle label={t("lighting.charging")} bind:checked={lighting.charging_indicator} onchange={save} />
        <Toggle label={t("lighting.wave")} bind:checked={lighting.offset_by_slot} onchange={save} />
      </div>

      <div class="card stack">
        <h2>{t("lighting.dualsense")}</h2>
        <div class="field">
          <span class="label">{t("lighting.playerLeds")}</span>
          <Segmented
            options={ledOptions}
            value={lighting.player_leds.type}
            onchange={(v) => {
              lighting.player_leds = v === "custom" ? { type: "custom", pattern: 0b10101 } : { type: v };
              save();
            }}
          />
          {#if lighting.player_leds.type === "custom"}
            {@const m = lighting.player_leds}
            <div class="leds">
              {#each [0, 1, 2, 3, 4] as i (i)}
                <button
                  class="led"
                  class:on={m.pattern & (1 << i)}
                  aria-label="LED {i + 1}"
                  onclick={() => { m.pattern ^= 1 << i; save(); }}
                ></button>
              {/each}
            </div>
          {/if}
        </div>
        <div class="field">
          <span class="label">{t("lighting.playerLedBrightness")}</span>
          <Segmented options={brightnessOptions} value={lighting.player_led_brightness} onchange={(v) => { lighting.player_led_brightness = v; save(); }} />
        </div>
        <div class="field">
          <span class="label">{t("lighting.micLed")}</span>
          <Segmented options={micOptions} value={lighting.mic_led} onchange={(v) => { lighting.mic_led = v; save(); }} />
        </div>
      </div>
    </div>
  </div>
</section>

<style>
  .layout {
    display: grid;
    grid-template-columns: 340px 1fr;
    gap: 18px;
    align-items: start;
  }
  .preview {
    position: sticky;
    top: 0;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
  }
  .preview .label {
    align-self: flex-start;
  }
  .swatch {
    width: 100%;
    height: 14px;
    border-radius: 7px;
  }
  .battery-sim {
    width: 100%;
  }
  .tiles {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(150px, 1fr));
    gap: 10px;
  }
  .tile {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 4px;
    text-align: left;
    padding: 10px;
    background: var(--bg-2);
  }
  .tile.active {
    border-color: var(--accent);
    box-shadow: 0 0 0 1px var(--accent), 0 6px 20px -8px var(--accent);
  }
  .tile-swatch {
    width: 100%;
    height: 26px;
    border-radius: 7px;
    margin-bottom: 4px;
  }
  .tile-name {
    font-weight: 600;
  }
  .tile-desc {
    font-size: 12px;
    color: var(--muted);
  }
  .params {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 18px;
    align-items: start;
  }
  .indent {
    padding-left: 12px;
    border-left: 2px solid var(--line);
  }
  .cycle {
    display: flex;
    flex-wrap: wrap;
    gap: 10px;
    align-items: center;
  }
  .cycle-item {
    display: flex;
    align-items: center;
  }
  .leds {
    display: flex;
    gap: 10px;
    padding: 6px 0;
  }
  .led {
    width: 22px;
    height: 22px;
    border-radius: 50%;
    padding: 0;
    background: var(--bg-2);
  }
  .led.on {
    background: #fff;
    box-shadow: 0 0 10px #fff;
  }
  @media (max-width: 1000px) {
    .layout {
      grid-template-columns: 1fr;
    }
    .preview {
      position: static;
    }
  }
</style>
