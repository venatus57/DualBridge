// Mirrors the serde types of the Rust side (dualbridge-core and the app).

export interface Rgb {
  r: number;
  g: number;
  b: number;
}

export type Effect =
  | { type: "off" }
  | { type: "slot_color" }
  | { type: "static"; color: Rgb }
  | { type: "breathing"; color: Rgb; period_ms: number; min_level: number }
  | { type: "rainbow"; period_ms: number; saturation: number }
  | { type: "color_cycle"; colors: Rgb[]; step_ms: number; smooth: boolean }
  | { type: "strobe"; color: Rgb; on_ms: number; off_ms: number }
  | { type: "battery_level"; empty: Rgb; full: Rgb };

export type EffectType = Effect["type"];

export type PlayerLedMode =
  | { type: "off" }
  | { type: "slot_number" }
  | { type: "battery" }
  | { type: "custom"; pattern: number };

export type Brightness = "high" | "medium" | "low";
export type MicLedMode = "off" | "on" | "pulse" | "show_mute";

export interface LightingConfig {
  effect: Effect;
  brightness: number;
  low_battery: { enabled: boolean; threshold: number; color: Rgb; period_ms: number };
  charging_indicator: boolean;
  offset_by_slot: boolean;
  player_leds: PlayerLedMode;
  player_led_brightness: Brightness;
  mic_led: MicLedMode;
}

export interface StickConfig {
  deadzone: number;
  outer: number;
  anti_deadzone: number;
  curve: number;
  invert_x: boolean;
  invert_y: boolean;
}

export interface TriggerConfig {
  deadzone: number;
  max: number;
  curve: number;
}

export type ButtonTarget =
  | "none"
  | "a"
  | "b"
  | "x"
  | "y"
  | "left_shoulder"
  | "right_shoulder"
  | "left_trigger"
  | "right_trigger"
  | "back"
  | "start"
  | "guide"
  | "left_thumb"
  | "right_thumb"
  | "dpad_up"
  | "dpad_down"
  | "dpad_left"
  | "dpad_right";

export interface MappingConfig {
  left_stick: StickConfig;
  right_stick: StickConfig;
  l2: TriggerConfig;
  r2: TriggerConfig;
  buttons: [string, ButtonTarget][];
  swap_sticks: boolean;
}

export type TriggerEffect =
  | { mode: "off" }
  | { mode: "feedback"; position: number; strength: number }
  | { mode: "weapon"; start: number; end: number; strength: number }
  | { mode: "vibration"; position: number; amplitude: number; frequency: number }
  | { mode: "raw"; bytes: number[] };

export interface Profile {
  version: number;
  name: string;
  lighting: LightingConfig;
  mapping: MappingConfig;
  virtual_kind: "xbox360" | "none";
  rumble: boolean;
  rumble_strength: number;
  left_trigger: TriggerEffect;
  right_trigger: TriggerEffect;
  games: string[];
}

export interface Settings {
  version: number;
  language: string | null;
  first_run_done: boolean;
  minimize_to_tray: boolean;
  start_minimized: boolean;
  autostart_configured: boolean;
  exclusive_mode: boolean;
  hidden_devices: string[];
  auto_profile_switch: boolean;
  profiles: Profile[];
  assignments: Record<string, string>;
  controller_names: Record<string, string>;
  controller_lighting: Record<string, LightingConfig>;
}

export type Model = "dual_shock4" | "dual_sense" | "dual_sense_edge" | "switch_pro";
export type Transport = "usb" | "bluetooth";

export interface Battery {
  percent: number;
  charging: "discharging" | "charging" | "full" | "error";
  cable: boolean;
}

export interface Touch {
  active: boolean;
  id: number;
  x: number;
  y: number;
}

export interface InputView {
  buttons: string[];
  lx: number;
  ly: number;
  rx: number;
  ry: number;
  l2: number;
  r2: number;
  touch: [Touch, Touch];
  motion: { gyro: [number, number, number]; accel: [number, number, number] };
  full: boolean;
}

export interface LatencyStats {
  last_ns: number;
  avg_ns: number;
  max_ns: number;
  reports: number;
  errors: number;
}

export interface ControllerView {
  slot: number;
  identity: string;
  name: string;
  model: Model;
  model_name: string;
  transport: Transport;
  battery: Battery;
  lightbar: string;
  profile: string;
  input: InputView;
  latency: LatencyStats;
  has_virtual: boolean;
  virtual_error: string | null;
  mic_muted: boolean;
  custom_lighting: boolean;
}

export type BackendStatus = "ready" | "driver_missing" | "unsupported";
export type Platform = "windows" | "macos" | "linux";

export interface Overview {
  settings: Settings;
  controllers: ControllerView[];
  virtual_status: BackendStatus;
  hidhide_installed: boolean;
  hidhide_error: string | null;
  platform: Platform;
  version: string;
  demo: boolean;
  active_game_profile: string | null;
}

export interface PreferencesPatch {
  language?: string | null;
  first_run_done?: boolean;
  minimize_to_tray?: boolean;
  start_minimized?: boolean;
  exclusive_mode?: boolean;
  auto_profile_switch?: boolean;
}
