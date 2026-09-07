import { test, expect } from "@playwright/test";
import { writeFileSync } from "node:fs";
import { randomBytes } from "node:crypto";
import { z } from "zod";

test("real Claude connection and main Agents reply", async ({ page, context }) => {
  process.umask(0o077);
  const origin = z.url().parse(process.env.DEVCENTER_ORIGIN);
  const root = z.string().parse(process.env.DEVCENTER_EVIDENCE_ROOT);
  expect(process.env.DEVCENTER_PROVIDER_MODE).toBe("live");
  const evidence: Record<string, unknown> = { provider_mode: "live", result: "NOT_COMPLETED" };
  const save = () => writeFileSync(`${root}/model.json`, JSON.stringify(evidence, null, 2));
  save();
  try {
    const connection = await context.request.get(`${origin}/api/connectors/claude-code`);
    evidence.connection_status = connection.status();
    expect(connection.ok()).toBe(true);
    const status = z.object({ connected: z.boolean() }).parse(await connection.json());
    if (!status.connected) {
      evidence.result = "AUTHORIZATION_REQUIRED";
      throw new Error(
        `Authorize Claude at ${origin}/connectors?tab=connections, then rerun local test. Acceptance is incomplete.`,
      );
    }
    const response = await context.request.get(`${origin}/api/agents`);
    expect(response.ok()).toBe(true);
    const agents = z
      .array(z.object({ id: z.string(), name: z.string() }))
      .parse(await response.json());
    const agent = agents.find((candidate) => candidate.name === "Local acceptance agent");
    if (!agent)
      throw new Error("Run local up to create the acceptance agent through the normal UI.");
    const expected = `LIVE_MODEL_${randomBytes(12).toString("hex")}_OK`;
    await page.goto(`/agents/${agent.id}`);
    await page
      .getByPlaceholder("Message this agent…")
      .fill(`Diagnostic check: reply with exactly ${expected}. Do not use tools or modify files.`);
    const admitted = page.waitForResponse(
      (result) =>
        result.request().method() === "POST" &&
        new URL(result.url()).pathname === `/api/agents/${agent.id}/tasks`,
    );
    await page.getByRole("button", { name: "Send", exact: true }).click();
    const admission = await admitted;
    evidence.admission_status = admission.status();
    expect(admission.status()).toBe(202);
    const task = z.object({ id: z.string() }).parse(await admission.json());
    evidence.task_id = task.id;
    await expect
      .poll(
        async () => {
          const observed = await context.request.get(`${origin}/api/tasks/${task.id}`);
          expect(observed.ok()).toBe(true);
          const result = z
            .object({
              status: z.string(),
              output: z.string().nullable().optional(),
              failure: z.object({ code: z.string() }).nullable().optional(),
            })
            .parse(await observed.json());
          evidence.task_status = result.status;
          evidence.failure_code = result.failure?.code;
          save();
          if (["failed", "cancelled", "canceled", "expired"].includes(result.status))
            throw new Error(
              `Real Claude attempt ${result.status}: ${result.failure?.code ?? "no failure code"}`,
            );
          if (result.status === "succeeded") expect(result.output?.trim()).toBe(expected);
          return result.status;
        },
        { timeout: 120_000, intervals: [1000] },
      )
      .toBe("succeeded");
    await expect(page.getByText(expected, { exact: true })).toBeVisible();
    evidence.result = "LIVE_MODEL_REPLY_PASS";
  } finally {
    save();
    await page.screenshot({ path: `${root}/model.png`, animations: "disabled" });
  }
});
