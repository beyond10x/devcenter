import type { RouteLocationRaw } from "vue-router";

export interface NavigationItem {
  id: string;
  label: string;
  to: RouteLocationRaw;
  chord: string;
  searchTerms: string;
}

export interface Shortcut {
  keys: string[];
  macKeys?: string[];
  label: string;
}

export interface ShortcutGroup {
  label: string;
  shortcuts: Shortcut[];
}

export const navigationItems: NavigationItem[] = [
  {
    id: "projects",
    label: "Projects",
    to: "/projects",
    chord: "p",
    searchTerms: "repositories workspace git",
  },
  { id: "agents", label: "Agents", to: "/agents", chord: "a", searchTerms: "tasks assistants" },
  {
    id: "workflows",
    label: "Workflows",
    to: "/workflows",
    chord: "w",
    searchTerms: "automation definitions drafts revisions nodes",
  },
  {
    id: "connectors",
    label: "Connectors",
    to: "/connectors",
    chord: "c",
    searchTerms: "connections catalog integrations",
  },
  {
    id: "services",
    label: "Services",
    to: "/services",
    chord: "s",
    searchTerms: "generated synthesized applications",
  },
  {
    id: "profiles",
    label: "Capability profiles",
    to: "/profiles",
    chord: "f",
    searchTerms: "permissions authority posture",
  },
  {
    id: "publications",
    label: "MCP publications",
    to: "/publications",
    chord: "m",
    searchTerms: "remote tools clients",
  },
  {
    id: "docs",
    label: "Documentation",
    to: "/docs",
    chord: "d",
    searchTerms: "help guides openapi",
  },
];

export const shortcutGroups: ShortcutGroup[] = [
  {
    label: "Search",
    shortcuts: [{ keys: ["Ctrl", "K"], macKeys: ["⌘", "K"], label: "Search all" }],
  },
  {
    label: "Navigation",
    shortcuts: navigationItems.map((item) => ({
      keys: ["G", item.chord.toUpperCase()],
      label: item.label,
    })),
  },
  {
    label: "General",
    shortcuts: [
      { keys: ["?"], label: "Show keyboard shortcuts" },
      { keys: ["Esc"], label: "Close the active panel" },
    ],
  },
];
