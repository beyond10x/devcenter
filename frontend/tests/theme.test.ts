import { beforeEach, describe, expect, it, vi } from "vitest";
import {
  THEME_STORAGE_KEY,
  initializeTheme,
  parseThemePreference,
  resolveTheme,
  setThemePreference,
  themeColor,
} from "@/theme/theme";

describe("theme preference", () => {
  beforeEach(() => {
    window.localStorage.clear();
    document.documentElement.removeAttribute("data-theme");
    document.documentElement.removeAttribute("data-theme-preference");
    document.head.innerHTML = '<meta name="theme-color" content="">';
    Object.defineProperty(window, "matchMedia", {
      configurable: true,
      value: vi.fn().mockReturnValue({
        matches: false,
        addEventListener: vi.fn(),
        removeEventListener: vi.fn(),
      }),
    });
  });

  it("falls back safely and resolves the system preference", () => {
    expect(parseThemePreference(undefined)).toBe("system");
    expect(parseThemePreference("unknown")).toBe("system");
    expect(parseThemePreference("solarized-dark")).toBe("solarized-dark");
    expect(resolveTheme("system", false)).toBe("light");
    expect(resolveTheme("system", true)).toBe("dark");
    expect(resolveTheme("monokai", false)).toBe("monokai");
  });

  it("applies and persists a named local theme without a server profile", () => {
    initializeTheme();
    setThemePreference("monokai");

    expect(window.localStorage.getItem(THEME_STORAGE_KEY)).toBe("monokai");
    expect(document.documentElement.dataset.themePreference).toBe("monokai");
    expect(document.documentElement.dataset.theme).toBe("monokai");
    expect(document.documentElement.style.colorScheme).toBe("dark");
    expect(document.querySelector('meta[name="theme-color"]')?.getAttribute("content")).toBe(
      themeColor("monokai"),
    );
  });
});
