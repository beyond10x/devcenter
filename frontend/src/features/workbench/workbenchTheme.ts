import type { ResolvedTheme } from "@/theme/theme";

export const WORKBENCH_MONO_FONT =
  '"JetBrains Mono Variable", "JetBrains Mono", ui-monospace, SFMono-Regular, Menlo, Consolas, monospace';

export interface TerminalTheme {
  foreground: string;
  background: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;
  selectionForeground: string;
  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;
  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

export interface SyntaxColors {
  keyword: string;
  type: string;
  string: string;
  number: string;
  comment: string;
  operator: string;
}

export interface WorkbenchTheme {
  id: ResolvedTheme;
  monacoName: string;
  monacoBase: "vs" | "vs-dark";
  editorBackground: string;
  editorForeground: string;
  editorLineHighlight: string;
  editorSelection: string;
  editorInactiveSelection: string;
  editorCursor: string;
  editorLineNumber: string;
  editorActiveLineNumber: string;
  syntax: SyntaxColors;
  terminal: TerminalTheme;
}

const solarizedSyntax: SyntaxColors = {
  keyword: "#268bd2",
  type: "#2aa198",
  string: "#859900",
  number: "#d33682",
  comment: "#839496",
  operator: "#6c71c4",
};

const themes: Record<ResolvedTheme, WorkbenchTheme> = {
  light: {
    id: "light",
    monacoName: "devcenter-light",
    monacoBase: "vs",
    editorBackground: "#ffffff",
    editorForeground: "#14201e",
    editorLineHighlight: "#f3f7f5",
    editorSelection: "#bddbd2",
    editorInactiveSelection: "#dcebe6",
    editorCursor: "#176b5b",
    editorLineNumber: "#81908b",
    editorActiveLineNumber: "#176b5b",
    syntax: {
      keyword: "#005fb8",
      type: "#7a3e9d",
      string: "#a31515",
      number: "#8a5a00",
      comment: "#567067",
      operator: "#4f46a5",
    },
    terminal: darkTerminal("#14201e", "#e7f4e5", "#58d3b0"),
  },
  dark: {
    id: "dark",
    monacoName: "devcenter-dark",
    monacoBase: "vs-dark",
    editorBackground: "#0b1210",
    editorForeground: "#dceae5",
    editorLineHighlight: "#13201c",
    editorSelection: "#275749",
    editorInactiveSelection: "#1d3b33",
    editorCursor: "#66d9ef",
    editorLineNumber: "#71857e",
    editorActiveLineNumber: "#71d3a8",
    syntax: {
      keyword: "#66d9ef",
      type: "#71d3a8",
      string: "#f1c86b",
      number: "#ff9e64",
      comment: "#82958e",
      operator: "#c792ea",
    },
    terminal: darkTerminal("#0b1210", "#dceae5", "#66d9ef"),
  },
  monokai: {
    id: "monokai",
    monacoName: "devcenter-monokai",
    monacoBase: "vs-dark",
    editorBackground: "#1e1f1c",
    editorForeground: "#f8f8f2",
    editorLineHighlight: "#2b2c27",
    editorSelection: "#4b4d42",
    editorInactiveSelection: "#3a3b34",
    editorCursor: "#a6e22e",
    editorLineNumber: "#85867d",
    editorActiveLineNumber: "#f8f8f2",
    syntax: {
      keyword: "#f92672",
      type: "#66d9ef",
      string: "#e6db74",
      number: "#ae81ff",
      comment: "#9b9c91",
      operator: "#f92672",
    },
    terminal: {
      foreground: "#f8f8f2",
      background: "#1e1f1c",
      cursor: "#a6e22e",
      cursorAccent: "#1e1f1c",
      selectionBackground: "#49483e",
      selectionForeground: "#f8f8f2",
      black: "#272822",
      red: "#f92672",
      green: "#a6e22e",
      yellow: "#e6db74",
      blue: "#66d9ef",
      magenta: "#ae81ff",
      cyan: "#a1efe4",
      white: "#f8f8f2",
      brightBlack: "#75715e",
      brightRed: "#ff5c8a",
      brightGreen: "#c2f05a",
      brightYellow: "#f4e98a",
      brightBlue: "#8be9fd",
      brightMagenta: "#c7a5ff",
      brightCyan: "#c2fff6",
      brightWhite: "#ffffff",
    },
  },
  "solarized-light": {
    id: "solarized-light",
    monacoName: "devcenter-solarized-light",
    monacoBase: "vs",
    editorBackground: "#fffdf5",
    editorForeground: "#073642",
    editorLineHighlight: "#f5efd9",
    editorSelection: "#d8d1ba",
    editorInactiveSelection: "#e8e1cc",
    editorCursor: "#006d91",
    editorLineNumber: "#839496",
    editorActiveLineNumber: "#006d91",
    syntax: solarizedSyntax,
    terminal: solarizedTerminal("#002b36", "#eee8d5", "#56bff2"),
  },
  "solarized-dark": {
    id: "solarized-dark",
    monacoName: "devcenter-solarized-dark",
    monacoBase: "vs-dark",
    editorBackground: "#001f27",
    editorForeground: "#eee8d5",
    editorLineHighlight: "#073642",
    editorSelection: "#15515e",
    editorInactiveSelection: "#0d414d",
    editorCursor: "#e4bb43",
    editorLineNumber: "#839496",
    editorActiveLineNumber: "#eee8d5",
    syntax: solarizedSyntax,
    terminal: solarizedTerminal("#001f27", "#eee8d5", "#e4bb43"),
  },
};

export function resolveWorkbenchTheme(value?: string): WorkbenchTheme {
  return themes[isResolvedTheme(value) ? value : "light"];
}

export function currentWorkbenchTheme(): WorkbenchTheme {
  return resolveWorkbenchTheme(document.documentElement.dataset.theme);
}

function isResolvedTheme(value: string | undefined): value is ResolvedTheme {
  return value !== undefined && Object.prototype.hasOwnProperty.call(themes, value);
}

function darkTerminal(background: string, foreground: string, cursor: string): TerminalTheme {
  return {
    foreground,
    background,
    cursor,
    cursorAccent: background,
    selectionBackground: "#dceae5",
    selectionForeground: background,
    black: "#0b1210",
    red: "#ff786d",
    green: "#58d3b0",
    yellow: "#f1c86b",
    blue: "#66d9ef",
    magenta: "#c792ea",
    cyan: "#7fdbca",
    white: "#dceae5",
    brightBlack: "#71857e",
    brightRed: "#ff9a92",
    brightGreen: "#82e8c1",
    brightYellow: "#ffe18d",
    brightBlue: "#8be9fd",
    brightMagenta: "#deb7ff",
    brightCyan: "#a8f2e3",
    brightWhite: "#ffffff",
  };
}

function solarizedTerminal(background: string, foreground: string, cursor: string): TerminalTheme {
  return {
    foreground,
    background,
    cursor,
    cursorAccent: background,
    selectionBackground: "#586e75",
    selectionForeground: "#fdf6e3",
    black: "#073642",
    red: "#dc322f",
    green: "#859900",
    yellow: "#b58900",
    blue: "#268bd2",
    magenta: "#d33682",
    cyan: "#2aa198",
    white: "#eee8d5",
    brightBlack: "#657b83",
    brightRed: "#ff7169",
    brightGreen: "#a8bd13",
    brightYellow: "#e4bb43",
    brightBlue: "#56bff2",
    brightMagenta: "#f05fa7",
    brightCyan: "#55cfc3",
    brightWhite: "#fdf6e3",
  };
}
