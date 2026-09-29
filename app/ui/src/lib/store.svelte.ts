import { api } from "./api";
import { defaultSettings } from "./defaults";
import { setLanguage, t } from "./i18n.svelte";
import type { ControllerView, Overview, Profile, Settings } from "./types";

export type Page = "controllers" | "lighting" | "profiles" | "settings";

export const app = $state({
  ready: false,
  overview: null as Overview | null,
  settings: defaultSettings() as Settings,
  controllers: [] as ControllerView[],
  page: "controllers" as Page,
  /** Profile being edited on the Lighting and Profiles pages. */
  editing: "Default",
  wizard: false,
  toast: null as { text: string; error: boolean } | null,
});

let toastTimer: ReturnType<typeof setTimeout> | undefined;

export function toast(text: string, error = false) {
  app.toast = { text, error };
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (app.toast = null), error ? 5000 : 1500);
}

function useSettings(s: Settings) {
  app.settings = s;
  setLanguage(s.language);
  if (!s.profiles.some((p) => p.name === app.editing)) app.editing = s.profiles[0].name;
}

/** Runs a settings-changing call, updating the UI or showing the error. */
export async function change(call: Promise<Settings>, quiet = true): Promise<boolean> {
  try {
    useSettings(await call);
    if (!quiet) toast(t("common.saved"));
    return true;
  } catch (e) {
    toast(t("common.error", { msg: String(e) }), true);
    return false;
  }
}

export async function refreshOverview() {
  const o = await api.getOverview();
  app.overview = o;
  app.controllers = o.controllers;
  useSettings(o.settings);
}

export async function init() {
  await refreshOverview();
  app.editing = app.settings.profiles[0].name;
  app.wizard = !app.settings.first_run_done;
  await api.onControllers((list) => (app.controllers = list));
  app.ready = true;
}

export function editedProfile(): Profile {
  return app.settings.profiles.find((p) => p.name === app.editing) ?? app.settings.profiles[0];
}

// Profile edits are saved shortly after the last change, so dragging a
// slider doesn't send hundreds of saves.
let saveTimer: ReturnType<typeof setTimeout> | undefined;
let pending: { profile: Profile; previous: string } | null = null;

export function saveProfileSoon(profile: Profile, previousName = profile.name, delay = 150) {
  pending = { profile: $state.snapshot(profile) as Profile, previous: pending?.previous ?? previousName };
  clearTimeout(saveTimer);
  saveTimer = setTimeout(flushProfile, delay);
}

export async function flushProfile() {
  clearTimeout(saveTimer);
  const p = pending;
  pending = null;
  if (!p) return;
  await change(api.saveProfile(p.profile, p.previous));
  if (p.previous === app.editing) app.editing = p.profile.name;
}
