import { test, expect, type Page, type APIRequestContext } from "@playwright/test";
import { randomBytes } from "node:crypto";
import { writeFileSync } from "node:fs";
import { z } from "zod";

const agentSchema = z.object({ id: z.string(), name: z.string(), active_revision: z.number() });
const conversationSchema = z.object({
  id: z.string(),
  title: z.string(),
  revision: z.number(),
  task_ids: z.array(z.string()),
});

async function createAgent(page: Page, name: string) {
  await page.goto("/agents");
  await page.getByRole("button", { name: "New agent", exact: true }).first().click();
  await page.getByLabel("Name", { exact: true }).fill(name);
  await page
    .getByLabel("Instructions", { exact: true })
    .fill(
      "Answer diagnostics exactly. Remember an example project name only within the current conversation. Do not use tools or change files.",
    );
  const response = page.waitForResponse(
    (r) => r.request().method() === "POST" && new URL(r.url()).pathname === "/api/agents",
  );
  await page.getByRole("button", { name: "Create and activate", exact: true }).click();
  const created = await response;
  expect(created.status()).toBe(201);
  const agent = agentSchema.parse(await created.json());
  await expect(page.getByRole("heading", { name, exact: true })).toBeVisible();
  return agent;
}

async function confirm(page: Page, name: string) {
  await page.getByRole("dialog").getByRole("button", { name, exact: true }).click();
  await expect(page.getByRole("dialog")).toBeHidden();
}

async function conversations(request: APIRequestContext, agent: string) {
  const response = await request.get(`/api/agents/${agent}/conversations`);
  expect(response.status()).toBe(200);
  return z.array(conversationSchema).parse(await response.json());
}

async function turn(
  page: Page,
  request: APIRequestContext,
  agent: string,
  prompt: string,
  expected: string,
) {
  await expect(page.getByPlaceholder("Message this agent…")).toBeEnabled();
  await page.getByPlaceholder("Message this agent…").fill(prompt);
  const response = page.waitForResponse(
    (r) =>
      r.request().method() === "POST" && new URL(r.url()).pathname === `/api/agents/${agent}/tasks`,
  );
  await page.getByRole("button", { name: "Send", exact: true }).click();
  const admitted = await response;
  expect(admitted.status()).toBe(202);
  const { id } = z.object({ id: z.string() }).parse(await admitted.json());
  await expect
    .poll(
      async () => {
        const response = await request.get(`/api/tasks/${id}`);
        expect(response.status()).toBe(200);
        const task = z
          .object({
            status: z.string(),
            output: z.string().nullish(),
            failure: z.object({ code: z.string(), message: z.string().optional() }).nullish(),
          })
          .parse(await response.json());
        if (["failed", "cancelled", "expired"].includes(task.status))
          throw new Error(
            `Real task ${id} ${task.status}: ${task.failure?.code ?? "unknown"}: ${task.failure?.message ?? ""}`,
          );
        if (task.status === "succeeded") expect(task.output?.trim()).toBe(expected);
        return task.status;
      },
      { timeout: 120_000, intervals: [1000] },
    )
    .toBe("succeeded");
  await expect(page.getByRole("button", { name: "New conversation", exact: true })).toBeEnabled();
  return id;
}

