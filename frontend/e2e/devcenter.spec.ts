import AxeBuilder from "@axe-core/playwright";
import { expect, test, type Page } from "@playwright/test";
import { Buffer } from "node:buffer";

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
const codingSession = {
  id: "session-test",
  project_id: project.id,
  source_revision: project.pinned_commit,
  base_materialization_ref: "substrate:base:test",
  working_materialization_ref: "substrate:working:test",
  manifest_sha256: "a".repeat(64),
  state: "ready",
  failure_code: null,
  limits: { max_files: 1000, max_total_bytes: 268_435_456, max_file_bytes: 184_320 },
  created_at_ms: 1_788_260_000_000,
  updated_at_ms: 1_788_260_000_000,
};
const terminalProfile = {
  id: "rust-stable-confined",
  label: "Rust stable · confined",
  runtime_ref: "substrate:image:rust-stable-test",
  shell: "/bin/bash",
  arguments: ["--noprofile", "--norc"],
  working_directory: "/workspace",
  environment: { TERM: "xterm-256color", COLORTERM: "truecolor" },
  workspace_access: "read_write",
  network: "none",
  limits: {
    timeout_ms: 3_600_000,
    cpu_millis: 2_000,
    memory_bytes: 536_870_912,
    processes: 128,
    output_bytes: 4_194_304,
    input_bytes: 65_536,
    frame_bytes: 65_536,
    queued_frames: 256,
    lease_ttl_ms: 3_600_000,
  },
} as const;
const terminalSession = {
  id: "terminal-test",
  coding_session_id: codingSession.id,
  agentide_session_id: codingSession.id,
  authority_grant_id: "grant-terminal-test",
  profile: terminalProfile,
  actor: "actor-1",
  process_id: "substrate-process-test",
  state: "running",
  exit: null,
  failure_code: null,
  created_at_ms: 1_788_260_100_000,
  updated_at_ms: 1_788_260_100_000,
} as const;
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

