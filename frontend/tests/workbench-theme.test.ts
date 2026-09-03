import { describe, expect, it } from "vitest";
import { resolveWorkbenchTheme, WORKBENCH_MONO_FONT } from "@/features/workbench/workbenchTheme";

describe("hosted workbench themes", () => {
  it("uses one self-hosted monospace family for editor and terminal rendering", () => {
    expect(WORKBENCH_MONO_FONT).toContain("JetBrains Mono Variable");
  });

  it("provides explicit syntax and ANSI palettes for every Devcenter theme", () => {
    for (const id of ["light", "dark", "monokai", "solarized-light", "solarized-dark"] as const) {
      const theme = resolveWorkbenchTheme(id);
      expect(theme.monacoName).toBe(`devcenter-${id}`);
      expect(new Set(Object.values(theme.syntax)).size).toBeGreaterThanOrEqual(5);
      expect(Object.keys(theme.terminal)).toHaveLength(22);
      expect(theme.terminal.background).not.toBe(theme.terminal.foreground);
    }
  });

  it("does not accept an untrusted document theme name", () => {
    expect(resolveWorkbenchTheme("spoofed").id).toBe("light");
  });
});
