import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";

const agents = [
  {
    id: "agent-release",
    tenant_id: "tenant-1",
    name: "Release steward",
    active_revision: 3,
    latest_revision: 3,
    created_by: "actor-1",
    created_at_ms: 1_767_225_600_000,
  },
  {
    id: "agent-review",
    tenant_id: "tenant-1",
    name: "Change reviewer",
    active_revision: 1,
    latest_revision: 1,
    created_by: "actor-1",
    created_at_ms: 1_769_904_000_000,
  },
];
const publication = {
  publication_id: "pub-test-1",
  tenant_id: "tenant-1",
  owner_subject: "actor-1",
  profile_id: "profile-release",
  active_revision: 2,
  toolset_digest: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef",
  state: "active",
  created_at_ms: 1_788_260_000_000,
  updated_at_ms: 1_788_260_000_000,
};

const project = {
  id: "project-test",
  forge_instance_ref: "connection:gitlab:test",
  project_ref: "42",
  path_with_namespace: "foundation/devcenter",
  name: "devcenter",
  default_branch: "trunk",
  selected_branch: "trunk",
  pinned_commit: "0123456789abcdef0123456789abcdef01234567",
  web_url: "https://gitlab.example.test/foundation/devcenter",
};
const todoCatalog = {
  format: "service-catalog/1",
  service_ref: "service:todo",
  display_name: "Todo",
  description: "Shared scoped lists and intent-driven items.",
  semantic_catalog: {
    format: "ess-browser-catalog/1",
    entities: [
      {
        name: "todo.list.TodoList",
        display: "Todo list",
        initial: "active",
        transitions: [{ name: "archive", from: ["active"], to: "archived" }],
      },
    ],
    views: [
      {
        name: "todo.list.VisibleLists",
        display: "Visible lists",
        consistency: "read_your_writes",
        fields: [{ name: "list_id", wire: "list-id" }],
      },
    ],
  },
  authentication: { source: "session", realm_policy: "optional" },
  operations: [
    {
      name: "create_list",
      operation_ref: "todo.create_list",
      semantic_ref: "todo.list.CreateList",
      kind: "intent",
      effect: "write",
      input_schema: {
        type: "object",
        properties: {
          list_id: { type: "string", title: "List ID" },
          title: { type: "string", title: "Title" },
        },
        required: ["list_id", "title"],
        additionalProperties: false,
      },
      output_schema: { type: "object" },
    },
    {
      name: "list_visible_lists",
      operation_ref: "todo.list_visible_lists",
      semantic_ref: "todo.list.VisibleLists",
      kind: "query",
      effect: "read",
      input_schema: { type: "object", properties: {}, additionalProperties: false },
      output_schema: { type: "array" },
    },
  ],
};

