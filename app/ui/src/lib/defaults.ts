// Default values matching `Default` implementations on the Rust side.

import type { LightingConfig, MappingConfig, Profile, Rgb, Settings } from "./types";

export const SLOT_COLORS: Rgb[] = [
  { r: 0, g: 64, b: 255 },
  { r: 255, g: 16, b: 16 },
  { r: 0, g: 220, b: 60 },
  { r: 255, g: 40, b: 160 },
  { r: 255, g: 120, b: 0 },
  { r: 0, g: 220, b: 220 },
  { r: 140, g: 40, b: 255 },
  { r: 255, g: 210, b: 0 },
];

export function defaultLighting(): LightingConfig {
  return {
    effect: { type: "slot_color" },
    brightness: 1,
    low_battery: { enabled: true, threshold: 15, color: { r: 255, g: 0, b: 0 }, period_ms: 1500 },
    charging_indicator: false,
    offset_by_slot: false,
    player_leds: { type: "slot_number" },
    player_led_brightness: "medium",
    mic_led: "show_mute",
  };
}

export function defaultMapping(): MappingConfig {
  const stick = { deadzone: 0, outer: 1, anti_deadzone: 0, curve: 1, invert_x: false, invert_y: false };
  const trigger = { deadzone: 0, max: 255, curve: 1 };
  return {
    left_stick: { ...stick },
    right_stick: { ...stick },
    l2: { ...trigger },
    r2: { ...trigger },
    buttons: [],
    swap_sticks: false,
  };
}

export function defaultProfile(name = "Default"): Profile {
  return {
    version: 1,
    name,
    lighting: defaultLighting(),
    mapping: defaultMapping(),
    virtual_kind: "xbox360",
    rumble: true,
    rumble_strength: 1,
    left_trigger: { mode: "off" },
    right_trigger: { mode: "off" },
    games: [],
  };
}

export function defaultSettings(): Settings {
  return {
    version: 1,
    language: null,
    first_run_done: false,
    minimize_to_tray: true,
    start_minimized: false,
    autostart_configured: false,
    exclusive_mode: true,
    hidden_devices: [],
    auto_profile_switch: true,
    check_updates: true,
    battery_saver: false,
    idle_off_minutes: 0,
    profiles: [defaultProfile()],
    assignments: {},
    controller_names: {},
    controller_lighting: {},
  };
}

/** Default Xbox target of each button, as in `mapping::default_target`. */
export const DEFAULT_TARGETS: Record<string, string> = {
  cross: "a",
  circle: "b",
  square: "x",
  triangle: "y",
  l1: "left_shoulder",
  r1: "right_shoulder",
  share: "back",
  options: "start",
  l3: "left_thumb",
  r3: "right_thumb",
  ps: "guide",
  touchpad: "back",
  dpad_up: "dpad_up",
  dpad_down: "dpad_down",
  dpad_left: "dpad_left",
  dpad_right: "dpad_right",
};