test("agent and profile lifecycle persists through reload with visible refusals", async ({
  page,
  context,
}) => {
  process.umask(0o077);
  const root = z.string().parse(process.env.DEVCENTER_EVIDENCE_ROOT);
  const prefix = `Lifecycle ${randomBytes(6).toString("hex")}`;
  const owned: string[] = [];
  let profileId: string | undefined;
  const evidence: Record<string, unknown> = { result: "NOT_COMPLETED" };
  try {
    const first = await createAgent(page, `${prefix} first`);
    owned.push(first.id);
    const second = await createAgent(page, `${prefix} second`);
    owned.push(second.id);
    await page.getByRole("button", { name: "Edit agent", exact: true }).click();
    await expect(page.getByLabel("Name", { exact: true })).toHaveValue(second.name);
    await page.getByLabel("Name", { exact: true }).fill(`${prefix} edited`);
    await page
      .getByLabel("Instructions", { exact: true })
      .fill("Answer diagnostics exactly. No external calls.");
    await page.getByRole("button", { name: "Save changes", exact: true }).click();
    await expect(page.getByRole("dialog")).toBeHidden();
    await page.reload();
    await expect(
      page.getByRole("heading", { name: `${prefix} edited`, exact: true }),
    ).toBeVisible();
    const details = await context.request.get(`/api/agents/${second.id}`);
    expect(details.status()).toBe(200);
    const updated = z
      .object({ agent: agentSchema, spec: z.object({ instructions: z.string() }) })
      .parse(await details.json());
    expect(updated.agent.active_revision).toBe(second.active_revision + 1);
    expect(updated.spec.instructions).toBe("Answer diagnostics exactly. No external calls.");
    evidence.repeated_create_and_edit = "PASS";
    await page.goto(`/agents/${first.id}`);
    await page.getByRole("button", { name: "Delete agent", exact: true }).click();
    await confirm(page, "Delete agent");
    await page.reload();
    const remaining = z
      .array(agentSchema)
      .parse(await (await context.request.get("/api/agents")).json());
    expect(remaining.some((agent) => agent.id === first.id)).toBe(false);
    evidence.agent_delete = "PASS";

    await page.goto("/profiles");
    await page.getByRole("button", { name: "New profile", exact: true }).first().click();
    await page.getByRole("dialog").getByLabel("Name", { exact: true }).fill(`${prefix} profile`);
    const creation = page.waitForResponse(
      (r) =>
        r.request().method() === "POST" && new URL(r.url()).pathname === "/api/capability-profiles",
    );
    await page.getByRole("button", { name: "Create profile", exact: true }).click();
    const profileResponse = await creation;
    expect(profileResponse.status()).toBe(201);
    profileId = z.object({ id: z.string() }).parse(await profileResponse.json()).id;
    await expect(page.getByRole("dialog")).toBeHidden();
    await page.getByRole("button", { name: "Allow all", exact: true }).click();
    await expect(
      page.getByRole("status").filter({ hasText: "Available capabilities are enabled" }),
    ).toBeVisible();
    await page.getByRole("button", { name: "Deny all", exact: true }).click();
    await expect(
      page.getByRole("status").filter({ hasText: "All visible capabilities are now denied" }),
    ).toBeVisible();
    await page.goto(`/agents/${second.id}`);
    await page.getByRole("button", { name: "Edit agent", exact: true }).click();
    await expect(page.getByRole("button", { name: "Save changes", exact: true })).toBeEnabled();
    await page.locator("#agent-profile").selectOption(profileId);
    await page.getByRole("button", { name: "Save changes", exact: true }).click();
    await expect(page.getByRole("dialog")).toBeHidden();
    await page.goto(`/profiles?profile=${profileId}`);
    await page.getByRole("button", { name: "Delete profile", exact: true }).click();
    await page
      .getByRole("dialog")
      .getByRole("button", { name: "Delete profile", exact: true })
      .click();
    await expect(page.getByRole("dialog").getByRole("alert")).toContainText(
      /active|uses|assigned|in use/i,
    );
    await page.getByRole("dialog").getByRole("button", { name: "Cancel", exact: true }).click();
    await page.goto(`/agents/${second.id}`);
    await page.getByRole("button", { name: "Delete agent", exact: true }).click();
    await confirm(page, "Delete agent");
    await page.goto(`/profiles?profile=${profileId}`);
    await page.getByRole("button", { name: "Delete profile", exact: true }).click();
    await confirm(page, "Delete profile");
    await page.reload();
    const profiles = z
      .array(z.object({ id: z.string() }))
      .parse(await (await context.request.get("/api/capability-profiles")).json());
    expect(profiles.some((profile) => profile.id === profileId)).toBe(false);
    evidence.profile_create_bulk_in_use_refusal_delete = "PASS";
    evidence.result = "LIFECYCLE_PASS";
  } finally {
    await page.screenshot({ path: `${root}/lifecycle.png`, animations: "disabled" });
    for (const id of owned) {
      const response = await context.request.delete(`/api/agents/${id}`);
      if (![204, 404].includes(response.status()))
        evidence[`cleanup_agent_${id}`] = response.status();
    }
    if (profileId) {
      const response = await context.request.delete(`/api/capability-profiles/${profileId}`);
      if (![204, 404].includes(response.status())) evidence.cleanup_profile = response.status();
    }
    writeFileSync(`${root}/lifecycle.json`, JSON.stringify(evidence, null, 2));
  }
});