async function mockAuthenticatedWorkspace(page: Page) {
  const capabilities = [
    {
      operation_ref: "git.project.list",
      title: "List GitLab projects",
      effect: "read_only",
      approval: "not_required",
      connections: [
        {
          connection_ref: "connection:gitlab:test",
          label: "My GitLab",
          provider: "gitlab",
          audiences: ["https://gitlab.example.test"],
        },
      ],
    },
    {
      operation_ref: "todo.list_visible_lists",
      title: "List visible Todo lists",
      effect: "read_only",
      approval: "not_required",
      connections: [
        {
          connection_ref: "connection:todo",
          label: "Todo",
          provider: "todo",
          audiences: [],
        },
      ],
    },
    {
      operation_ref: "todo.create_list",
      title: "Create Todo list",
      effect: "mutating",
      approval: "required",
      connections: [
        {
          connection_ref: "connection:todo",
          label: "Todo",
          provider: "todo",
          audiences: [],
        },
      ],
    },
  ];
  let capabilityProfile = {
    id: "profile-release",
    name: "Release profile",
    revision: 3,
    mappings: [
      {
        operation_ref: "todo.list_visible_lists",
        tool_name: "todo_list_visible_lists",
        connection_ref: "connection:todo",
        posture: "allow",
      },
      {
        operation_ref: "todo.create_list",
        tool_name: "todo_create_list",
        connection_ref: "connection:todo",
        posture: "approval_required",
      },
    ],
    created_at_ms: 1_788_260_000_000,
    updated_at_ms: 1_788_260_000_000,
  };
  await page.route(/^https?:\/\/[^/]+\/api\//, async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === "/api/session") {
      await route.fulfill({
        json: {
          tenant_id: "tenant-1",
          subject: "actor-1",
          email: "engineer@example.test",
          groups: ["engineers"],
          connectors_docs_available: false,
        },
      });
      return;
    }
    if (path === "/api/connectors/claude-code") {
      await route.fulfill({ json: { provider: "claude-code", connected: false } });
      return;
    }
    if (path === "/api/capabilities") {
      await route.fulfill({ json: capabilities });
      return;
    }
    if (path === "/api/capability-profiles" && request.method() === "GET") {
      await route.fulfill({ json: [capabilityProfile] });
      return;
    }
    if (path === "/api/capability-profiles/profile-release" && request.method() === "PATCH") {
      const submitted = request.postDataJSON() as {
        expected_revision: number;
        name: string;
        mappings: typeof capabilityProfile.mappings;
      };
      expect(submitted.expected_revision).toBe(capabilityProfile.revision);
      expect(submitted.name).toBe(capabilityProfile.name);
      capabilityProfile = {
        ...capabilityProfile,
        revision: capabilityProfile.revision + 1,
        mappings: submitted.mappings,
        updated_at_ms: capabilityProfile.updated_at_ms + 1,
      };
      await route.fulfill({ json: capabilityProfile });
      return;
    }
    if (path === "/api/services" && request.method() === "GET") {
      await route.fulfill({
        json: {
          services: [
            {
              service_ref: "service:todo",
              display_name: "Todo",
              description: todoCatalog.description,
              digest: "a".repeat(64),
            },
          ],
        },
      });
      return;
    }
    if (path === "/api/services/catalog" && request.method() === "POST") {
      expect(request.postDataJSON()).toEqual({ service_ref: "service:todo" });
      await route.fulfill({ json: todoCatalog });
      return;
    }
    if (path === "/api/services/invoke" && request.method() === "POST") {
      const submitted = request.postDataJSON() as {
        operation_ref: string;
        input: Record<string, unknown>;
        confirmed: boolean;
      };
      await route.fulfill({
        json: {
          output:
            submitted.operation_ref === "todo.list_visible_lists"
              ? [{ list_id: "release", title: "Release service console", state: "active" }]
              : { outcome: "applied", through_version: 1, replayed: false },
          connector_audit_ref: "audit:test",
        },
      });
      return;
    }
    if (path === "/api/connectors/catalog") {
      await route.fulfill({
        json: {
          providers: [
            {
              provider_ref: "gitlab",
              authority: "com.gitlab",
              vendor: "GitLab",
              description: "Projects, branches, merge requests, issues, and pipelines.",
              audiences: ["https://gitlab.example.test"],
              services: ["git", "issues", "ci"],
              operation_count: 24,
              configurable: true,
              setup_profiles: [{ auth_profile: "gitlab.oauth_user", actor: "person" }],
            },
          ],
          next_offset: null,
        },
      });
      return;
    }
    if (path === "/api/connectors/catalog/gitlab") {
      await route.fulfill({
        json: {
          provider: {
            provider_ref: "gitlab",
            authority: "com.gitlab",
            vendor: "GitLab",
            description: "Projects, branches, merge requests, issues, and pipelines.",
            audiences: ["https://gitlab.example.test"],
            services: ["git", "issues", "ci"],
            operation_count: 24,
            configurable: true,
            setup_profiles: [{ auth_profile: "gitlab.oauth_user", actor: "person" }],
          },
          operations: [
            {
              operation_ref: "git.project.list",
              service: "git",
              description: "List projects visible to the connected person.",
              risk: "read_only",
              exposed: true,
            },
          ],
        },
      });
      return;
    }
    if (path === "/api/connections") {
      await route.fulfill({ json: [] });
      return;
    }
    if (path === "/api/agents" && request.method() === "GET") {
      await route.fulfill({ json: agents });
      return;
    }
    if (path === "/api/repositories") {
      await route.fulfill({
        json: [
          {
            forge_instance_ref: project.forge_instance_ref,
            project_ref: project.project_ref,
            path_with_namespace: project.path_with_namespace,
            name: project.name,
            default_branch: "trunk",
            visibility: "private",
            web_url: project.web_url,
            opened_project_id: null,
          },
        ],
      });
      return;
    }
    if (path === "/api/projects" && request.method() === "POST") {
      expect(request.postDataJSON()).toEqual({
        forge_instance_ref: project.forge_instance_ref,
        project_ref: project.project_ref,
      });
      await route.fulfill({ json: project, status: 201 });
      return;
    }
    if (path === `/api/projects/${project.id}`) {
      await route.fulfill({ json: project });
      return;
    }
    if (path === `/api/projects/${project.id}/branches`) {
      await route.fulfill({
        json: [
          {
            name: "trunk",
            commit: project.pinned_commit,
            provider_default: true,
            protected: true,
          },
        ],
      });
      return;
    }
    if (path === `/api/projects/${project.id}/tree`) {
      await route.fulfill({
        json: [
          { object_id: "tree-src", name: "src", path: "src", kind: "tree", mode: "040000" },
          {
            object_id: "blob-readme",
            name: "README.md",
            path: "README.md",
            kind: "blob",
            mode: "100644",
          },
        ],
      });
      return;
    }
    if (path === `/api/projects/${project.id}/engineering-artifacts`) {
      await route.fulfill({
        json: {
          artifacts: [
            {
              id: "artifact-project-boundary",
              locator: "ep://foundation/devcenter/design/repository-workspace-boundary",
              entity_type: "aep.design/v1",
              revision: 3,
              title: "Repository workspace boundary",
              status: "draft",
              updated_at_ms: 1_788_260_000_000,
              source_revision: project.pinned_commit,
            },
          ],
          has_more: false,
        },
      });
      return;
    }
    if (path === `/api/projects/${project.id}/threads`) {
      await route.fulfill({ json: [] });
      return;
    }
    if (path === `/api/projects/${project.id}/workflows`) {
      await route.fulfill({
        json: [
          {
            id: "review.code/v1",
            name: "Code review",
            description: "Commit-pinned findings.",
          },
        ],
      });
      return;
    }
    if (path === "/api/agents" && request.method() === "POST") {
      const submitted = request.postDataJSON() as { name: string };
      await route.fulfill({ json: { ...agents[0], id: "agent-new", name: submitted.name } });
      return;
    }
    if (path === "/api/mcp/publications" && request.method() === "GET") {
      await route.fulfill({ json: [publication] });
      return;
    }
    if (path === "/api/mcp/publications/pub-test-1" && request.method() === "PATCH") {
      const submitted = request.postDataJSON() as { state: string };
      await route.fulfill({ json: { ...publication, state: submitted.state } });
      return;
    }
    if (path === "/api/mcp/publications/pub-test-1/clients") {
      await route.fulfill({ json: [] });
      return;
    }
    if (path === "/api/mcp/publications/pub-test-1/approvals") {
      await route.fulfill({ json: [] });
      return;
    }
    await route.fulfill({ status: 404, json: { code: "not_found" } });
  });
}