async function mockAuthenticatedWorkspace(
  page: Page,
  options: {
    agentideWorkspace?: boolean;
    terminalProfile?: boolean;
    staleTerminal?: boolean;
  } = {},
) {
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
    audience: "personal",
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
    created_by: "actor-1",
    created_at_ms: 1_788_260_000_000,
    updated_at_ms: 1_788_260_000_000,
  };
  let agentIdeBound = false;
  let agentIdeVersion = 1;
  const agentIdeGrants: Array<Record<string, unknown>> = [];
  const agentIdePins: Array<Record<string, unknown>> = [];
  const coordinationView = () => ({
    summary: {
      state: "ready",
      through_version: agentIdeVersion,
      failure_code: null,
      retryable: false,
    },
    session: {
      session_id: codingSession.id,
      workspace_session_id: codingSession.id,
      project_id: project.id,
      source_revision: codingSession.source_revision,
      manifest_digest: codingSession.manifest_sha256,
      objective: "Work on foundation/devcenter",
      state: "Active",
    },
    grants: agentIdeGrants,
    pins: agentIdePins,
    checkpoints: [],
  });
  let currentTerminal: Record<string, unknown> = { ...terminalSession };
  let terminalInventoryReads = 0;
  const codingTasks: Array<Record<string, unknown>> = [];
  let workflowRun: Record<string, unknown> | undefined;
  let workflowPolls = 0;
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
          agentide_workspace_enabled: options.agentideWorkspace ?? false,
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
      if (submitted.operation_ref === "agentide.list_sessions") {
        await route.fulfill({
          json: {
            output: {
              items: agentIdeBound
                ? [
                    {
                      session_id: "agentide-session-test",
                      workspace_session_id: codingSession.id,
                      project_id: project.id,
                      source_revision: codingSession.source_revision,
                      manifest_digest: codingSession.manifest_sha256,
                      objective: "Work on foundation/devcenter",
                    },
                  ]
                : [],
              next_cursor: null,
              partial: false,
            },
            connector_audit_ref: "audit:agentide:sessions",
          },
        });
        return;
      }
      if (
        submitted.operation_ref === "agentide.list_grants" ||
        submitted.operation_ref === "agentide.list_context_pins" ||
        submitted.operation_ref === "agentide.list_approval_checkpoints"
      ) {
        await route.fulfill({
          json: {
            output: {
              items:
                submitted.operation_ref === "agentide.list_grants"
                  ? agentIdeGrants
                  : submitted.operation_ref === "agentide.list_context_pins"
                    ? agentIdePins
                    : [],
              next_cursor: null,
              partial: false,
            },
            connector_audit_ref: `audit:${submitted.operation_ref}`,
          },
        });
        return;
      }
      if (submitted.operation_ref === "agentide.start_session" && submitted.confirmed) {
        agentIdeBound = true;
        agentIdeVersion = 1;
        await route.fulfill({
          json: {
            output: {
              outcome: "started",
              events: [
                {
                  name: "agentide.session.SessionStarted",
                  fields: { session_id: "agentide-session-test" },
                },
              ],
              through_version: agentIdeVersion,
              replayed: false,
            },
            connector_audit_ref: "audit:agentide:start",
          },
        });
        return;
      }
      if (submitted.operation_ref === "agentide.create_grant" && submitted.confirmed) {
        agentIdeVersion += 1;
        const grantId = "grant-test";
        agentIdeGrants.push({
          grant_id: grantId,
          grantee: submitted.input.grantee,
          allowed_intents: submitted.input.allowed_intents,
          path_prefixes: submitted.input.path_prefixes,
          maximum_risk: submitted.input.maximum_risk,
          expires_at: null,
          revision: 1,
          state: "Active",
        });
        await route.fulfill({
          json: {
            output: {
              outcome: "created",
              events: [
                { name: "agentide.coordination.GrantCreated", fields: { grant_id: grantId } },
              ],
              through_version: agentIdeVersion,
              replayed: false,
            },
            connector_audit_ref: "audit:agentide:grant",
          },
        });
        return;
      }
      if (submitted.operation_ref === "agentide.pin_context" && submitted.confirmed) {
        agentIdeVersion += 1;
        agentIdePins.push({
          pin_id: "pin-test",
          kind: submitted.input.kind,
          reference: submitted.input.reference,
          start_line: submitted.input.start_line,
          end_line: submitted.input.end_line,
          sha256: submitted.input.sha256,
          state: "Active",
        });
        await route.fulfill({
          json: {
            output: {
              outcome: "pinned",
              events: [{ name: "agentide.coordination.ContextPinned", fields: {} }],
              through_version: agentIdeVersion,
              replayed: false,
            },
            connector_audit_ref: "audit:agentide:pin",
          },
        });
        return;
      }
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
    const agentTasksMatch = path.match(/^\/api\/agents\/([^/]+)\/tasks$/);
    if (agentTasksMatch && request.method() === "GET") {
      await route.fulfill({
        json: codingTasks.filter((task) => task.agent_id === agentTasksMatch[1]),
      });
      return;
    }
    const codingTurnMatch = path.match(
      /^\/api\/project-sessions\/([^/]+)\/agents\/([^/]+)\/turns$/,
    );
    if (codingTurnMatch && request.method() === "POST") {
      const submitted = request.postDataJSON() as {
        prompt: string;
        focused_selections: Array<Record<string, unknown>>;
        open_files: Array<Record<string, unknown>>;
      };
      expect(codingTurnMatch[1]).toBe(codingSession.id);
      expect(submitted).not.toHaveProperty("agentide_session_id");
      expect(submitted.focused_selections[0]).toMatchObject({
        kind: "diff_hunk",
        truncated: false,
      });
      expect(submitted.open_files[0]).toMatchObject({
        path: "src/main.rs",
        sha256: "b".repeat(64),
        dirty: false,
      });
      expect(submitted.open_files[0]).not.toHaveProperty("content");
      const task = {
        id: "task-coding-test",
        agent_id: codingTurnMatch[2],
        status: "accepted",
        attempt_id: "attempt-coding-test",
        prompt: submitted.prompt,
        output: null,
        failure_code: null,
        failure_message: null,
        accepted_at_ms: Date.now(),
        completed_at_ms: null,
        workspace_session_id: codingSession.id,
        agentide_session_id: codingSession.id,
      };
      codingTasks.push(task);
      await route.fulfill({ status: 202, json: task });
      return;
    }
    if (/^\/api\/tasks\/[^/]+\/events$/.test(path) && request.method() === "GET") {
      const events = [
        { event: { kind: "accepted" } },
        { event: { kind: "running" } },
        { event: { kind: "context_changed", revision: "context-test-2" } },
        {
          event: {
            kind: "inventory_changed",
            revision: "inventory-test-2",
            published_tools: ["code_read", "code_changes", "code_edit"],
          },
        },
        { event: { kind: "text_delta", text: "Inspecting the saved workspace…" } },
        {
          event: {
            kind: "succeeded",
            output: "The saved Workspace content was inspected with the current AgentIDE tools.",
          },
        },
      ];
      await route.fulfill({
        status: 200,
        contentType: "text/event-stream",
        body: events.map((event) => `event: task\ndata: ${JSON.stringify(event)}\n\n`).join(""),
      });
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
    if (path === `/api/projects/${project.id}/workflow-runs`) {
      if (request.method() === "POST") {
        const submitted = request.postDataJSON() as {
          definition_id: string;
          branch: string;
          commit: string;
          idempotency_key: string;
        };
        expect(submitted).toMatchObject({
          definition_id: "review.code/v1",
          branch: project.selected_branch,
          commit: project.pinned_commit,
        });
        expect(submitted.idempotency_key).not.toBe("");
        workflowRun = {
          id: "workflow-run-test",
          definition_id: submitted.definition_id,
          project_id: project.id,
          branch: submitted.branch,
          commit: submitted.commit,
          state: "accepted",
          failure_code: null,
          output: null,
          created_at_ms: 1_788_260_000_000,
        };
        workflowPolls = 0;
        await route.fulfill({ status: 200, json: workflowRun });
        return;
      }
      if (!workflowRun) {
        await route.fulfill({ json: [] });
        return;
      }
      workflowPolls += 1;
      workflowRun = {
        ...workflowRun,
        state: workflowPolls === 1 ? "running" : "succeeded",
        output:
          workflowPolls === 1
            ? null
            : "## Review complete\n\n**No blockers.** The pinned snapshot is ready.",
      };
      await route.fulfill({ json: [workflowRun] });
      return;
    }
    if (path === `/api/projects/${project.id}/sessions`) {
      await route.fulfill({ json: [codingSession] });
      return;
    }
    if (
      path === `/api/project-sessions/${codingSession.id}/resume` &&
      request.method() === "POST"
    ) {
      agentIdeBound = true;
      await route.fulfill({
        json: { ...codingSession, coordination: coordinationView().summary },
      });
      return;
    }
    if (path === `/api/project-sessions/${codingSession.id}/coordination`) {
      await route.fulfill({ json: coordinationView() });
      return;
    }
    if (
      path === `/api/project-sessions/${codingSession.id}/coordination/pins` &&
      request.method() === "POST"
    ) {
      const submitted = request.postDataJSON() as Record<string, unknown>;
      agentIdeVersion += 1;
      agentIdePins.push({
        pin_id: "pin-test",
        kind: submitted.kind,
        reference: submitted.reference,
        start_line: submitted.start_line,
        end_line: submitted.end_line,
        sha256: submitted.sha256,
        state: "Active",
      });
      await route.fulfill({ json: coordinationView() });
      return;
    }
    if (
      path === `/api/project-sessions/${codingSession.id}/coordination/grants` &&
      request.method() === "POST"
    ) {
      const submitted = request.postDataJSON() as { grantee: string };
      agentIdeVersion += 1;
      agentIdeGrants.push({
        grant_id: "grant-test",
        grantee: submitted.grantee,
        allowed_intents: ["code_edit", "code_create", "code_delete", "code_rename"],
        path_prefixes: [""],
        maximum_risk: "Medium",
        expires_at: null,
        revision: 1,
        state: "Active",
      });
      await route.fulfill({ json: coordinationView() });
      return;
    }
    if (path === `/api/project-sessions/${codingSession.id}`) {
      await route.fulfill({ json: codingSession });
      return;
    }
    if (path === `/api/project-sessions/${codingSession.id}/tree`) {
      await route.fulfill({
        json: {
          format: "workspace.tree/1",
          entries: [
            { path: "src", kind: "directory", size: null, sha256: null },
            { path: "src/main.rs", kind: "file", size: 32, sha256: "b".repeat(64) },
          ],
          truncated: true,
          omitted: 3,
        },
      });
      return;
    }
    if (path === `/api/project-sessions/${codingSession.id}/files/src/main.rs`) {
      await route.fulfill({
        json: {
          format: "workspace.file/1",
          revision: {
            path: "src/main.rs",
            sha256: "b".repeat(64),
            size: 32,
            language: "rust",
            modification: "modified",
          },
          content: 'fn main() {\n    println!("hello");\n}\n',
          binary: false,
          truncated: false,
        },
      });
      return;
    }
    if (path === `/api/project-sessions/${codingSession.id}/diff`) {
      const submitted = request.postDataJSON() as { mode: "patch" | "stat" | "files_only" };
      await route.fulfill({
        json: {
          format: "workspace.diff/1",
          selector: { kind: "workspace" },
          mode: submitted.mode,
          digest: "c".repeat(64),
          source_revision: codingSession.source_revision,
          files: [
            {
              old_path: "src/main.rs",
              new_path: "src/main.rs",
              status: "modified",
              additions: 1,
              deletions: 1,
              old_sha256: "d".repeat(64),
              new_sha256: "b".repeat(64),
              attribution: ["workspace"],
              hunks:
                submitted.mode === "patch"
                  ? [
                      {
                        id: "hunk-test",
                        old: { start: 1, lines: 1 },
                        new: { start: 1, lines: 1 },
                        heading: null,
                        lines: [
                          { kind: "deletion", old_line: 1, new_line: null, content: "old" },
                          { kind: "addition", old_line: null, new_line: 1, content: "new" },
                        ],
                      },
                    ]
                  : [],
            },
          ],
          additions: 1,
          deletions: 1,
          partial: false,
        },
      });
      return;
    }
    if (path === `/api/project-sessions/${codingSession.id}/terminal-profiles`) {
      await route.fulfill({ json: options.terminalProfile ? [terminalProfile] : [] });
      return;
    }
    if (path === `/api/project-sessions/${codingSession.id}/terminals`) {
      terminalInventoryReads += 1;
      await route.fulfill({
        json:
          options.terminalProfile && (!options.staleTerminal || terminalInventoryReads === 1)
            ? [currentTerminal]
            : [],
      });
      return;
    }
    if (path === `/api/project-terminals/${terminalSession.id}` && request.method() === "DELETE") {
      currentTerminal = {
        ...currentTerminal,
        state: "terminated",
        exit: { code: null, signal: "SIGKILL" },
        updated_at_ms: terminalSession.updated_at_ms + 1,
      };
      await route.fulfill({ json: currentTerminal });
      return;
    }
    if (path === `/api/project-terminals/${terminalSession.id}`) {
      if (options.staleTerminal) {
        await route.fulfill({
          status: 404,
          json: { code: "workspace_terminal_not_found" },
        });
        return;
      }
      await route.fulfill({ json: currentTerminal });
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
  const agentPicker = page.getByLabel("Current agent");
  await expect(agentPicker).toBeVisible();
  await expect(agentPicker).toHaveValue("agent-review");
  await expect(page.locator(".agent-chat-workspace > .task-workspace")).toHaveCount(1);
  await agentPicker.selectOption("agent-release");
  await expect(page).toHaveURL(/\/agents\/agent-release$/);
  await expect(page.getByRole("heading", { name: "Release steward" })).toBeVisible();
  await expect(agentPicker).toHaveValue("agent-release");
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
  await page.route("**/api/agents/agent-release/tasks", (route) => {
    if (route.request().method() === "GET") return route.fulfill({ json: [] });
    return route.fulfill({
      status: 202,
      json: {
        id: "task-1",
        agent_id: "agent-release",
        status: "accepted",
        attempt_id: "attempt-1",
        prompt: "Create the release Todo.",
        accepted_at_ms: 1_788_260_000_000,
      },
    });
  });
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
  await page.getByLabel("Message Release steward").fill("Create the release Todo.");
  await page.getByRole("button", { name: "Send" }).click();
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

test("advances an accepted workflow and preserves its rendered report", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop project workflow behavior");
  await mockAuthenticatedWorkspace(page);
  await page.goto(`/projects/${project.id}`);

  await page.getByRole("button", { name: "workflows" }).click();
  await page.getByRole("button", { name: /Run at 0123456789/ }).click();
  const run = page.locator(".workflow-run").filter({ hasText: "review.code/v1" });
  await expect(run.getByText("running", { exact: true })).toBeVisible();
  await expect(run.getByText("succeeded", { exact: true })).toBeVisible();
  await expect(run.getByRole("heading", { name: "Review complete" })).toBeVisible();
  await expect(run.getByText("No blockers.", { exact: true })).toBeVisible();

  await page.reload();
  await page.getByRole("button", { name: "workflows" }).click();
  await expect(page.getByRole("heading", { name: "Review complete" })).toBeVisible();
});

test("restores the native coding workbench from URL-backed state", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop hosted-workbench behavior");
  await mockAuthenticatedWorkspace(page, { agentideWorkspace: true });
  await page.goto(
    `/projects/${project.id}/sessions/${codingSession.id}?pane=diff&mode=patch&layout=side_by_side`,
  );

  await expect(page.getByRole("link", { name: project.path_with_namespace })).toBeVisible();
  await expect(page.getByRole("treeitem", { name: /main.rs/ })).toBeVisible();
  await expect(page.getByText("3 entries omitted.")).toBeVisible();
  await expect(page.getByRole("button", { name: "Split" })).toHaveClass(/active/);
  await expect(page.getByText("new", { exact: true })).toBeVisible();
  await expect(page.getByText("AgentIDE ready", { exact: true })).toBeVisible();
  await expect(page).not.toHaveURL(/agentide=/);
  await page.getByRole("button", { name: "Terminal", exact: true }).click();
  await expect(page.getByText("Interactive terminal refused")).toBeVisible();

  await page.getByRole("button", { name: "Attach hunk" }).click();
  await page.getByRole("button", { name: "Pin for session" }).click();
  await expect(page.getByText("DiffHunk", { exact: true })).toBeVisible();

  await page.getByRole("button", { name: "agents", exact: true }).click();
  await page.getByRole("button", { name: "Grant coding edits" }).first().click();
  await page.getByRole("button", { name: "grants", exact: true }).click();
  await expect(page.getByText("code_edit, code_create, code_delete, code_rename")).toBeVisible();

  await page.getByRole("button", { name: /Diff/ }).click();
  await page.getByRole("button", { name: "Attach hunk" }).click();
  await page.getByRole("button", { name: /Agent/ }).click();
  await page.getByLabel("Coding agent prompt").fill("Review and improve the saved change.");
  await page.getByRole("button", { name: "Send turn" }).click();
  await expect(page.getByText("Review and improve the saved change.")).toBeVisible();
  await expect(
    page.getByText("The saved Workspace content was inspected with the current AgentIDE tools."),
  ).toBeVisible();
  await expect(page.getByText("context context-tes")).toBeVisible();
  await expect(page.getByText("code_read, code_changes, code_edit")).toBeVisible();

  await page.getByRole("button", { name: /Editor/ }).click();
  await expect(page).toHaveURL(/pane=editor/);
  await expect(page.locator(".hosted-monaco-editor")).toBeVisible();
  await expect
    .poll(() =>
      page
        .locator(".hosted-monaco-editor .view-line")
        .first()
        .evaluate((line) => {
          const target = line as unknown as {
            ownerDocument: {
              defaultView: {
                getComputedStyle(element: unknown): { fontFamily: string };
              } | null;
            };
          };
          return target.ownerDocument.defaultView?.getComputedStyle(target).fontFamily ?? "";
        }),
    )
    .toContain("JetBrains Mono Variable");
  await expect
    .poll(() =>
      page.locator(".hosted-monaco-editor .view-lines span[class*='mtk']").evaluateAll((tokens) => {
        return new Set(
          tokens.map((token) => {
            const target = token as unknown as {
              ownerDocument: {
                defaultView: {
                  getComputedStyle(element: unknown): { color: string };
                } | null;
              };
            };
            return target.ownerDocument.defaultView?.getComputedStyle(target).color ?? "";
          }),
        ).size;
      }),
    )
    .toBeGreaterThanOrEqual(3);
});

