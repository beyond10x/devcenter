import { readonly, ref } from "vue";

export const THEME_STORAGE_KEY = "b10x.devcenter.theme.v1";

export const themeOptions = [
  { value: "system", label: "System" },
  { value: "light", label: "Light" },
  { value: "dark", label: "Dark" },
  { value: "monokai", label: "Monokai" },
  { value: "solarized-light", label: "Solarized Light" },
  { value: "solarized-dark", label: "Solarized Dark" },
] as const;

export type ThemePreference = (typeof themeOptions)[number]["value"];
export type ResolvedTheme = Exclude<ThemePreference, "system">;

const allowedThemes = new Set<ThemePreference>(themeOptions.map((option) => option.value));
const preference = ref<ThemePreference>("system");
const resolved = ref<ResolvedTheme>("light");
let mediaQuery: MediaQueryList | undefined;
let listening = false;

export function parseThemePreference(value: string | null | undefined): ThemePreference {
  return value && allowedThemes.has(value as ThemePreference)
    ? (value as ThemePreference)
    : "system";
}

export function resolveTheme(preferred: ThemePreference, systemDark: boolean): ResolvedTheme {
  return preferred === "system" ? (systemDark ? "dark" : "light") : preferred;
}

export function themeColor(theme: ResolvedTheme): string {
  switch (theme) {
    case "dark":
      return "#101715";
    case "monokai":
      return "#272822";
    case "solarized-light":
      return "#fdf6e3";
    case "solarized-dark":
      return "#002b36";
    default:
      return "#f3f5f4";
  }
}

function applyTheme(nextPreference: ThemePreference) {
  if (typeof window === "undefined" || typeof document === "undefined") return;
  mediaQuery ??= window.matchMedia("(prefers-color-scheme: dark)");
  const nextResolved = resolveTheme(nextPreference, mediaQuery.matches);
  preference.value = nextPreference;
  resolved.value = nextResolved;
  document.documentElement.dataset.themePreference = nextPreference;
  document.documentElement.dataset.theme = nextResolved;
  document.documentElement.style.colorScheme = ["light", "solarized-light"].includes(nextResolved)
    ? "light"
    : "dark";
  document
    .querySelector<HTMLMetaElement>('meta[name="theme-color"]')
    ?.setAttribute("content", themeColor(nextResolved));
}

function onSystemThemeChange() {
  if (preference.value === "system") applyTheme("system");
}

export function initializeTheme() {
  if (typeof window === "undefined") return;
  let stored: string | null = null;
  try {
    stored = window.localStorage.getItem(THEME_STORAGE_KEY);
  } catch {
    // A denied storage policy must not prevent the application from rendering.
  }
  const saved = parseThemePreference(stored);
  applyTheme(saved);
  if (!listening) {
    mediaQuery?.addEventListener("change", onSystemThemeChange);
    listening = true;
  }
}

export function setThemePreference(next: ThemePreference) {
  if (typeof window !== "undefined") {
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, next);
    } catch {
      // The in-memory preference still applies when storage is unavailable.
    }
  }
  applyTheme(next);
}

export function useTheme() {
  return {
    preference: readonly(preference),
    resolved: readonly(resolved),
    options: themeOptions,
    setPreference: setThemePreference,
  };
}