test("renders a signed-out authority path instead of an empty shell", async ({ page }) => {
  await page.route("**/api/session", (route) =>
    route.fulfill({ status: 401, json: { code: "identity_authentication_required" } }),
  );
  await page.goto("/");

  await expect(page.getByRole("heading", { name: /Direct the work/ })).toBeVisible();
  await expect(page.getByRole("link", { name: /Sign in through Identity/ })).toBeVisible();
  await expect(page.getByText("Connectors holds access")).toBeVisible();
  await page.getByRole("link", { name: /Documentation/ }).click();
  await expect(page.getByRole("heading", { name: "Devcenter, by contract" })).toBeVisible();
});

test("opens a deep-linked agent and creates a governed worker", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop workspace behavior");
  await mockAuthenticatedWorkspace(page);
  await page.goto("/agents/agent-review");

  await expect(page.getByRole("heading", { name: "Change reviewer" })).toBeVisible();
  await page.getByRole("button", { name: "New agent" }).click();
  await expect(page.getByRole("dialog")).toBeVisible();
  await page.getByLabel("Name").fill("Evidence keeper");
  await page
    .getByLabel("Instructions", { exact: true })
    .fill("Collect gate evidence and stop before publication.");
  await page.getByRole("button", { name: "Create and activate" }).click();

  await expect(page.getByRole("heading", { name: "Evidence keeper" })).toBeVisible();
  await expect(page.getByRole("status").filter({ hasText: "created and activated" })).toBeVisible();
  const accessibility = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();
  expect(accessibility.violations).toEqual([]);
});