test("drives the hosted terminal byte channel, recovers partial replay, and keeps kill explicit", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop hosted-terminal behavior");
  let input = "";
  let connections = 0;
  const pageErrors: string[] = [];
  page.on("pageerror", (error) => pageErrors.push(error.message));
  await page.routeWebSocket(
    `**/api/project-terminals/${terminalSession.id}/attach*`,
    (webSocket) => {
      connections += 1;
      const connection = connections;
      let sequence = 0n;
      const sendOutput = (content: string | Buffer) => {
        sequence += 1n;
        const payload = typeof content === "string" ? Buffer.from(content) : content;
        const frame = Buffer.allocUnsafe(payload.byteLength + 8);
        frame.writeBigUInt64BE(sequence, 0);
        payload.copy(frame, 8);
        webSocket.send(frame);
      };
      webSocket.onMessage((message) => {
        if (typeof message === "string") return;
        const chunk = Buffer.from(message);
        input += chunk.toString("utf8");
        sendOutput(chunk);
        if (chunk.includes(13)) {
          if (connection === 1) sequence += 1n;
          sendOutput("\r\n/workspace\r\n$ ");
        }
      });
      webSocket.send(
        JSON.stringify({
          kind: "attached",
          replay: { complete: true, oldest_sequence: 0, newest_sequence: 0 },
        }),
      );
      if (connection > 1) {
        sendOutput(
          `${Array.from({ length: 80 }, (_, index) => `retained line ${String(index)}`).join("\r\n")}\r\nterminal-search-marker\r\n`,
        );
      }
      sendOutput("\u001b[32mactor-1@substrate\u001b[0m:/workspace$ ");
    },
  );
  await mockAuthenticatedWorkspace(page, { agentideWorkspace: true, terminalProfile: true });
  await page.goto(`/projects/${project.id}/sessions/${codingSession.id}?terminal=terminal-test`);

  await expect(page.locator(".terminal-connection.running")).toBeVisible();
  await expect
    .poll(() =>
      page.locator(".ghostty-host canvas").evaluate((canvas) => {
        const context = (
          canvas as unknown as {
            getContext(kind: "2d"): { font: string } | null;
          }
        ).getContext("2d");
        return context?.font ?? "";
      }),
    )
    .toContain("JetBrains Mono Variable");
  await page.locator(".ghostty-host").click();
  await page.keyboard.type("pwd");
  await page.keyboard.press("Enter");
  await expect.poll(() => input).toContain("pwd");
  await expect(page.locator(".terminal-connection.partial")).toBeVisible();
  await page.getByRole("button", { name: "Reload retained output" }).click();
  await expect(page.locator(".terminal-connection.running")).toBeVisible();
  await expect.poll(() => connections).toBe(2);
  await page.getByLabel("Search terminal scrollback").fill("terminal-search-marker");
  await page.getByLabel("Search terminal scrollback").press("Enter");
  await expect.poll(() => pageErrors).toEqual([]);

  const panel = page.locator(".workbench-terminal");
  const before = await panel.boundingBox();
  await page.getByRole("separator", { name: "Resize terminal panel" }).press("ArrowUp");
  await expect
    .poll(async () => (await panel.boundingBox())?.height ?? 0)
    .toBeGreaterThan(before?.height ?? 0);

  await page.getByRole("button", { name: "Kill", exact: false }).click();
  await expect(page.locator(".terminal-refused strong")).toHaveText("Terminal terminated");
  await expect(page.getByText("SIGKILL", { exact: true })).toBeVisible();
});