test("separate conversations remember only their own real Claude turns and can be cleared or deleted", async ({
  page,
  context,
}) => {
  process.umask(0o077);
  const root = z.string().parse(process.env.DEVCENTER_EVIDENCE_ROOT);
  expect(process.env.DEVCENTER_PROVIDER_MODE).toBe("live");
  const nonce = randomBytes(10).toString("hex");
  // A harmless random phrase checks recall without resembling a credential. Preserve strict
  // equality: the second turn must recover information absent from its own request.
  const words = ["maple", "otter", "cabin", "violet", "meadow", "willow", "finch", "harbor"];
  const projectName = [...randomBytes(6)].map((byte) => words[byte % words.length]).join(" ");
  const evidence: Record<string, unknown> = { result: "NOT_COMPLETED", provider_mode: "live" };
  let agentId: string | undefined;
  try {
    const agent = await createAgent(page, `Conversation acceptance ${nonce}`);
    agentId = agent.id;
    await turn(
      page,
      context.request,
      agent.id,
      `My example project is named "${projectName}". Remember that name in this conversation. Reply exactly READY.`,
      "READY",
    );
    const first = (await conversations(context.request, agent.id))[0];
    await page.getByRole("button", { name: "Rename conversation", exact: true }).click();
    await page.getByLabel("Conversation name", { exact: true }).fill("Example project");
    await page.getByRole("button", { name: "Save name", exact: true }).click();
    await expect(page.getByLabel("Conversation name", { exact: true })).toBeHidden();
    await page.reload();
    await expect(page.getByLabel("Conversation", { exact: true })).toHaveValue(first.id);
    evidence.recall_task = await turn(
      page,
      context.request,
      agent.id,
      "Reply with only the name of my example project, without quotation marks.",
      projectName,
    );
    await page.getByRole("button", { name: "New conversation", exact: true }).click();
    await expect(page.getByLabel("Conversation", { exact: true })).not.toHaveValue(first.id);
    const secondId = await page.getByLabel("Conversation", { exact: true }).inputValue();
    await expect(page.getByText(projectName, { exact: true })).toHaveCount(0);
    evidence.isolation_task = await turn(
      page,
      context.request,
      agent.id,
      "What example project name did I give you in this conversation? If none, say: No project has been named.",
      "No project has been named.",
    );
    await page.getByLabel("Conversation", { exact: true }).selectOption(first.id);
    await page.getByRole("button", { name: "Clear conversation", exact: true }).click();
    await confirm(page, "Clear conversation");
    const replacement = await page.getByLabel("Conversation", { exact: true }).inputValue();
    expect(replacement).not.toBe(first.id);
    await page.reload();
    evidence.clear_task = await turn(
      page,
      context.request,
      agent.id,
      "What example project name did I give you in this conversation? If none, say: No project has been named.",
      "No project has been named.",
    );
    await page.getByLabel("Conversation", { exact: true }).selectOption(secondId);
    await page.getByRole("button", { name: "Delete conversation", exact: true }).click();
    await confirm(page, "Delete conversation");
    await page.reload();
    const retained = await conversations(context.request, agent.id);
    expect(retained.map((item) => item.id)).toEqual([replacement]);
    expect(retained[0]?.title).toBe("Example project");
    expect(retained[0]?.task_ids).toHaveLength(1);
    evidence.result = "LIVE_CONVERSATIONS_PASS";
  } finally {
    await page.screenshot({ path: `${root}/conversations.png`, animations: "disabled" });
    if (agentId) {
      const response = await context.request.delete(`/api/agents/${agentId}`);
      if (![204, 404].includes(response.status()))
        evidence.cleanup_agent = { id: agentId, status: response.status() };
    }
    writeFileSync(`${root}/conversations.json`, JSON.stringify(evidence, null, 2));
  }
});