test("runs the SDK-generated Todo console through the live BFF binding", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop generated-service behavior");
  await mockAuthenticatedWorkspace(page);
  await page.goto("/services");

  await expect(page.getByRole("heading", { name: "Services" })).toBeVisible();
  await expect(page.getByRole("heading", { name: "Todo", exact: true })).toBeVisible();
  await page.getByRole("button", { name: /list_visible_lists/ }).click();
  await page.getByRole("button", { name: "Run query" }).click();
  await expect(page.getByText("Release service console")).toBeVisible();

  await page.getByRole("button", { name: /create_list/ }).click();
  await page.getByLabel("List ID *").fill("service-console");
  await page.getByLabel("Title *").fill("Ship the generated console");
  await page.getByLabel("Confirm this state-changing intent").check();
  await page.getByRole("button", { name: "Send intent" }).click();
  await expect(page.getByText(/through_version/).first()).toBeVisible();
});

test("reviews and approves only the exact suspended agent call", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop approval behavior");
  await mockAuthenticatedWorkspace(page);
  const pending = {
    id: "approval-1",
    task_id: "task-1",
    attempt_id: "attempt-1",
    call_id: "call-1",
    tool_name: "todo_item_create",
    operation_ref: "todo.item.create",
    connection_ref: "todo",
    description_ref: "description-1",
    input: { list_id: "release", title: "Publish the release" },
    context: {
      tenant_id: "tenant-1",
      agent_id: "agent-release",
      agent_revision: 3,
      authority_snapshot_id: "request-1",
      authority_snapshot_sha256: "a".repeat(64),
    },
    requested_at_ms: 1_788_260_000_000,
  };
  let submittedDecision: unknown;
  await page.route("**/api/agents/agent-release/tasks", (route) =>
    route.fulfill({
      status: 202,
      json: {
        id: "task-1",
        agent_id: "agent-release",
        status: "accepted",
        attempt_id: "attempt-1",
      },
    }),
  );
  await page.route("**/api/tasks/task-1/events", (route) =>
    route.fulfill({
      contentType: "text/event-stream",
      body: `event: task\ndata: ${JSON.stringify({
        event: {
          kind: "approval_requested",
          approval_id: pending.id,
          call_id: pending.call_id,
          operation_ref: pending.operation_ref,
          connection_ref: pending.connection_ref,
        },
      })}\n\n`,
    }),
  );
  await page.route("**/api/tasks/task-1/approvals", (route) => route.fulfill({ json: [pending] }));
  await page.route("**/api/tasks/task-1/approvals/approval-1", async (route) => {
    submittedDecision = route.request().postDataJSON();
    await route.fulfill({ json: pending });
  });

  await page.goto("/agents/agent-release");
  await page.getByLabel("Task instructions").fill("Create the release Todo.");
  await page.getByRole("button", { name: "Run task" }).click();
  await expect(page.getByRole("heading", { name: "Human decision required" })).toBeVisible();
  await expect(page.getByText("todo.item.create")).toBeVisible();
  await expect(page.locator(".task-approval pre")).toContainText("Publish the release");
  await page.getByRole("button", { name: "Approve exact call" }).click();
  await expect.poll(() => submittedDecision).toEqual({ decision: "approve" });
});

