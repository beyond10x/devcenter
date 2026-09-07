import { test, expect } from "@playwright/test";
import { chmodSync, writeFileSync } from "node:fs";
import { z } from "zod";
import { provisionCredentials } from "./provision";

test("normal Identity login and Connector custody", async ({ page, context }) => {
  process.umask(0o077);
  const root = z.string().parse(process.env.DEVCENTER_EVIDENCE_ROOT);
  const origin = z.url().parse(process.env.DEVCENTER_ORIGIN);
  const observed: Array<{ path: string; status: number }> = [];
  page.on("response", (response) => {
    const path = new URL(response.url()).pathname;
    observed.push({ path, status: response.status() });
    writeFileSync(`${root}/setup-responses.json`, JSON.stringify(observed, null, 2));
  });
  await page.goto("/");
  await page.getByRole("link", { name: /Sign in through/ }).click();
  await page.waitForURL(
    (url) =>
      url.origin === origin &&
      !url.pathname.startsWith("/auth/") &&
      !url.pathname.startsWith("/oauth/"),
    {
      timeout: 60_000,
    },
  );
  await expect
    .poll(async () => (await context.request.get(`${origin}/api/session`)).status())
    .toBe(200);
  const storage = `${root}/storage-state.json`;
  await context.storageState({ path: storage });
  chmodSync(storage, 0o600);
  const cookie = (await context.cookies(origin)).find((entry) =>
    entry.value.startsWith("identity_session_v1_"),
  );
  if (!cookie) throw new Error("Identity-issued browser session missing");
  const tokenResponse = await context.request.post(`${origin}/v1/access-token`, {
    headers: { Authorization: `Bearer ${cookie.value}`, Origin: origin },
    data: {
      audience: "urn:b10x:connectors",
      scope:
        "connectors.integrations.manage connectors.connections.self connectors.catalog.read connectors.invoke connectors.credentials.lease",
    },
  });
  expect(tokenResponse.status()).toBe(200);
  const access = z.object({ access_token: z.string() }).parse(await tokenResponse.json());
  const headers = { Authorization: `Bearer ${access.access_token}`, Origin: origin };
  await provisionCredentials(context.request, origin, headers);
  const model = await context.request.put(
    `${origin}/api/connectors/v1/subscription-credentials/claude-code`,
    {
      headers,
      data: { credential: "local-model-fixture-credential" },
    },
  );
  expect(model.ok()).toBe(true);
  const agentsResponse = await context.request.get(`${origin}/api/agents`);
  expect(agentsResponse.ok()).toBe(true);
  const agents = z
    .array(z.object({ id: z.string(), name: z.string() }))
    .parse(await agentsResponse.json());
  if (!agents.some((agent) => agent.name === "Local acceptance agent")) {
    await page.goto("/agents");
    await page.getByRole("button", { name: "New agent", exact: true }).first().click();
    await page.getByLabel("Name", { exact: true }).fill("Local acceptance agent");
    await page
      .getByLabel("Instructions", { exact: true })
      .fill("Answer local acceptance diagnostics. Do not modify files unless explicitly asked.");
    await page.getByRole("button", { name: "Create and activate", exact: true }).click();
    await expect(
      page.getByRole("heading", { name: "Local acceptance agent", exact: true }),
    ).toBeVisible();
  }
  const candidatesSchema = z.array(
    z.object({
      project_ref: z.string(),
      forge_instance_ref: z.string(),
      opened_project_id: z.string().nullable().optional(),
    }),
  );
  async function repositories() {
    const response = await context.request.get(`${origin}/api/repositories`);
    expect(response.status()).toBe(200);
    return candidatesSchema.parse(await response.json());
  }
  let candidates = await repositories();
  if (!candidates.some((entry) => entry.project_ref === "1")) {
    const connection = await context.request.post(`${origin}/api/connections`, {
      headers: { Origin: origin },
      data: {
        integration_ref: "gitlab",
        label: "Local fixture repository",
        auth_profile: "gitlab.oauth_user",
      },
    });
    expect(connection.ok()).toBe(true);
    const connect = z
      .object({ browser_completion_url: z.url(), connect_session_ref: z.string() })
      .parse(await connection.json());
    const authorization = connect.browser_completion_url;
    if (!authorization) throw new Error("Connector authorization URL missing");
    await page.goto(authorization);
    await page.getByRole("link", { name: "Continue to GitLab" }).click();
    await expect
      .poll(
        async () => {
          const response = await context.request.get(
            `${origin}/api/connect-sessions/${encodeURIComponent(connect.connect_session_ref)}`,
          );
          const status = z.object({ state: z.string() }).parse(await response.json());
          return status.state;
        },
        { timeout: 30_000 },
      )
      .toBe("completed");
    candidates = await repositories();
  }
  const candidate = candidates.find((entry) => entry.project_ref === "1");
  if (!candidate) throw new Error("local fixture repository missing");
  const project = candidate.opened_project_id
    ? await context.request.get(`${origin}/api/projects/${candidate.opened_project_id}`)
    : await context.request.post(`${origin}/api/projects`, {
        headers: { Origin: origin },
        data: {
          forge_instance_ref: candidate.forge_instance_ref,
          project_ref: candidate.project_ref,
        },
      });
  expect(project.ok()).toBe(true);
  const opened = z
    .object({ id: z.string() })
    .loose()
    .parse(await project.json());
  if (process.env.DEVCENTER_EXPECTED_PROJECT_ID)
    expect(opened.id).toBe(process.env.DEVCENTER_EXPECTED_PROJECT_ID);
  writeFileSync(`${root}/project.json`, JSON.stringify(opened, null, 2));
  await context.storageState({ path: storage });
  chmodSync(storage, 0o600);
});
