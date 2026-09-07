import { test, expect } from "@playwright/test";
import { writeFileSync } from "node:fs";
import { z } from "zod";

// Small explicit pages exercise the real service's cursor and filtered-row contract.
// The BFF's production page bound and refusal cases are covered by its Rust gate.
test("closed coordination history can be read beyond the first page", async ({ request }) => {
  const root = z.string().parse(process.env.DEVCENTER_EVIDENCE_ROOT);
  const projectId = z.string().min(1).parse(process.env.DEVCENTER_PROJECT_ID);
  const response = await request.get(`/api/projects/${encodeURIComponent(projectId)}`);
  expect(response.ok()).toBe(true);
  const project = z
    .object({ id: z.string(), pinned_commit: z.string().optional() })
    .parse(await response.json());
  expect(project.id).toBe(projectId);
  const catalog = await request.post("/api/services/catalog", {
    data: { service_ref: "service:agentide" },
  });
  expect(catalog.ok()).toBe(true);
  const operations = z
    .object({ operations: z.array(z.object({ operation_ref: z.string() })) })
    .parse(await catalog.json());
  for (const name of ["ensure_hosted_session", "close_session", "get_session"])
    expect(
      operations.operations.some((operation) => operation.operation_ref === `agentide.${name}`),
    ).toBe(true);
  async function invoke<T>(operation: string, input: object, schema: z.ZodType<T>) {
    const response = await request.post("/api/services/invoke", {
      data: { operation_ref: `agentide.${operation}`, input, confirmed: true },
      timeout: 30_000,
    });
    expect(response.ok(), `${operation}: ${String(response.status())}`).toBe(true);
    return z
      .object({ output: schema, connector_audit_ref: z.string().min(1) })
      .parse(await response.json()).output;
  }
  const pageSchema = z.object({
    items: z.array(z.object({ session_id: z.string(), state: z.string() })),
    next_cursor: z.string().nullable(),
    partial: z.boolean(),
    through_version: z.number().nullable(),
  });
  async function existing(id: string) {
    let cursor: string | undefined;
    for (let page = 0; page < 100; page++) {
      const observed = await invoke(
        "get_session",
        { session_id: id, $page: { limit: 1000, ...(cursor ? { cursor } : {}) } },
        pageSchema,
      );
      const item = observed.items.find((row) => row.session_id === id);
      if (item) return { state: item.state, version: z.number().parse(observed.through_version) };
      if (!observed.next_cursor) return undefined;
      cursor = observed.next_cursor;
    }
    throw new Error("history lookup exceeded its page bound");
  }
  const intent = z.object({ through_version: z.number(), outcome: z.string() });
  const ids = Array.from(
    { length: 40 },
    (_, index) => `history-${project.id}-${String(index).padStart(3, "0")}`,
  );
  // Independent aggregates; cap concurrency to avoid turning acceptance into a load test.
  for (let offset = 0; offset < ids.length; offset += 4) {
    await Promise.all(
      ids.slice(offset, offset + 4).map(async (id) => {
        const observed = await existing(id);
        if (observed?.state === "Closed") return;
        const created = observed
          ? { through_version: observed.version }
          : await invoke(
              "ensure_hosted_session",
              {
                session_id: id,
                workspace_session_id: id,
                workspace_root: "substrate:git:closed-fixture-history",
                manifest_digest: "0".repeat(64),
                project_id: project.id,
                source_revision: project.pinned_commit ?? "0".repeat(40),
                objective: "Closed local acceptance history",
                scopes: {},
                idempotency_key: `create-${id}`,
              },
              intent,
            );
        await invoke(
          "close_session",
          {
            session_id: id,
            request_id: `close-${id}`,
            expected_version: created.through_version,
            idempotency_key: `close-${id}`,
          },
          intent,
        );
      }),
    );
  }
  let target: string | undefined;
  for (const id of ids.slice(0, 5)) {
    const first = await invoke("get_session", { session_id: id, $page: { limit: 4 } }, pageSchema);
    if (first.items.length === 0 && first.partial) {
      target = id;
      break;
    }
  }
  if (!target) throw new Error("no known history record beyond the first page");
  const pages: Array<{ items: number; partial: boolean; version: number | null }> = [];
  const cursors = new Set<string>();
  let cursor: string | undefined;
  let found = false;
  do {
    const result = await invoke(
      "get_session",
      { session_id: target, $page: { limit: 4, ...(cursor ? { cursor } : {}) } },
      pageSchema,
    );
    pages.push({
      items: result.items.length,
      partial: result.partial,
      version: result.through_version,
    });
    for (const item of result.items) {
      expect(item.session_id).toBe(target);
      expect(item.state).toBe("Closed");
      expect(result.through_version).toBeGreaterThan(0);
      found = true;
    }
    expect(result.partial).toBe(Boolean(result.next_cursor));
    cursor = result.next_cursor ?? undefined;
    if (cursor) {
      expect(cursors.has(cursor)).toBe(false);
      cursors.add(cursor);
    }
    expect(pages.length).toBeLessThan(100);
  } while (cursor);
  expect(found).toBe(true);
  expect(pages.length).toBeGreaterThan(1);
  expect(pages[0]?.items).toBe(0);
  expect(pages.slice(1).some((page) => page.items > 0)).toBe(true);
  writeFileSync(
    `${root}/history.json`,
    JSON.stringify(
      { result: "HISTORY_PAGINATION_PASS", closed_sessions: ids.length, page_limit: 4, pages },
      null,
      2,
    ),
  );
});
