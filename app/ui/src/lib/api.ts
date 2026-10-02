// Talks to the Rust side. When the UI runs in a plain browser (`npm run dev`
// without Tauri), a small simulated backend is used instead so the interface
// can be developed and tested without the app or a controller.

import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { hsv, lerp, scale, toHex } from "./color";
import { defaultSettings, SLOT_COLORS } from "./defaults";
import type {
  ControllerView,
  LightingConfig,
  Overview,
  PreferencesPatch,
  UpdateInfo,
  Profile,
  Settings,
} from "./types";

export interface Api {
  getOverview(): Promise<Overview>;
  onControllers(cb: (list: ControllerView[]) => void): Promise<() => void>;
  onConflicts(cb: (programs: string[]) => void): Promise<() => void>;
  saveProfile(profile: Profile, previousName: string | null): Promise<Settings>;
  deleteProfile(name: string): Promise<Settings>;
  assignProfile(identity: string, profile: string): Promise<Settings>;
  renameController(identity: string, name: string): Promise<Settings>;
  setControllerLighting(identity: string, lighting: LightingConfig | null): Promise<Settings>;
  setPreferences(patch: PreferencesPatch): Promise<Settings>;
  swapSlots(a: number, b: number): Promise<boolean>;
  hideControllersNow(): Promise<void>;
  identify(slot: number): Promise<boolean>;
  /** Rejects with a `PowerOffError` when the controller can't be switched off. */
  powerOff(slot: number): Promise<void>;
  powerOffAll(): Promise<number>;
  /** A newer release, or null when up to date. */
  checkUpdate(): Promise<UpdateInfo | null>;
  /** Downloads and starts the installer; the app then quits. */
  installUpdate(update: UpdateInfo): Promise<void>;
  onUpdateProgress(cb: (percent: number) => void): Promise<() => void>;
  resetLatency(slot: number): Promise<void>;
  previewLighting(lighting: LightingConfig, slot: number, batteryPercent: number, durationMs: number, fps: number): Promise<string[]>;
  installDriver(driver: "vigembus" | "hidhide"): Promise<void>;
  getAutostart(): Promise<boolean>;
  setAutostart(on: boolean): Promise<void>;
  openUrl(url: string): Promise<void>;
}

export const isTauri = typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

const tauriApi: Api = {
  getOverview: () => invoke("get_overview"),
  onControllers: (cb) => listen<ControllerView[]>("controllers", (e) => cb(e.payload)),
  onConflicts: (cb) => listen<string[]>("conflicts", (e) => cb(e.payload)),
  saveProfile: (profile, previousName) => invoke("save_profile", { profile, previousName }),
  deleteProfile: (name) => invoke("delete_profile", { name }),
  assignProfile: (identity, profile) => invoke("assign_profile", { identity, profile }),
  renameController: (identity, name) => invoke("rename_controller", { identity, name }),
  setControllerLighting: (identity, lighting) => invoke("set_controller_lighting", { identity, lighting }),
  setPreferences: (patch) => invoke("set_preferences", { patch }),
  swapSlots: (a, b) => invoke("swap_slots", { a, b }),
  hideControllersNow: () => invoke("hide_controllers_now"),
  identify: (slot) => invoke("identify_controller", { slot }),
  powerOff: (slot) => invoke("power_off_controller", { slot }),
  powerOffAll: () => invoke("power_off_all"),
  checkUpdate: () => invoke("check_update"),
  installUpdate: (update) => invoke("install_update", { update }),
  onUpdateProgress: (cb) => listen<number>("update-progress", (e) => cb(e.payload)),
  resetLatency: (slot) => invoke("reset_latency", { slot }),
  previewLighting: (lighting, slot, batteryPercent, durationMs, fps) =>
    invoke("preview_lighting", { lighting, slot, batteryPercent, durationMs, fps }),
  installDriver: (driver) => invoke("install_driver", { driver }),
  async getAutostart() {
    const { isEnabled } = await import("@tauri-apps/plugin-autostart");
    return isEnabled();
  },
  async setAutostart(on) {
    const { enable, disable } = await import("@tauri-apps/plugin-autostart");
    await (on ? enable() : disable());
  },
  async openUrl(url) {
    const { openUrl } = await import("@tauri-apps/plugin-opener");
    await openUrl(url);
  },
};

// ---------------------------------------------------------------------------
// Browser demo backend

