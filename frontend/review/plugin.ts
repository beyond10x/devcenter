import type { IncomingMessage, ServerResponse } from "node:http";
import { Buffer } from "node:buffer";
import type { Plugin } from "vite";

interface ReviewAgent {
  id: string;
  tenant_id: string;
  name: string;
  active_revision: number;
  latest_revision: number;
  created_by: string;
  created_at_ms: number;
}

interface ReviewPublication {
  publication_id: string;
  tenant_id: string;
  owner_subject: string;
  profile_id: string;
  active_revision: number;
  toolset_digest: string;
  state: "active" | "suspended";
  created_at_ms: number;
  updated_at_ms: number;
}

let connected = false;
let nextAgent = 3;
let nextPublication = 2;
const initialPublication: ReviewPublication = {
  publication_id: "pub_review_7mz4v2",
  tenant_id: "review-tenant",
  owner_subject: "review-engineer",
  profile_id: "profile-release-operations",
  active_revision: 4,
  toolset_digest: "41dc0e9963dd312bb656d0907b91990a16f57e8465886407476776ca08284f57",
  state: "active",
  created_at_ms: 1_788_260_000_000,
  updated_at_ms: 1_788_260_000_000,
};
const publications: ReviewPublication[] = [initialPublication];
const agents: ReviewAgent[] = [
  {
    id: "agent-release",
    tenant_id: "review-tenant",
    name: "Release steward",
    active_revision: 3,
    latest_revision: 3,
    created_by: "review-engineer",
    created_at_ms: 1_767_225_600_000,
  },
  {
    id: "agent-review",
    tenant_id: "review-tenant",
    name: "Change reviewer",
    active_revision: 1,
    latest_revision: 1,
    created_by: "review-engineer",
    created_at_ms: 1_769_904_000_000,
  },
];
const reviewProject = {
  id: "project-review-devcenter",
  forge_instance_ref: "connection:gitlab:review",
  project_ref: "1042",
  path_with_namespace: "foundation/devcenter",
  name: "devcenter",
  default_branch: "trunk",
  selected_branch: "trunk",
  pinned_commit: "6d17f3812ca53ef7aacb4cb973bcbb2ddc93be12",
  web_url: "https://gitlab.example.test/foundation/devcenter",
};
const reviewBranches = [
  { name: "trunk", commit: reviewProject.pinned_commit, provider_default: true, protected: true },
  {
    name: "feature/projects",
    commit: "ec214cbd24300df85427c2016af0f4218909a932",
    provider_default: false,
    protected: false,
  },
];
const reviewTree = [
  { object_id: "tree-crates", name: "crates", path: "crates", kind: "tree", mode: "040000" },
  {
    object_id: "blob-agents",
    name: "AGENTS.md",
    path: "AGENTS.md",
    kind: "blob",
    mode: "100644",
  },
  {
    object_id: "blob-cargo",
    name: "Cargo.toml",
    path: "Cargo.toml",
    kind: "blob",
    mode: "100644",
  },
  {
    object_id: "blob-readme",
    name: "README.md",
    path: "README.md",
    kind: "blob",
    mode: "100644",
  },
];
const reviewArtifacts = {
  artifacts: [
    {
      id: "artifact-review-boundary",
      locator: "ep://foundation/devcenter/design/repository-workspace-boundary",
      entity_type: "aep.design/v1",
      revision: 3,
      title: "Repository workspace boundary",
      status: "draft",
      updated_at_ms: 1_788_260_000_000,
      source_revision: reviewProject.pinned_commit,
    },
  ],
  has_more: false,
};
const reviewThreads: Array<Record<string, unknown>> = [];
const reviewMessages = new Map<string, Array<Record<string, unknown>>>();
const reviewWorkflows = [
  {
    id: "review.code/v1",
    name: "Code review",
    description: "Commit-pinned correctness and maintainability findings with file citations.",
  },
  {
    id: "review.security/v1",
    name: "Security review",
    description: "Commit-pinned security findings with typed severity and evidence.",
  },
  {
    id: "reverse.aep-ess/v1",
    name: "Reverse AEP + ESS",
    description:
      "Evidence-backed draft planning entities and a current-state system specification.",
  },
];
const reviewProviders = [
  {
    provider_ref: "gitlab",
    authority: "com.gitlab",
    vendor: "GitLab",
    description: "Projects, branches, merge requests, issues, and pipelines.",
    audiences: ["https://gitlab.example.test"],
    services: ["git", "issues", "ci"],
    operation_count: 24,
    configurable: true,
    setup_profiles: [
      { auth_profile: "gitlab.oauth_user", actor: "person" },
      { auth_profile: "gitlab.personal_token", actor: "person" },
    ],
  },
  {
    provider_ref: "slack",
    authority: "com.slack.api",
    vendor: "Slack",
    description: "Channels, messages, replies, reactions, and users.",
    audiences: ["https://slack.com/api"],
    services: ["messaging"],
    operation_count: 12,
    configurable: false,
    setup_profiles: [],
  },
];
const reviewGitlabOperations = [
  {
    operation_ref: "git.project.list",
    service: "git",
    description: "List projects visible to the connected person.",
    risk: "read_only",
    exposed: true,
  },
  {
    operation_ref: "git.merge_request.create",
    service: "git",
    description: "Create a merge request from an existing branch.",
    risk: "mutating",
    exposed: false,
  },
];
const reviewServiceCatalog = {
  format: "service-catalog/1",
  service_ref: "service:todo",
  display_name: "Todo",
  description: "Shared scoped lists and intent-driven items.",
  semantic_catalog: {
    format: "ess-browser-catalog/1",
    system: "todo",
    entities: [
      {
        name: "todo.list.TodoList",
        display: "Todo list",
        initial: "active",
        states: ["active", "archived", "expired"],
        transitions: [
          { name: "archive", from: ["active"], to: "archived" },
          { name: "expire", from: ["active", "archived"], to: "expired" },
        ],
      },
    ],
    views: [
      {
        name: "todo.list.VisibleLists",
        display: "Visible lists",
        consistency: "read_your_writes",
        fields: [
          { name: "list_id", wire: "list-id" },
          { name: "title", wire: "title" },
          { name: "state", wire: "state" },
        ],
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

function sendJson(response: ServerResponse, status: number, value: unknown) {
  response.statusCode = status;
  response.setHeader("content-type", "application/json");
  response.setHeader("cache-control", "no-store");
  response.end(JSON.stringify(value));
}

async function readJson(request: IncomingMessage): Promise<Record<string, unknown>> {
  const decoder = new TextDecoder();
  let body = "";
  for await (const chunk of request as AsyncIterable<unknown>) {
    if (typeof chunk === "string") body += chunk;
    else if (chunk instanceof Uint8Array || Buffer.isBuffer(chunk))
      body += decoder.decode(chunk, { stream: true });
    else throw new TypeError("unsupported request body chunk");
  }
  body += decoder.decode();
  return JSON.parse(body) as Record<string, unknown>;
}

function sendTaskEvents(response: ServerResponse) {
  response.statusCode = 200;
  response.setHeader("content-type", "text/event-stream");
  response.setHeader("cache-control", "no-store");
  response.setHeader("connection", "keep-alive");
  response.flushHeaders();

  const events = [
    { event: { kind: "accepted" } },
    { event: { kind: "running" } },
    { event: { kind: "text_delta", text: "Reviewing the requested outcome…\n" } },
    { event: { kind: "text_delta", text: "• Identity session verified\n" } },
    { event: { kind: "text_delta", text: "• Connector authority checked\n" } },
    {
      event: {
        kind: "succeeded",
        output:
          "Review complete.\n\nIdentity session verified.\nConnector authority checked.\nNo external changes were made.",
      },
    },
  ];
  events.forEach((event, index) => {
    setTimeout(
      () => {
        if (response.destroyed) return;
        response.write(`event: task\ndata: ${JSON.stringify(event)}\n\n`);
        if (index === events.length - 1) response.end();
      },
      250 + index * 450,
    );
  });
}

export function reviewApi(): Plugin {
  return {
    name: "devcenter-review-api",
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        void (async () => {
          const method = request.method ?? "GET";
          const url = new URL(request.url ?? "/", "http://review.local");
          const path = url.pathname;

          if (path === "/review/provider") {
            response.setHeader("content-type", "text/html; charset=utf-8");
            response.end(
              "<!doctype html><title>Review authorization</title><main style='font:16px system-ui;max-width:40rem;margin:5rem auto;padding:2rem'><h1>Review authorization approved</h1><p>Return to Devcenter and enter <strong>REVIEW-CODE</strong>.</p><p>This local review flow contacts no provider.</p></main>",
            );
            return;
          }
          if (path === "/api/session" && method === "GET") {
            sendJson(response, 200, {
              tenant_id: "review-tenant",
              subject: "review-engineer",
              email: "reviewer@example.test",
              groups: ["engineers"],
              connectors_docs_available: true,
            });
            return;
          }
          if (path === "/api/agents" && method === "GET") {
            sendJson(response, 200, agents);
            return;
          }
          if (path === "/api/repositories" && method === "GET") {
            sendJson(response, 200, [
              {
                forge_instance_ref: "connection:gitlab:review",
                project_ref: "1042",
                path_with_namespace: "foundation/devcenter",
                name: "devcenter",
                default_branch: "trunk",
                visibility: "private",
                web_url: reviewProject.web_url,
                opened_project_id: reviewProject.id,
              },
              {
                forge_instance_ref: "connection:gitlab:review",
                project_ref: "2081",
                path_with_namespace: "foundation/workflow",
                name: "workflow",
                default_branch: "stable",
                visibility: "private",
                web_url: "https://gitlab.example.test/foundation/workflow",
              },
            ]);
            return;
          }
          if (path === "/api/projects" && method === "POST") {
            sendJson(response, 200, reviewProject);
            return;
          }
          if (path === `/api/projects/${reviewProject.id}` && method === "GET") {
            sendJson(response, 200, reviewProject);
            return;
          }
          if (path === `/api/projects/${reviewProject.id}/branches` && method === "GET") {
            sendJson(response, 200, reviewBranches);
            return;
          }
          if (path === `/api/projects/${reviewProject.id}/tree` && method === "GET") {
            sendJson(response, 200, reviewTree);
            return;
          }
          if (
            path === `/api/projects/${reviewProject.id}/engineering-artifacts` &&
            method === "GET"
          ) {
            sendJson(response, 200, reviewArtifacts);
            return;
          }
          if (path === `/api/projects/${reviewProject.id}/branch` && method === "POST") {
            const submitted = await readJson(request);
            const branch = reviewBranches.find((candidate) => candidate.name === submitted.branch);
            if (!branch) {
              sendJson(response, 404, { code: "workspace_resource_not_found" });
              return;
            }
            reviewProject.selected_branch = branch.name;
            reviewProject.pinned_commit = branch.commit;
            sendJson(response, 200, reviewProject);
            return;
          }
          if (path === `/api/projects/${reviewProject.id}/threads` && method === "GET") {
            sendJson(response, 200, reviewThreads);
            return;
          }
          if (path === `/api/projects/${reviewProject.id}/threads` && method === "POST") {
            const submitted = await readJson(request);
            const thread = {
              id: `thread-review-${String(reviewThreads.length + 1)}`,
              project_id: reviewProject.id,
              branch: submitted.branch,
              pinned_commit: submitted.pinned_commit,
              title: submitted.title,
              created_at_ms: Date.now(),
            };
            reviewThreads.unshift(thread);
            reviewMessages.set(thread.id, []);
            sendJson(response, 200, thread);
            return;
          }
          const messageMatch = path.match(/^\/api\/threads\/([^/]+)\/messages$/);
          if (messageMatch && method === "GET") {
            sendJson(response, 200, reviewMessages.get(messageMatch[1]) ?? []);
            return;
          }
          if (messageMatch && method === "POST") {
            const submitted = await readJson(request);
            const threadId = messageMatch[1];
            const current = reviewMessages.get(threadId) ?? [];
            const message = {
              sequence: current.length + 1,
              role: "user",
              content: submitted.content,
              branch: reviewProject.selected_branch,
              commit: reviewProject.pinned_commit,
              created_at_ms: Date.now(),
            };
            current.push(message);
            reviewMessages.set(threadId, current);
            sendJson(response, 200, message);
            return;
          }
          if (path === `/api/projects/${reviewProject.id}/workflows` && method === "GET") {
            sendJson(response, 200, reviewWorkflows);
            return;
          }
          if (path === `/api/projects/${reviewProject.id}/workflow-runs` && method === "POST") {
            const submitted = await readJson(request);
            sendJson(response, 200, {
              id: `run-review-${String(Date.now())}`,
              definition_id: submitted.definition_id,
              project_id: reviewProject.id,
              branch: submitted.branch,
              commit: submitted.commit,
              state: "accepted",
              created_at_ms: Date.now(),
            });
            return;
          }
          if (path === "/api/agents" && method === "POST") {
            const submitted = await readJson(request);
            const created: ReviewAgent = {
              ...agents[0],
              id: `agent-review-${String(nextAgent++)}`,
              name: typeof submitted.name === "string" ? submitted.name : "Review agent",
              active_revision: 1,
              latest_revision: 1,
              created_at_ms: Date.now(),
            };
            agents.unshift(created);
            sendJson(response, 201, created);
            return;
          }
          if (path === "/api/mcp/publications" && method === "GET") {
            sendJson(response, 200, publications);
            return;
          }
          if (path === "/api/mcp/publications" && method === "POST") {
            const submitted = await readJson(request);
            if (typeof submitted.profile_id !== "string" || !submitted.profile_id.trim()) {
              sendJson(response, 422, { code: "capability_profile_id_invalid" });
              return;
            }
            const now = Date.now();
            const created: ReviewPublication = {
              ...initialPublication,
              publication_id: `pub_review_created_${String(nextPublication++)}`,
              profile_id: submitted.profile_id.trim(),
              active_revision: 1,
              state: "active",
              created_at_ms: now,
              updated_at_ms: now,
            };
            publications.unshift(created);
            sendJson(response, 201, created);
            return;
          }
          const publicationMatch = path.match(/^\/api\/mcp\/publications\/([^/]+)$/);
          if (publicationMatch && method === "PATCH") {
            const current = publications.find(
              (publication) => publication.publication_id === publicationMatch[1],
            );
            if (!current) {
              sendJson(response, 404, { code: "publication_not_found" });
              return;
            }
            const submitted = await readJson(request);
            if (submitted.state === "active" || submitted.state === "suspended") {
              current.state = submitted.state;
              current.updated_at_ms = Date.now();
              sendJson(response, 200, current);
            } else {
              sendJson(response, 503, { code: "identity_publication_revocation_unavailable" });
            }
            return;
          }
          const clientsMatch = path.match(/^\/api\/mcp\/publications\/([^/]+)\/clients$/);
          if (clientsMatch && method === "GET") {
            const current = publications.find(
              (publication) => publication.publication_id === clientsMatch[1],
            );
            if (!current) {
              sendJson(response, 404, { code: "publication_not_found" });
              return;
            }
            sendJson(
              response,
              200,
              current.publication_id === initialPublication.publication_id
                ? [
                    {
                      authorization_id: "authorization-review-codex",
                      publication_id: current.publication_id,
                      subject: "review-engineer",
                      client_id: "codex-cli",
                      display_name: "Codex CLI",
                      state: "active",
                      first_used_at_ms: 1_788_260_000_000,
                      last_used_at_ms: 1_788_260_600_000,
                    },
                  ]
                : [],
            );
            return;
          }
          const approvalsMatch = path.match(/^\/api\/mcp\/publications\/([^/]+)\/approvals$/);
          if (approvalsMatch && method === "GET") {
            if (
              !publications.some((publication) => publication.publication_id === approvalsMatch[1])
            ) {
              sendJson(response, 404, { code: "publication_not_found" });
              return;
            }
            sendJson(response, 200, []);
            return;
          }
          if (path === "/api/connectors/claude-code" && method === "GET") {
            sendJson(response, 200, { provider: "claude-code", connected });
            return;
          }
          if (path === "/api/services" && method === "GET") {
            sendJson(response, 200, {
              services: [
                {
                  service_ref: reviewServiceCatalog.service_ref,
                  display_name: reviewServiceCatalog.display_name,
                  description: reviewServiceCatalog.description,
                  digest: "a".repeat(64),
                },
              ],
            });
            return;
          }
          if (path === "/api/services/catalog" && method === "POST") {
            const submitted = await readJson(request);
            if (submitted.service_ref !== reviewServiceCatalog.service_ref) {
              sendJson(response, 404, { code: "service_operation_not_found" });
              return;
            }
            sendJson(response, 200, reviewServiceCatalog);
            return;
          }
          if (path === "/api/services/invoke" && method === "POST") {
            const submitted = await readJson(request);
            if (submitted.operation_ref === "todo.list_visible_lists") {
              sendJson(response, 200, {
                output: [{ list_id: "release", title: "Release service console", state: "active" }],
                connector_audit_ref: "audit:review:query",
              });
              return;
            }
            if (submitted.operation_ref === "todo.create_list" && submitted.confirmed === true) {
              sendJson(response, 200, {
                output: {
                  outcome: "applied",
                  events: ["todo.list.ListCreated"],
                  through_version: 1,
                  replayed: false,
                },
                connector_audit_ref: "audit:review:intent",
              });
              return;
            }
            sendJson(response, 409, { code: "service_write_confirmation_required" });
            return;
          }
          if (path === "/api/connectors/catalog" && method === "GET") {
            const query = (url.searchParams.get("query") ?? "").toLowerCase();
            const providers = reviewProviders.filter(
              (provider) =>
                !query ||
                provider.provider_ref.includes(query) ||
                provider.vendor.toLowerCase().includes(query) ||
                provider.description.toLowerCase().includes(query),
            );
            sendJson(response, 200, { providers, next_offset: null });
            return;
          }
          if (path === "/api/connectors/catalog/gitlab" && method === "GET") {
            sendJson(response, 200, {
              provider: reviewProviders[0],
              operations: reviewGitlabOperations,
            });
            return;
          }
          if (path === "/api/connections" && method === "GET") {
            sendJson(response, 200, []);
            return;
          }
          if (path === "/api/connections" && method === "POST") {
            const submitted = await readJson(request);
            sendJson(response, 201, {
              connect_session_ref: `review-connect-${String(Date.now())}`,
              integration_ref: submitted.integration_ref,
              state: "completed",
              expires_at_unix_ms: Date.now() + 600_000,
              connection_ref: `connection:${String(submitted.integration_ref)}:review`,
            });
            return;
          }
          if (path === "/api/connectors/claude-code" && method === "DELETE") {
            connected = false;
            sendJson(response, 200, { provider: "claude-code", connected });
            return;
          }
          if (path === "/api/connectors/claude-code/oauth/start" && method === "POST") {
            sendJson(response, 200, {
              authorization_url: "/review/provider",
              flow_id: "review-flow",
              expires_at: Math.floor(Date.now() / 1000) + 600,
            });
            return;
          }
          if (path === "/api/connectors/claude-code/oauth/complete" && method === "POST") {
            const submitted = await readJson(request);
            if (submitted.flow_id !== "review-flow" || submitted.code !== "REVIEW-CODE") {
              sendJson(response, 422, { code: "claude_connection_refused" });
              return;
            }
            connected = true;
            sendJson(response, 200, { provider: "claude-code", connected });
            return;
          }
          if (/^\/api\/agents\/[^/]+\/tasks$/.test(path) && method === "POST") {
            sendJson(response, 202, {
              id: `task-review-${String(Date.now())}`,
              status: "accepted",
            });
            return;
          }
          if (/^\/api\/tasks\/[^/]+\/events$/.test(path) && method === "GET") {
            sendTaskEvents(response);
            return;
          }
          if (path.startsWith("/api/")) {
            sendJson(response, 404, { code: "review_route_not_found" });
            return;
          }
          next();
        })().catch(next);
      });
    },
  };
}
