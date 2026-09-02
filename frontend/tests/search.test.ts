import { describe, expect, it } from "vitest";
import { rankSearchEntries, scoreSearchEntry, type SearchEntry } from "@/app/search";

const sectionEntry: SearchEntry = {
  key: "section:services",
  group: "Sections",
  label: "Services",
  detail: "Go to Services",
  to: "/services",
  terms: "generated synthesized applications",
};
const serviceEntry: SearchEntry = {
  key: "service:todo",
  group: "Services",
  label: "Todo",
  detail: "Shared scoped lists and intent-driven items",
  to: "/services?service=service:todo",
  terms: "service:todo",
};
const entries: SearchEntry[] = [
  sectionEntry,
  serviceEntry,
  {
    key: "agent:release",
    group: "Agents",
    label: "Release steward",
    detail: "Agent revision 3",
    to: "/agents/release",
  },
];

describe("global search ranking", () => {
  it("prioritizes exact and prefix matches over descriptive matches", () => {
    expect(scoreSearchEntry(serviceEntry, "todo")).toBe(1_000);
    expect(scoreSearchEntry(sectionEntry, "serv")).toBe(800);
    expect(scoreSearchEntry(serviceEntry, "scoped items")).toBeGreaterThan(500);
    expect(rankSearchEntries(entries, "todo").map((entry) => entry.key)).toEqual(["service:todo"]);
  });

  it("keeps an empty-query launcher useful and caps noisy groups", () => {
    const manyServices = Array.from({ length: 8 }, (_, index): SearchEntry => ({
      key: `service:${String(index)}`,
      group: "Services",
      label: `Service ${String(index)}`,
      detail: "Generated service",
      to: "/services",
    }));
    const ranked = rankSearchEntries([...entries, ...manyServices], "service", 3);

    expect(ranked.filter((entry) => entry.group === "Services")).toHaveLength(3);
    expect(rankSearchEntries(entries, "")[0]?.group).toBe("Sections");
  });
});