/** Approximation of `LightingConfig::render`, for the browser demo only. */
function demoRender(l: LightingConfig, t: number, slot: number, battery: number): string {
  const phase = (period: number) => (t % Math.max(1, period)) / Math.max(1, period);
  const wave = (p: number) => 0.5 - 0.5 * Math.cos(p * Math.PI * 2);
  if (l.low_battery.enabled && battery <= l.low_battery.threshold) {
    return toHex(scale(scale(l.low_battery.color, 0.15 + 0.85 * wave(phase(l.low_battery.period_ms))), l.brightness));
  }
  const e = l.effect;
  let c;
  switch (e.type) {
    case "off":
      c = { r: 0, g: 0, b: 0 };
      break;
    case "slot_color":
      c = SLOT_COLORS[(Math.max(1, slot) - 1) % 8];
      break;
    case "static":
      c = e.color;
      break;
    case "breathing":
      c = scale(e.color, e.min_level + (1 - e.min_level) * wave(phase(e.period_ms)));
      break;
    case "rainbow":
      c = hsv(phase(e.period_ms) * 360, e.saturation, 1);
      break;
    case "color_cycle": {
      if (!e.colors.length) {
        c = { r: 0, g: 0, b: 0 };
        break;
      }
      const step = Math.max(1, e.step_ms);
      const i = Math.floor(t / step) % e.colors.length;
      c = e.smooth ? lerp(e.colors[i], e.colors[(i + 1) % e.colors.length], (t % step) / step) : e.colors[i];
      break;
    }
    case "strobe":
      c = t % Math.max(1, e.on_ms + e.off_ms) < e.on_ms ? e.color : { r: 0, g: 0, b: 0 };
      break;
    case "battery_level":
      c = lerp(e.empty, e.full, battery / 100);
      break;
  }
  return toHex(scale(c, l.brightness));
}

