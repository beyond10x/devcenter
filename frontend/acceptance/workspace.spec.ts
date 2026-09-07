/// <reference lib="dom" />
import { test, expect } from "@playwright/test";
import { mkdtempSync, writeFileSync } from "node:fs";
import { createHash, randomBytes, randomUUID } from "node:crypto";
import { z } from "zod";

declare global {
  interface Window {
    localAcceptanceViolations: string[];
  }
}
const sessionSchema = z.object({
  id: z.string(),
  state: z.string(),
  failure_code: z.string().nullable().optional(),
  coordination: z.object({ state: z.string() }).nullable().optional(),
});
const fileSchema = z.object({
  content: z.string(),
  revision: z.object({ sha256: z.string() }),
  binary: z.boolean().optional(),
  truncated: z.boolean().optional(),
});
const terminalSchema = z.object({ id: z.string(), state: z.string() });
const taskSchema = z.object({ status: z.string(), output: z.string().nullable().optional() });
const messageSchema = z.object({ sequence: z.number(), role: z.string(), content: z.string() });
const expectedReply = `WORKSPACE_${randomBytes(12).toString("hex")}_OK`;
const prompt = `Diagnostic check: reply with exactly ${expectedReply}. Do not use tools or modify files.`;
const hash = (value: string) => createHash("sha256").update(value).digest("hex");