test("drops a stale terminal id instead of reconnecting forever", async ({ page }, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop hosted-terminal behavior");
  await mockAuthenticatedWorkspace(page, {
    agentideWorkspace: true,
    terminalProfile: true,
    staleTerminal: true,
  });

  await page.goto(`/projects/${project.id}/sessions/${codingSession.id}?terminal=terminal-test`);

  await expect(page.getByText("No attached terminals", { exact: true })).toBeVisible();
  await expect(page.locator(".terminal-connection.connecting")).toHaveCount(0);
  await expect(page.getByRole("button", { name: "Open terminal", exact: true })).toBeEnabled();
  await expect.poll(() => new URL(page.url()).searchParams.get("terminal")).toBeNull();
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
  await page.getByRole("button", { name: "Open navigation" }).click();
  await page.getByLabel("Theme").selectOption("solarized-dark");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "solarized-dark");
  await expect(page).toHaveScreenshot("mobile-navigation-solarized-dark.png", {
    animations: "disabled",
    maxDiffPixelRatio: 0.01,
  });
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

test("persists themes and makes search and navigation shortcuts discoverable", async ({
  page,
}, testInfo) => {
  test.skip(testInfo.project.name !== "chromium", "Desktop command surface behavior");
  await mockAuthenticatedWorkspace(page);
  await page.goto("/agents");

  await page.getByLabel("Theme").selectOption("monokai");
  await expect(page.locator("html")).toHaveAttribute("data-theme", "monokai");
  await expect
    .poll(() => page.evaluate(() => localStorage.getItem("b10x.devcenter.theme.v1")))
    .toBe("monokai");
  await page.reload();
  await expect(page.locator("html")).toHaveAttribute("data-theme", "monokai");

  const contrastRatios = await page.evaluate<number[]>(`(() => {
    const themes = ["light", "dark", "monokai", "solarized-light", "solarized-dark"];
    const parse = (color) => {
      const hex = color.trim().slice(1);
      return [0, 2, 4].map((offset) => Number.parseInt(hex.slice(offset, offset + 2), 16));
    };
    const luminance = (color) => {
      const channels = parse(color).map((value) => {
        const normalized = value / 255;
        return normalized <= 0.04045
          ? normalized / 12.92
          : ((normalized + 0.055) / 1.055) ** 2.4;
      });
      return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
    };
    const ratio = (foreground, background) => {
      const left = luminance(foreground);
      const right = luminance(background);
      return (Math.max(left, right) + 0.05) / (Math.min(left, right) + 0.05);
    };
    const ratios = themes.flatMap((theme) => {
      document.documentElement.dataset.theme = theme;
      const style = getComputedStyle(document.documentElement);
      return [
        ratio(style.getPropertyValue("--ink"), style.getPropertyValue("--canvas")),
        ratio(style.getPropertyValue("--muted"), style.getPropertyValue("--surface")),
        ratio(style.getPropertyValue("--on-accent"), style.getPropertyValue("--accent")),
      ];
    });
    document.documentElement.dataset.theme = "monokai";
    return ratios;
  })()`);
  expect(Math.min(...contrastRatios)).toBeGreaterThanOrEqual(4.5);

  await page.keyboard.press("Control+k");
  await expect(page.getByRole("dialog", { name: "Search all Devcenter resources" })).toBeVisible();
  await page.getByRole("combobox", { name: "Search all Devcenter resources" }).fill("todo");
  const todoResult = page.getByRole("option").filter({ hasText: "Todo" }).first();
  await expect(todoResult).toBeVisible();
  await expect(page).toHaveScreenshot("global-search-monokai.png", {
    animations: "disabled",
    maxDiffPixelRatio: 0.01,
  });
  await todoResult.click();
  await expect(page).toHaveURL(/\/services\?service=service(?::|%3A)todo$/);

  await page.keyboard.press("?");
  const shortcutDialog = page.getByRole("dialog", { name: "Keyboard shortcuts" });
  await expect(shortcutDialog).toBeVisible();
  await expect(shortcutDialog.getByText("Capability profiles", { exact: true })).toBeVisible();
  await page.keyboard.press("Escape");
  await page.keyboard.press("g");
  await page.keyboard.press("c");
  await expect(page).toHaveURL(/\/connectors$/);

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
  await expect(page.getByLabel("Capability profile ID")).toHaveText(/Release profile/);
  await expect(page.getByRole("heading", { name: "Release profile" })).toBeVisible();
  await expect(page.locator(".endpoint-row code")).toHaveText(/\/mcp\/pub-test-1$/);
  await expect(
    page.getByText(/codex mcp add devcenter.*--oauth-client-id devcenter-cli/),
  ).toBeVisible();
  await expect(page.getByText(/codex mcp login devcenter --scopes mcp\.tools\.call/)).toBeVisible();
  await expect(
    page.getByText(/claude mcp add --transport http --client-id devcenter-cli/),
  ).toBeVisible();
  await expect(page.getByText("Browser logout does not revoke it.")).toBeVisible();
  await page.getByRole("button", { name: "Revoke permanently" }).click();
  await expect(page.getByRole("status").filter({ hasText: "Publication revoked." })).toBeVisible();
  const accessibility = await new AxeBuilder({ page }).withTags(["wcag2a", "wcag2aa"]).analyze();
  expect(accessibility.violations).toEqual([]);
});
