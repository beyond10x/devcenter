import { Buffer } from "node:buffer";
import { expect, test } from "@playwright/test";

const workspacePath =
  "/projects/project-review-devcenter/sessions/workspace-session-review?pane=editor";
const realTerminalLab = Boolean(process.env.DEVCENTER_REVIEW_TERMINAL_UPSTREAM);

function frameText(payload: string | Buffer, sequencePrefix: boolean) {
  if (typeof payload === "string") return payload;
  return payload.subarray(sequencePrefix ? 8 : 0).toString("utf8");
}

test("runs the standalone hosted workbench and terminal without viewport overflow", async ({
  page,
}) => {
  const failures: string[] = [];
  const inputFrames: string[] = [];
  const outputFrames: string[] = [];
  let terminalConnections = 0;

  page.on("console", (message) => {
    if (message.type() === "error") failures.push(`console: ${message.text()}`);
  });
  page.on("pageerror", (error) => failures.push(`page: ${error.message}`));
  page.on("requestfailed", (request) =>
    failures.push(`request: ${request.url()} ${request.failure()?.errorText ?? "failed"}`),
  );
  page.on("response", (response) => {
    if (response.status() >= 400)
      failures.push(`http ${String(response.status())}: ${response.url()}`);
  });
  page.on("websocket", (socket) => {
    if (!socket.url().includes("/api/project-terminals/")) return;
    terminalConnections += 1;
    socket.on("framesent", ({ payload }) => inputFrames.push(frameText(payload, false)));
    socket.on("framereceived", ({ payload }) => outputFrames.push(frameText(payload, true)));
  });

  await page.goto(workspacePath);
  await expect(page.getByText("AgentIDE ready", { exact: true })).toBeVisible();
  await expect(page.locator(".hosted-monaco-editor")).toBeVisible();
  await expect(page.locator(".view-lines")).toContainText("use std::process::ExitCode");
  expect(await page.locator(".view-line span span").count()).toBeGreaterThan(3);
  await expect(page.getByRole("button", { name: /bind/i })).toHaveCount(0);
  if (realTerminalLab) {
    await expect(page.locator(".terminal-panel")).toBeVisible();
  } else {
    await expect(page.locator(".terminal-panel")).toHaveCount(0);
  }

  const staleAttach = await page.evaluate(
    async (socketOrigin) => {
      const started = performance.now();
      return await new Promise<{
        closeCode: number;
        elapsedMs: number;
        refusal: { kind?: unknown; code?: unknown } | null;
      }>((resolve, reject) => {
        const timeout = globalThis.setTimeout(
          () => reject(new Error("stale terminal attach did not finish")),
          2_000,
        );
        let refusal: { kind?: unknown; code?: unknown } | null = null;
        const socket = new WebSocket(`${socketOrigin}/api/project-terminals/terminal-stale/attach`);
        socket.addEventListener("message", (event) => {
          if (typeof event.data === "string") {
            refusal = JSON.parse(event.data) as { kind?: unknown; code?: unknown };
          }
        });
        socket.addEventListener("close", (event) => {
          globalThis.clearTimeout(timeout);
          resolve({ closeCode: event.code, elapsedMs: performance.now() - started, refusal });
        });
      });
    },
    new URL(page.url()).origin.replace(/^http/, "ws"),
  );
  expect(staleAttach.refusal).toEqual({
    kind: "refused",
    code: "workspace_terminal_not_found",
  });
  expect(staleAttach.closeCode).toBe(1008);
  expect(staleAttach.elapsedMs).toBeLessThan(1_000);

  const terminalButton = page.getByRole("button", { name: "Terminal", exact: true });
  await expect(terminalButton).toBeVisible();
  const terminalBounds = await terminalButton.boundingBox();
  const viewport = page.viewportSize();
  expect(terminalBounds).not.toBeNull();
  expect(viewport).not.toBeNull();
  expect((terminalBounds?.y ?? 0) + (terminalBounds?.height ?? 0)).toBeLessThanOrEqual(
    viewport?.height ?? 0,
  );

  await page.setViewportSize({ width: 760, height: 700 });
  const compactTerminalBounds = await terminalButton.boundingBox();
  expect(compactTerminalBounds).not.toBeNull();
  expect(
    (compactTerminalBounds?.y ?? 0) + (compactTerminalBounds?.height ?? 0),
  ).toBeLessThanOrEqual(700);

  if (!realTerminalLab) await terminalButton.click();
  await expect(page.locator(".terminal-profile-actions small")).toContainText("network none");
  if (!realTerminalLab) {
    await page.getByRole("button", { name: "Open terminal", exact: true }).click();
  }
  await expect(page.locator(".terminal-connection.running")).toBeVisible();
  await expect(page.locator(".ghostty-host")).toBeVisible();
  const terminalCanvas = page.locator(".ghostty-host canvas");
  await expect(terminalCanvas).toBeVisible();
  const compactTerminalHostBounds = await terminalCanvas.boundingBox();
  expect(compactTerminalHostBounds).not.toBeNull();
  expect(
    (compactTerminalHostBounds?.y ?? 0) + (compactTerminalHostBounds?.height ?? 0),
  ).toBeLessThanOrEqual(700);

  inputFrames.length = 0;
  outputFrames.length = 0;
  await page.getByRole("textbox", { name: "Terminal input" }).first().focus();
  await page.keyboard.type("pwd");
  await page.keyboard.press("Enter");
  await expect.poll(() => inputFrames.join(""), { timeout: 5_000 }).toContain("pwd");
  await expect.poll(() => outputFrames.join(""), { timeout: 5_000 }).toContain("/workspace");

  const initialConnections = terminalConnections;
  await page.reload();
  await expect(page.locator(".terminal-connection.running")).toBeVisible();
  await expect.poll(() => terminalConnections).toBeGreaterThan(initialConnections);

  await page.getByTitle("Detach terminal tab").click();
  await expect(page.getByText("No attached terminals", { exact: true })).toBeVisible();
  if (realTerminalLab) {
    await page.reload();
  } else {
    await page.getByRole("button", { name: "Open terminal", exact: true }).click();
  }
  await expect(page.locator(".terminal-connection.running")).toBeVisible();
  await page.getByTitle("Kill terminal process").click();
  await expect(page.locator(".terminal-refused strong")).toHaveText("Terminal terminated");

  expect(failures).toEqual([]);
});