test("Files, real PTY and both agent surfaces", async ({ page, context }, testInfo) => {
  process.umask(0o077);
  expect(process.env.DEVCENTER_PROVIDER_MODE).toBe("live");
  const origin = z.url().parse(process.env.DEVCENTER_ORIGIN);
  const projectId = z
    .string()
    .regex(/^project-[a-zA-Z0-9_-]+$/)
    .parse(process.env.DEVCENTER_PROJECT_ID);
  const root = mkdtempSync(
    `${z.string().parse(process.env.DEVCENTER_EVIDENCE_ROOT)}/deployment-acceptance-`,
  );
  const requests: Array<{ method: string; path: string; status: number }> = [];
  const errors: string[] = [];
  const cleanup: Array<{ id: string; state: string; coordination?: string }> = [];
  const owned = new Set<string>();
  const evidence: Record<string, unknown> = {
    at: new Date().toISOString(),
    provider_mode: process.env.DEVCENTER_PROVIDER_MODE,
    result: "NOT_COMPLETED",
    requests,
    javascript_errors: errors,
    cleanup,
  };
  const save = () =>
    writeFileSync(
      `${root}/result.json`,
      JSON.stringify({ ...evidence, owned_sessions: [...owned] }, null, 2),
    );
  const step = (name: string) => {
    evidence.step = name;
    save();
  };
  page.setDefaultTimeout(20_000);
  page.on("pageerror", (error) => errors.push(error.name));
  await context.addInitScript(() => {
    window.localAcceptanceViolations = [];
    addEventListener("securitypolicyviolation", (event) =>
      window.localAcceptanceViolations.push(event.effectiveDirective),
    );
  });
  let sessionId = "";
  let original: z.infer<typeof fileSchema> | undefined;
  let expectedSavedHash: string | undefined;
  let edited = false;
  let entryOwnership: Promise<void> | undefined;
  let failure: Error | undefined;
  const filePath = () => `/api/project-sessions/${sessionId}/files/README.md`;
  async function api<T>(
    method: string,
    path: string,
    schema: z.ZodType<T>,
    data?: object,
  ): Promise<T> {
    const response = await context.request.fetch(origin + path, {
      method,
      data,
      maxRedirects: 0,
      timeout: 30_000,
    });
    requests.push({ method, path, status: response.status() });
    save();
    expect(response.ok(), `${method} ${path}: ${String(response.status())}`).toBe(true);
    return schema.parse(await response.json());
  }
  async function restore() {
    if (!original || !edited) return;
    const current = await api("GET", filePath(), fileSchema);
    if (current.content !== original.content) {
      if (!expectedSavedHash || current.revision.sha256 !== expectedSavedHash)
        throw new Error("concurrent_edit_preserved_restore_refused");
      await api("PUT", filePath(), z.unknown(), {
        content: original.content,
        expected: { state: "sha256", sha256: current.revision.sha256 },
        create_parents: false,
        operation_id: randomUUID(),
      });
    }
    const restored = await api("GET", filePath(), fileSchema);
    expect(restored.content).toBe(original.content);
    expect(restored.revision.sha256).toBe(original.revision.sha256);
    evidence.restored_exact = true;
    edited = false;
    save();
  }
  async function focus(button: string, pane: string) {
    const persisted = page.waitForResponse((response) => {
      if (
        new URL(response.url()).pathname !== `/api/project-sessions/${sessionId}/workbench` ||
        response.request().method() !== "POST"
      )
        return false;
      const data = z
        .object({ action: z.object({ pane_id: z.string().optional() }) })
        .safeParse(response.request().postDataJSON());
      return data.success && data.data.action.pane_id === pane;
    });
    await page.getByRole("button", { name: button, exact: true }).click();
    expect((await persisted).ok()).toBe(true);
  }
  try {
    step("Files_entry");
    const navigation = await page.goto(`/projects/${projectId}`, { waitUntil: "domcontentloaded" });
    expect(navigation?.ok()).toBe(true);
    const policy = navigation?.headers()["content-security-policy"] ?? "";
    const directives = policy.split(";").map((item) => item.trim());
    expect(directives).toContain("style-src-attr 'unsafe-inline'");
    expect(directives).toContain("font-src 'self'");
    for (const directive of ["script-src", "style-src"])
      expect(
        directives.some(
          (item) => item.startsWith(`${directive} 'self'`) && item.includes("'nonce-"),
        ),
      ).toBe(true);
    expect(directives.filter((item) => item.includes("unsafe-inline"))).toEqual([
      "style-src-attr 'unsafe-inline'",
    ]);
    await page.locator(".projects-view .view-header h1").waitFor();
    const project = await api(
      "GET",
      `/api/projects/${projectId}`,
      z.object({ pinned_commit: z.string() }),
    );
    const started = Date.now();
    const observeEntry = (response: import("@playwright/test").Response) => {
      if (
        new URL(response.url()).pathname === `/api/projects/${projectId}/sessions` &&
        response.request().method() === "POST" &&
        response.ok()
      ) {
        entryOwnership = response.json().then((body: unknown) => {
          sessionId = sessionSchema.parse(body).id;
          owned.add(sessionId);
          save();
        });
      }
    };
    page.on("response", observeEntry);
    await page
      .getByRole("navigation", { name: "Project areas" })
      .getByRole("button", { name: "files", exact: true })
      .click();
    await page.waitForURL((url) => url.pathname.startsWith(`/projects/${projectId}/sessions/`), {
      timeout: 30_000,
    });
    page.off("response", observeEntry);
    await entryOwnership;
    await page.locator("[data-agentide-renderer='vue']").waitFor();
    await expect(page.getByLabel("Message the agent")).toHaveCount(0);
    evidence.files_entry = "FILES_ENTRY_PASS";
    if (!sessionId) {
      const created = await api("POST", `/api/projects/${projectId}/sessions`, sessionSchema, {
        source_revision: project.pinned_commit,
        idempotency_key: randomUUID(),
      });
      sessionId = created.id;
      owned.add(sessionId);
      await page.goto(`/projects/${projectId}/sessions/${sessionId}?pane=editor`);
    }
    evidence.session_id = sessionId;
    await expect
      .poll(
        async () => {
          const session = await api("GET", `/api/project-sessions/${sessionId}`, sessionSchema);
          if (!["ready", "preparing", "unknown"].includes(session.state))
            throw new Error(`workspace_${session.state}_${session.failure_code ?? "unspecified"}`);
          return session.state;
        },
        { timeout: 100_000, intervals: [1000] },
      )
      .toBe("ready");
    evidence.ready_ms = Date.now() - started;
    await api("GET", `/api/project-sessions/${sessionId}/tree?path=&limit=500`, z.unknown());
    // The API can become ready before the browser polls again. Its initial
    // explorer also has a "Load workspace" button, so require a real file.
    const readme = page
      .locator(".explorer")
      .getByRole("button", { name: /^README\.md(?:\s*◫)?$/i })
      .first();
    await readme.waitFor({ timeout: 100_000 });
    await page.getByLabel("Workspace loading progress").waitFor({ state: "hidden" });
    await page.waitForLoadState("networkidle");
    step("Files_navigation");
    for (let cycle = 0; cycle < 2; cycle++) {
      await focus("Agent chat", "chat");
      await page.getByLabel("Message the agent").waitFor();
      await focus("Workspace explorer", "files");
      await page
        .getByText("Select a file from the workspace explorer to start editing.", { exact: true })
        .waitFor();
      if (cycle === 0) await page.getByRole("button", { name: "Close Files", exact: true }).click();
    }
    evidence.files_navigation = "FILES_NAVIGATION_PASS";
    await page.screenshot({ path: `${root}/files-entry.png`, animations: "disabled" });
    await readme.click();
    const editor = page.locator(".editor-leaf .monaco-editor");
    await editor.waitFor();
    original = await api("GET", filePath(), fileSchema);
    expect(original.binary).not.toBe(true);
    expect(original.truncated).not.toBe(true);
    expect(hash(original.content)).toBe(original.revision.sha256);
    step("editor_layout_and_save");
    await expect
      .poll(async () => {
        const geometry = await page.locator(".editor-leaf .view-line").evaluateAll((lines) => {
          const bounds = lines.map((line) => line.getBoundingClientRect());
          return {
            lines: bounds.length,
            non_overlapping:
              bounds.length >= 40 &&
              bounds.every(
                (rect, i) => rect.height === 20 && (i === 0 || rect.top >= bounds[i - 1].top + 20),
              ),
          };
        });
        evidence.editor_geometry = geometry;
        return geometry.non_overlapping;
      })
      .toBe(true);
    expect(await page.evaluate(() => window.localAcceptanceViolations)).toEqual([]);
    const marker = `\n<!-- reversible workspace smoke ${randomUUID()} -->\n`;
    // Set the expected hash before sending the save, so timeout recovery is safe.
    expectedSavedHash = hash(original.content + marker);
    await editor.click();
    await page.keyboard.press("Control+End");
    await page.keyboard.type(marker);
    edited = true;
    await page.getByLabel("Unsaved changes").waitFor();
    const saved = page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname === filePath() && response.request().method() === "PUT",
    );
    await page.keyboard.press("Control+s");
    expect((await saved).ok()).toBe(true);
    expect((await api("GET", filePath(), fileSchema)).content).toBe(original.content + marker);
    await restore();
    await page.reload({ waitUntil: "domcontentloaded" });
    await readme.click();
    await editor.waitFor();
    expect(await page.evaluate(() => window.localAcceptanceViolations)).toEqual([]);
    expect(errors).toEqual([]);
    evidence.file_acceptance_pass = true;
    await page.screenshot({ path: `${root}/restored-workspace.png`, animations: "disabled" });

    step("real_terminal");
    const profiles = await api(
      "GET",
      `/api/project-sessions/${sessionId}/terminal-profiles`,
      z.array(z.unknown()),
    );
    expect(profiles.length).toBeGreaterThan(0);
    const sentinel = `TERMINAL_${randomBytes(8).toString("hex")}_OK`;
    let output = "";
    const transport = { received_binary_frames: 0, sent_binary_bytes: 0 };
    page.on("websocket", (socket) => {
      if (!new URL(socket.url()).pathname.startsWith("/api/project-terminals/")) return;
      socket.on("framereceived", (frame) => {
        if (Buffer.isBuffer(frame.payload) && frame.payload.length >= 8) {
          transport.received_binary_frames++;
          output = (output + frame.payload.subarray(8).toString("utf8")).slice(-65536);
        }
      });
      socket.on("framesent", (frame) => {
        if (Buffer.isBuffer(frame.payload)) transport.sent_binary_bytes += frame.payload.length;
      });
    });
    await page.getByRole("button", { name: "Terminal", exact: true }).click();
    const canvas = page.locator(".terminal-leaf canvas");
    await canvas.waitFor();
    const terminalId = z
      .string()
      .regex(/^[a-zA-Z0-9_-]+$/)
      .parse(await page.locator(".terminal-leaf").getAttribute("data-terminal-id"));
    await expect.poll(() => output.length).toBeGreaterThan(0);
    await canvas.click();
    expect(
      await page.evaluate(() => Boolean(document.activeElement?.closest(".terminal-leaf"))),
    ).toBe(true);
    await page.keyboard.type(`printf '%s%s\\n' '${sentinel.slice(0, 12)}' '${sentinel.slice(12)}'`);
    await page.keyboard.press("Enter");
    await expect.poll(() => output.includes(sentinel), { timeout: 10_000 }).toBe(true);
    expect(transport.received_binary_frames).toBeGreaterThan(0);
    expect(transport.sent_binary_bytes).toBeGreaterThan(0);
    const stopped = await api("DELETE", `/api/project-terminals/${terminalId}`, terminalSchema);
    expect(["terminated", "exited"]).toContain(stopped.state);
    expect((await api("GET", `/api/project-sessions/${sessionId}`, sessionSchema)).state).toBe(
      "ready",
    );
    await api("GET", `/api/project-sessions/${sessionId}/tree?path=&limit=500`, z.unknown());
    evidence.terminal = {
      result: "TERMINAL_EXEC_PASS",
      transport,
      termination_state: stopped.state,
      workspace_state_after_termination: "ready",
    };
    await page.screenshot({ path: `${root}/terminal.png`, animations: "disabled" });

    step("coding_agent");
    await focus("Agent chat", "chat");
    await page.waitForLoadState("networkidle");
    await page.getByLabel("Message the agent").fill(prompt);
    const accepted = page.waitForResponse(
      (response) =>
        new URL(response.url()).pathname.includes(`/api/project-sessions/${sessionId}/`) &&
        new URL(response.url()).pathname.includes("turns") &&
        response.request().method() === "POST",
    );
    const agentStarted = Date.now();
    await page.getByRole("button", { name: "Send", exact: true }).click();
    const admission = await accepted;
    expect(admission.status()).toBe(202);
    const task = z.object({ id: z.string() }).parse(await admission.json());
    await expect
      .poll(
        async () => {
          const result = await api("GET", `/api/tasks/${task.id}`, taskSchema);
          if (["failed", "cancelled", "canceled"].includes(result.status))
            throw new Error(`agent_${result.status}`);
          if (result.status === "succeeded") expect(result.output?.trim()).toBe(expectedReply);
          return result.status;
        },
        { timeout: 120_000, intervals: [1000] },
      )
      .toBe("succeeded");
    evidence.agent = { result: "AGENT_REPLY_PASS", reply_ms: Date.now() - agentStarted };
    await page.screenshot({ path: `${root}/coding-agent.png`, animations: "disabled" });
  } catch (error) {
    failure = error instanceof Error ? error : new Error("acceptance failed");
    evidence.error = error instanceof Error ? error.message : "unknown failure";
    await page
      .screenshot({ path: `${root}/failure.png`, animations: "disabled" })
      .catch(() => undefined);
  } finally {
    try {
      await entryOwnership;
      await restore();
      for (const id of owned) {
        const terminals = await api(
          "GET",
          `/api/project-sessions/${id}/terminals`,
          z.array(terminalSchema),
        );
        for (const terminal of terminals)
          if (!["terminated", "exited"].includes(terminal.state)) {
            expect(["terminated", "exited"]).toContain(
              (await api("DELETE", `/api/project-terminals/${terminal.id}`, terminalSchema)).state,
            );
          }
      }
    } catch (error) {
      failure ??= error instanceof Error ? error : new Error("acceptance failed");
      evidence.cleanup_error = "terminal or file cleanup failed";
    }
    // Check the independent project surface even when the workspace journey fails.
    try {
      step("project_agent");
      await page.goto(`/projects/${projectId}`, { waitUntil: "domcontentloaded" });
      await page
        .getByRole("navigation", { name: "Project areas" })
        .getByRole("button", { name: "chat", exact: true })
        .click();
      await page.locator(".project-composer textarea").fill(prompt);
      const sent = page.waitForResponse(
        (response) =>
          /\/api\/threads\/[^/]+\/messages$/.test(new URL(response.url()).pathname) &&
          response.request().method() === "POST",
      );
      await page.getByRole("button", { name: "Send to project agent" }).click();
      const response = await sent;
      expect(response.ok()).toBe(true);
      const admitted = z.object({ sequence: z.number() }).parse(await response.json());
      await expect
        .poll(
          async () => {
            const messages = await api(
              "GET",
              new URL(response.url()).pathname,
              z.array(messageSchema),
            );
            const reply = messages.find(
              (message) =>
                message.sequence > admitted.sequence &&
                ["assistant", "system"].includes(message.role),
            );
            if (reply) {
              expect(reply.role).toBe("assistant");
              expect(reply.content.trim()).toBe(expectedReply);
            }
            return Boolean(reply);
          },
          { timeout: 60_000, intervals: [1000] },
        )
        .toBe(true);
      evidence.project_agent = "AGENT_REPLY_PASS";
    } catch (error) {
      failure ??= error instanceof Error ? error : new Error("acceptance failed");
      evidence.project_agent = "AGENT_REPLY_FAILED";
    }
    for (const id of owned) {
      try {
        await expect
          .poll(
            async () => {
              const closed = await api("DELETE", `/api/project-sessions/${id}`, sessionSchema);
              cleanup.push({ id, state: closed.state, coordination: closed.coordination?.state });
              return closed.state === "closed" && closed.coordination?.state === "closed";
            },
            { timeout: 90_000, intervals: [1000] },
          )
          .toBe(true);
      } catch (error) {
        failure ??= error instanceof Error ? error : new Error("acceptance failed");
        evidence.cleanup_error = "workspace cleanup unconfirmed";
      }
    }
    evidence.result = failure ? "DEPLOYMENT_ACCEPTANCE_FAILED" : "DEPLOYMENT_ACCEPTANCE_PASS";
    save();
    await testInfo.attach("acceptance", {
      path: `${root}/result.json`,
      contentType: "application/json",
    });
  }
  if (failure) throw failure;
});