test("opens a visible repository as a commit-pinned project", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop project behavior");
  await mockAuthenticatedWorkspace(page);
  await page.goto("/projects");

  await expect(page.getByRole("heading", { name: "Open a project" })).toBeVisible();
  const searched = page.waitForRequest((request) => {
    const url = new URL(request.url());
    return url.pathname === "/api/repositories" && url.searchParams.get("query") === "devcenter";
  });
  await page.getByRole("searchbox", { name: "Search repositories" }).fill("devcenter");
  await searched;
  await page.getByRole("button", { name: /foundation\/devcenter/ }).click();
  await expect(page.getByRole("heading", { name: "foundation/devcenter" })).toBeVisible();
  await expect(page.getByText("0123456789", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Refresh snapshot" })).toBeVisible();
  await page.getByRole("button", { name: "files" }).click();
  await expect(page.getByText("README.md", { exact: true })).toBeVisible();
  await expect(page.getByText(/separate materialization step/)).toBeVisible();
  await page.getByRole("button", { name: "aep" }).click();
  await expect(page.getByRole("heading", { name: "Repository workspace boundary" })).toBeVisible();
  await expect(
    page
      .locator(".aep-artifact-card")
      .getByText(project.pinned_commit.slice(0, 10), { exact: true }),
  ).toBeVisible();
  const accessibility = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();
  expect(accessibility.violations).toEqual([]);
});

test("keeps catalog and connection custody usable on a mobile viewport", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "mobile-chromium", "Mobile navigation behavior");
  await mockAuthenticatedWorkspace(page);
  await page.goto("/agents");

  await page.getByRole("button", { name: "Open navigation" }).click();
  await page.getByRole("link", { name: "Connectors" }).click();
  await expect(page.getByRole("heading", { name: "Connectors" })).toBeVisible();
  await expect(page.getByRole("button", { name: "My connectors" })).toHaveAttribute(
    "aria-current",
    "page",
  );
  await expect(page.getByRole("heading", { name: "Model access" })).toBeVisible();
  await page.getByRole("button", { name: "Catalog" }).click();
  await expect(page.getByRole("heading", { name: "GitLab" })).toBeVisible();
  await page.getByRole("link", { name: /GitLab/ }).click();
  await expect(page.getByText("git.project.list", { exact: true })).toBeVisible();
  await page.getByRole("button", { name: "My connectors" }).click();
  await expect(page.getByRole("heading", { name: "Model access" })).toBeVisible();
  await expect(page.getByText("Credential bytes", { exact: true })).toBeVisible();
  await expect(page.getByRole("button", { name: "Connect Claude" })).toBeVisible();
});

test("makes capability posture explicit and applies bulk changes atomically", async ({ page }) => {
  await mockAuthenticatedWorkspace(page);
  await page.goto("/profiles");

  await expect(page.getByRole("heading", { name: "Release profile" })).toBeVisible();
  await expect(page.getByText("1 allowed", { exact: true })).toBeVisible();
  await expect(page.getByText("1 approval", { exact: true })).toBeVisible();
  await expect(page.getByText("1 denied", { exact: true })).toBeVisible();

  const gitLabCapability = page.locator(".capability-row").filter({
    hasText: "List GitLab projects",
  });
  await expect(gitLabCapability.getByRole("button", { name: "Deny" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );

  await page.getByRole("button", { name: "Allow all" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "All 3 capabilities are now allowed." }),
  ).toBeVisible();
  await expect(page.getByText("3 allowed", { exact: true })).toBeVisible();
  await expect(gitLabCapability.getByRole("button", { name: "Allow" })).toHaveAttribute(
    "aria-pressed",
    "true",
  );

  await page.getByRole("button", { name: "Deny all" }).click();
  await expect(
    page.getByRole("status").filter({ hasText: "All 3 capabilities are now denied." }),
  ).toBeVisible();
  await expect(page.getByText("3 denied", { exact: true })).toBeVisible();
  const accessibility = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();
  expect(accessibility.violations).toEqual([]);
});

test("shows one stable MCP endpoint and separate client setup commands", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop publication behavior");
  await mockAuthenticatedWorkspace(page);
  await page.goto("/publications");

  await expect(page.getByRole("heading", { name: "Publish governed tools" })).toBeVisible();
  await expect(page.locator(".endpoint-row code")).toHaveText(/\/mcp\/pub-test-1$/);
  await expect(page.getByText(/codex mcp add devcenter/)).toBeVisible();
  await expect(page.getByText(/claude mcp add --transport http/)).toBeVisible();
  await expect(page.getByText("Browser logout does not revoke it.")).toBeVisible();
  await page.getByRole("button", { name: "Revoke permanently" }).click();
  await expect(page.getByRole("status").filter({ hasText: "Publication revoked." })).toBeVisible();
  const accessibility = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();
  expect(accessibility.violations).toEqual([]);
});