function createDemoApi(): Api {
  const settings: Settings = defaultSettings();
  const start = performance.now();
  let progressCb: ((p: number) => void) | null = null;
  const identities = ["DEMO-DUALSENSE", "DEMO-DS4", "DEMO-SWITCH"];
  const kinds = [
    { model: "dual_sense", name: "DualSense", transport: "usb", battery: 76, charging: "charging" },
    { model: "dual_shock4", name: "DualShock 4", transport: "bluetooth", battery: 14, charging: "discharging" },
    { model: "switch_pro", name: "Pro Controller", transport: "bluetooth", battery: 70, charging: "discharging" },
  ] as const;
  const order = [0, 1, 2];

  const controllers = (): ControllerView[] => {
    const t = performance.now() - start;
    const s = t / 1000;
    return order.map((i, idx) => {
      const identity = identities[i];
      const slot = idx + 1;
      const profileName = settings.assignments[identity] ?? settings.profiles[0].name;
      const profile = settings.profiles.find((p) => p.name === profileName) ?? settings.profiles[0];
      const k = kinds[i];
      const battery = k.battery;
      const ax = (v: number) => Math.round(128 + Math.max(-1, Math.min(1, v)) * 127);
      const names = ["cross", "circle", "square", "triangle", "l1", "r1", "dpad_up", "dpad_right"];
      const pressed = [names[Math.floor(s / 0.4 + i * 3) % names.length]];
      return {
        slot,
        identity,
        name: settings.controller_names[identity] ?? k.name,
        model: k.model,
        model_name: k.name,
        transport: k.transport,
        battery: { percent: battery, charging: k.charging, cable: k.transport === "usb" },
        lightbar: settings.battery_saver
          ? "#000000"
          : demoRender(settings.controller_lighting[identity] ?? profile.lighting, t, slot, battery),
        profile: profile.name,
        input: {
          buttons: pressed,
          lx: ax(Math.cos(s * 1.2 + i)),
          ly: ax(Math.sin(s * 1.2 + i)),
          rx: ax(Math.sin(s * 0.8)),
          ry: ax(Math.sin(s * 1.6) * 0.6),
          l2: Math.round(127.5 + 127.5 * Math.sin(s * Math.PI * 0.5)),
          r2: s % 3 < 0.6 ? 255 : 0,
          touch: [
            { active: s % 4 < 2.5, id: 1, x: 960 + 800 * Math.sin(s * 0.7), y: 470 + 300 * Math.cos(s * 1.3) },
            { active: false, id: 0, x: 0, y: 0 },
          ],
          motion: { gyro: [Math.sin(s) * 20, 0, 0], accel: [0, 0, 1] },
          full: true,
        },
        latency: { last_ns: 21000 + ((t * 7) % 9000), avg_ns: 24000, max_ns: 61000, reports: Math.floor(t / 4), errors: 0 },
        has_virtual: true,
        virtual_error: null,
        mic_muted: false,
        custom_lighting: identity in settings.controller_lighting,
      } satisfies ControllerView;
    });
  };

  // Callers get copies, like they would from the real backend.
  const clone = (): Settings => JSON.parse(JSON.stringify(settings));

  return {
    async getOverview() {
      return {
        settings: clone(),
        controllers: controllers(),
        virtual_status: "ready",
        hidhide_installed: false,
        hidhide_error: null,
        platform: "windows",
        version: "0.2.0",
        demo: true,
        active_game_profile: null,
        conflicts: [],
      };
    },
    async onConflicts() {
      return () => {};
    },
    async onControllers(cb) {
      const id = setInterval(() => cb(controllers()), 33);
      return () => clearInterval(id);
    },
    async saveProfile(profile, previousName) {
      const old = previousName ?? profile.name;
      const i = settings.profiles.findIndex((p) => p.name === old);
      if (settings.profiles.some((p) => p.name === profile.name && p.name !== old)) {
        throw new Error(`a profile named "${profile.name}" already exists`);
      }
      if (i >= 0) settings.profiles[i] = structuredClone(profile);
      else settings.profiles.push(structuredClone(profile));
      for (const k of Object.keys(settings.assignments)) {
        if (settings.assignments[k] === old) settings.assignments[k] = profile.name;
      }
      return clone();
    },
    async deleteProfile(name) {
      if (settings.profiles.length <= 1) throw new Error("the last profile cannot be deleted");
      settings.profiles = settings.profiles.filter((p) => p.name !== name);
      for (const k of Object.keys(settings.assignments)) {
        if (settings.assignments[k] === name) delete settings.assignments[k];
      }
      return clone();
    },
    async assignProfile(identity, profile) {
      settings.assignments[identity] = profile;
      return clone();
    },
    async setControllerLighting(identity, lighting) {
      if (lighting) settings.controller_lighting[identity] = JSON.parse(JSON.stringify(lighting));
      else delete settings.controller_lighting[identity];
      return clone();
    },
    async renameController(identity, name) {
      if (name.trim()) settings.controller_names[identity] = name.trim();
      else delete settings.controller_names[identity];
      return clone();
    },
    async setPreferences(patch) {
      Object.assign(settings, patch);
      return clone();
    },
    async swapSlots(a, b) {
      [order[a - 1], order[b - 1]] = [order[b - 1], order[a - 1]];
      return true;
    },
    async identify() {
      return true;
    },
    // Add "?update" to the URL to see the update banner in the demo.
    async checkUpdate() {
      if (!location.search.includes("update")) return null;
      return {
        version: "0.3.0",
        page: "https://github.com/venatus57/DualBridge/releases",
        asset: { name: "DualBridge_0.3.0_x64-setup.exe", url: "", size: 1, sha256: null },
      };
    },
    async installUpdate() {
      for (let p = 0; p <= 100; p += 5) {
        progressCb?.(p);
        await new Promise((r) => setTimeout(r, 60));
      }
      throw new Error("demo mode");
    },
    async onUpdateProgress(cb) {
      progressCb = cb;
      return () => (progressCb = null);
    },
    async powerOff(slot) {
      const i = order[slot - 1];
      if (i === undefined) throw { kind: "not_found" };
      if (kinds[i].transport === "usb") throw { kind: "usb" };
      order.splice(slot - 1, 1);
    },
    async powerOffAll() {
      const before = order.length;
      for (let k = order.length - 1; k >= 0; k--) if (kinds[order[k]].transport !== "usb") order.splice(k, 1);
      return before - order.length;
    },
    async hideControllersNow() {},
    async resetLatency() {},
    async previewLighting(lighting, slot, battery, durationMs, fps) {
      const frames = Math.max(1, Math.floor((durationMs * fps) / 1000));
      return Array.from({ length: frames }, (_, i) => demoRender(lighting, (i * 1000) / fps, slot, battery));
    },
    async installDriver() {},
    async getAutostart() {
      return false;
    },
    async setAutostart() {},
    async openUrl(url) {
      window.open(url, "_blank");
    },
  };
}

export const api: Api = isTauri ? tauriApi : createDemoApi();
