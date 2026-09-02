import type { RouteLocationRaw } from "vue-router";

export type SearchGroup =
  | "Sections"
  | "Projects and repositories"
  | "Agents"
  | "Connections and connectors"
  | "Capability profiles"
  | "Services"
  | "MCP publications"
  | "Documentation";

export interface SearchEntry {
  key: string;
  group: SearchGroup;
  label: string;
  detail: string;
  to: RouteLocationRaw;
  terms?: string;
}

export interface RankedSearchEntry extends SearchEntry {
  score: number;
}

export function scoreSearchEntry(entry: SearchEntry, query: string): number | undefined {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return entry.group === "Sections" ? 100 : 1;
  const label = entry.label.toLocaleLowerCase();
  const detail = entry.detail.toLocaleLowerCase();
  const terms = `${entry.key} ${entry.terms ?? ""}`.toLocaleLowerCase();
  if (label === needle || entry.key.toLocaleLowerCase() === needle) return 1_000;
  if (label.startsWith(needle)) return 800;
  const words = needle.split(/\s+/).filter(Boolean);
  const haystack = `${label} ${detail} ${terms}`;
  if (words.every((word) => haystack.split(/[^a-z0-9]+/).some((part) => part.startsWith(word)))) {
    return 600 - haystack.indexOf(words[0] ?? "") / 1_000;
  }
  if (haystack.includes(needle)) return 400 - haystack.indexOf(needle) / 1_000;
  return undefined;
}

export function rankSearchEntries(
  entries: SearchEntry[],
  query: string,
  perGroup = 5,
): RankedSearchEntry[] {
  const counts = new Map<SearchGroup, number>();
  return entries
    .map((entry) => {
      const score = scoreSearchEntry(entry, query);
      return score === undefined ? undefined : { ...entry, score };
    })
    .filter((entry): entry is RankedSearchEntry => entry !== undefined)
    .sort((left, right) => right.score - left.score || left.label.localeCompare(right.label))
    .filter((entry) => {
      const count = counts.get(entry.group) ?? 0;
      counts.set(entry.group, count + 1);
      return count < perGroup;
    });
}
