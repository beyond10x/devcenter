import type { IncomingMessage, ServerResponse } from "node:http";
import { Buffer } from "node:buffer";
import type { Duplex } from "node:stream";
import type { Plugin } from "vite";
import { WebSocket, WebSocketServer } from "ws";

const reviewTerminalUpstream = process.env.DEVCENTER_REVIEW_TERMINAL_UPSTREAM?.trim() || undefined;

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

interface ReviewCapabilityMapping {
  operation_ref: string;
  tool_name: string;
  connection_ref?: string;
  posture: "allow" | "approval_required" | "deny";
}

interface ReviewCapabilityProfile {
  id: string;
  name: string;
  audience: "personal" | "tenant";
  revision: number;
  mappings: ReviewCapabilityMapping[];
  created_by: string;
  created_at_ms: number;
  updated_at_ms: number;
}

interface ReviewTask {
  id: string;
  agent_id: string;
  status: string;
  attempt_id: string;
  prompt: string;
  output: string | null;
  failure_code: string | null;
  failure_message: string | null;
  accepted_at_ms: number;
  completed_at_ms: number | null;
  workspace_session_id?: string;
  agentide_session_id?: string;
}

let connected = false;
let nextAgent = 3;
let nextPublication = 2;
let nextCapabilityProfile = 2;
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
const reviewCapabilities = [
  {
    operation_ref: "git.project.list",
    title: "List GitLab projects",
    effect: "read_only",
    approval: "not_required",
    connections: [
      {
        connection_ref: "connection:gitlab:review",
        label: "Review GitLab",
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
        connection_ref: "connection:todo:review",
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
        connection_ref: "connection:todo:review",
        label: "Todo",
        provider: "todo",
        audiences: [],
      },
    ],
  },
] as const;
const capabilityProfiles: ReviewCapabilityProfile[] = [
  {
    id: "profile-release-operations",
    name: "Release operations",
    audience: "personal",
    revision: 4,
    mappings: [
      {
        operation_ref: "git.project.list",
        tool_name: "git_project_list",
        connection_ref: "connection:gitlab:review",
        posture: "allow",
      },
      {
        operation_ref: "todo.list_visible_lists",
        tool_name: "todo_list_visible_lists",
        connection_ref: "connection:todo:review",
        posture: "allow",
      },
      {
        operation_ref: "todo.create_list",
        tool_name: "todo_create_list",
        connection_ref: "connection:todo:review",
        posture: "approval_required",
      },
    ],
    created_by: "review-engineer",
    created_at_ms: 1_788_260_000_000,
    updated_at_ms: 1_788_260_000_000,
  },
];
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
const reviewCodingSession = {
  id: "workspace-session-review",
  project_id: reviewProject.id,
  source_revision: reviewProject.pinned_commit,
  base_materialization_ref: "substrate:base:review",
  working_materialization_ref: "substrate:working:review",
  manifest_sha256: "a".repeat(64),
  state: "ready",
  failure_code: null,
  limits: { max_files: 10_000, max_total_bytes: 268_435_456, max_file_bytes: 4_194_304 },
  created_at_ms: 1_788_260_000_000,
  updated_at_ms: 1_788_260_000_000,
};
const reviewCodingSource = `use std::process::ExitCode;

fn main() -> ExitCode {
    println!("DevCenter owns the human surface; Workspace owns files and diffs.");
    ExitCode::SUCCESS
}
`;
let reviewAgentIdeVersion = 1;
const reviewAgentIdeSessionId = reviewCodingSession.id;
const initialReviewTerminalGrant: Record<string, unknown> = {
  grant_id: "grant-review-terminal",
  grantee: "review-engineer",
  allowed_intents: ["interactive_terminal"],
  path_prefixes: [""],
  maximum_risk: "Medium",
  expires_at: null,
  revision: 1,
  state: "Active",
};
const reviewAgentIdeGrants: Array<Record<string, unknown>> = reviewTerminalUpstream
  ? [initialReviewTerminalGrant]
  : [];
const reviewAgentIdePins: Array<Record<string, unknown>> = [];
const reviewAgentIdeCheckpoints: Array<Record<string, unknown>> = [];
const reviewTasks: ReviewTask[] = [];

function reviewCoordinationSummary() {
  return {
    state: "ready" as const,
    through_version: reviewAgentIdeVersion,
    failure_code: null,
    retryable: false,
  };
}

function reviewCoordinationView() {
  return {
    summary: reviewCoordinationSummary(),
    session: {
      session_id: reviewAgentIdeSessionId,
      workspace_session_id: reviewCodingSession.id,
      project_id: reviewProject.id,
      source_revision: reviewProject.pinned_commit,
      manifest_digest: reviewCodingSession.manifest_sha256,
      objective: "Review the hosted coding workspace",
      state: "Active" as const,
    },
    grants: reviewAgentIdeGrants,
    pins: reviewAgentIdePins,
    checkpoints: reviewAgentIdeCheckpoints,
  };
}

function ensureReviewTerminalGrant() {
  const active = reviewAgentIdeGrants.find(
    (candidate) =>
      candidate.grantee === "review-engineer" &&
      candidate.state === "Active" &&
      Array.isArray(candidate.allowed_intents) &&
      candidate.allowed_intents.includes("interactive_terminal"),
  );
  if (typeof active?.grant_id === "string") return active.grant_id;
  const grantId = `grant-review-terminal-${String(reviewAgentIdeGrants.length + 1)}`;
  reviewAgentIdeGrants.push({ ...initialReviewTerminalGrant, grant_id: grantId });
  reviewAgentIdeVersion += 1;
  return grantId;
}
const reviewTerminalProfile = {
  id: "rust-stable-confined",
  label: reviewTerminalUpstream ? "Rust stable · real daemon lab" : "Rust stable · confined",
  runtime_ref: reviewTerminalUpstream
    ? "substrate:local-daemon-lab"
    : "substrate:image:rust-stable-review",
  shell: "/bin/sh",
  arguments: ["-i"],
  working_directory: "/workspace",
  environment: { TERM: "xterm-256color", COLORTERM: "truecolor" },
  workspace_access: "read_write",
  network: "none",
  limits: {
    timeout_ms: 3_600_000,
    cpu_millis: 3_600_000,
    memory_bytes: 536_870_912,
    processes: 128,
    output_bytes: 1_048_576,
    input_bytes: 16_777_216,
    frame_bytes: 65_536,
    queued_frames: 16,
    lease_ttl_ms: 3_600_000,
  },
} as const;
interface ReviewTerminal {
  id: string;
  coding_session_id: string;
  agentide_session_id: string;
  authority_grant_id: string;
  profile: typeof reviewTerminalProfile;
  actor: string;
  process_id: string | null;
  state: "preparing" | "running" | "exited" | "terminated" | "refused" | "unknown";
  exit: { code: number | null; signal: string | null } | null;
  failure_code: string | null;
  created_at_ms: number;
  updated_at_ms: number;
}

const initialReviewTerminal: ReviewTerminal = {
  id: "terminal-review-1",
  coding_session_id: reviewCodingSession.id,
  agentide_session_id: reviewAgentIdeSessionId,
  authority_grant_id: "grant-review-terminal",
  profile: reviewTerminalProfile,
  actor: "review-engineer",
  process_id: reviewTerminalUpstream
    ? "real-substrate-daemon-via-workspace-lab"
    : "substrate-process-review-1",
  state: "running",
  exit: null,
  failure_code: null,
  created_at_ms: 1_788_260_100_000,
  updated_at_ms: 1_788_260_100_000,
};
const reviewTerminals: ReviewTerminal[] = reviewTerminalUpstream ? [initialReviewTerminal] : [];
const reviewTerminalSequences = new WeakMap<WebSocket, bigint>();
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

function sendTerminalOutput(webSocket: WebSocket, value: string | Buffer) {
  if (webSocket.readyState !== WebSocket.OPEN) return;
  const sequence = (reviewTerminalSequences.get(webSocket) ?? 0n) + 1n;
  reviewTerminalSequences.set(webSocket, sequence);
  const payload = typeof value === "string" ? Buffer.from(value) : value;
  const frame = Buffer.allocUnsafe(8 + payload.byteLength);
  frame.writeBigUInt64BE(sequence, 0);
  payload.copy(frame, 8);
  webSocket.send(frame, { binary: true });
}

function reviewTerminalCommand(value: string): string {
  const command = value.trim();
  const prompt = "\u001b[32mreview-engineer@substrate\u001b[0m:\u001b[34m/workspace\u001b[0m$ ";
  const output =
    command === ""
      ? ""
      : command === "pwd"
        ? "/workspace\r\n"
        : command === "ls" || command === "ls -la"
          ? "\u001b[34msrc\u001b[0m  Cargo.toml  README.md\r\n"
          : command === "whoami"
            ? "review-engineer\r\n"
            : command === "top"
              ? "\u001b[1mReview telemetry (not a real process list)\u001b[0m\r\nPID  PROCESS              STATE\r\n42   substrate-review     confined\r\n"
              : `review emulator: “${command}” was not executed by substrate-daemon\r\n`;
  return `\r\n${output}${prompt}`;
}

function terminalUpstreamUrl(requestUrl: string): URL {
  if (!reviewTerminalUpstream) throw new Error("terminal_lab_not_configured");
  const base = new URL(reviewTerminalUpstream);
  if (
    !["ws:", "wss:"].includes(base.protocol) ||
    !["127.0.0.1", "localhost", "[::1]"].includes(base.hostname)
  ) {
    throw new Error("terminal_lab_origin_refused");
  }
  return new URL(requestUrl, base);
}

function bridgeReviewTerminal(browser: WebSocket, upstream: WebSocket) {
  browser.on("message", (data, isBinary) => {
    if (upstream.readyState === WebSocket.OPEN) upstream.send(data, { binary: isBinary });
  });
  upstream.on("message", (data, isBinary) => {
    if (browser.readyState === WebSocket.OPEN) browser.send(data, { binary: isBinary });
  });
  browser.on("close", () => upstream.close());
  upstream.on("close", () => browser.close());
  browser.on("error", () => upstream.close());
  upstream.on("error", () => browser.close());
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
    { event: { kind: "context_changed", revision: "context-review-2" } },
    {
      event: {
        kind: "inventory_changed",
        revision: "inventory-review-2",
        published_tools: ["code_read", "code_changes", "code_edit", "code_create"],
      },
    },
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
  const terminalWebSockets = new WebSocketServer({ noServer: true });
  const terminalUpstreams = new WeakMap<WebSocket, WebSocket>();
  return {
    name: "devcenter-review-api",
    configureServer(server) {
      server.httpServer?.on("upgrade", (request: IncomingMessage, socket: Duplex, head: Buffer) => {
        const url = new URL(request.url ?? "/", "http://review.local");
        const match = url.pathname.match(/^\/api\/project-terminals\/([^/]+)\/attach$/);
        if (!match) return;
        if (!reviewTerminals.some((terminal) => terminal.id === match[1])) {
          terminalWebSockets.handleUpgrade(request, socket, head, (webSocket) => {
            webSocket.send(
              JSON.stringify({ kind: "refused", code: "workspace_terminal_not_found" }),
              () => webSocket.close(1008, "workspace_terminal_not_found"),
            );
          });
          return;
        }
        if (reviewTerminalUpstream) {
          let upstream: WebSocket;
          try {
            upstream = new WebSocket(terminalUpstreamUrl(request.url ?? "/"));
          } catch {
            socket.destroy();
            return;
          }
          const timeout = setTimeout(() => {
            upstream.terminate();
            socket.destroy();
          }, 10_000);
          const failed = () => {
            clearTimeout(timeout);
            socket.destroy();
          };
          upstream.once("error", failed);
          upstream.once("open", () => {
            clearTimeout(timeout);
            upstream.off("error", failed);
            terminalWebSockets.handleUpgrade(request, socket, head, (webSocket) => {
              terminalUpstreams.set(webSocket, upstream);
              terminalWebSockets.emit("connection", webSocket, request);
            });
          });
          return;
        }
        terminalWebSockets.handleUpgrade(request, socket, head, (webSocket) => {
          terminalWebSockets.emit("connection", webSocket, request);
        });
      });
      terminalWebSockets.on("connection", (webSocket, request) => {
        const upstream = terminalUpstreams.get(webSocket);
        if (upstream) {
          bridgeReviewTerminal(webSocket, upstream);
          return;
        }
        const url = new URL(request.url ?? "/", "http://review.local");
        const requestedSequence = url.searchParams.get("from_sequence");
        const initialSequence =
          requestedSequence && /^\d+$/.test(requestedSequence) ? BigInt(requestedSequence) : 0n;
        reviewTerminalSequences.set(webSocket, initialSequence);
        webSocket.send(
          JSON.stringify({
            kind: "attached",
            replay: {
              complete: true,
              oldest_sequence: Number(initialSequence),
              newest_sequence: Number(initialSequence),
            },
          }),
        );
        if (!url.searchParams.has("from_sequence")) {
          sendTerminalOutput(
            webSocket,
            "\u001b[2mConfined Substrate review process · network none\u001b[0m\r\n" +
              "\u001b[32mreview-engineer@substrate\u001b[0m:\u001b[34m/workspace\u001b[0m$ ",
          );
        }
        let inputBuffer = "";
        webSocket.on("message", (data, isBinary) => {
          if (!isBinary) return;
          const input = Array.isArray(data)
            ? Buffer.concat(data)
            : data instanceof ArrayBuffer
              ? Buffer.from(data)
              : Buffer.from(data.buffer, data.byteOffset, data.byteLength);
          sendTerminalOutput(webSocket, input);
          inputBuffer += input.toString("utf8");
          const lines = inputBuffer.split(/\r\n|\r|\n/);
          inputBuffer = lines.pop() ?? "";
          for (const command of lines) {
            sendTerminalOutput(webSocket, reviewTerminalCommand(command));
          }
        });
      });
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
              agentide_workspace_enabled: true,
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
          if (path === `/api/projects/${reviewProject.id}/sessions` && method === "GET") {
            sendJson(response, 200, [reviewCodingSession]);
            return;
          }
          if (path === `/api/project-sessions/${reviewCodingSession.id}` && method === "GET") {
            sendJson(response, 200, reviewCodingSession);
            return;
          }
          if (
            path === `/api/project-sessions/${reviewCodingSession.id}/resume` &&
            method === "POST"
          ) {
            sendJson(response, 200, {
              ...reviewCodingSession,
              coordination: reviewCoordinationSummary(),
            });
            return;
          }
          if (
            path === `/api/project-sessions/${reviewCodingSession.id}/coordination` &&
            method === "GET"
          ) {
            sendJson(response, 200, reviewCoordinationView());
            return;
          }
          if (
            path === `/api/project-sessions/${reviewCodingSession.id}/coordination/pins` &&
            method === "POST"
          ) {
            const submitted = await readJson(request);
            reviewAgentIdeVersion += 1;
            reviewAgentIdePins.push({
              pin_id: `pin-review-${String(reviewAgentIdePins.length + 1)}`,
              kind: submitted.kind,
              reference: submitted.reference,
              start_line: submitted.start_line,
              end_line: submitted.end_line,
              sha256: submitted.sha256,
              state: "Active",
            });
            sendJson(response, 200, reviewCoordinationView());
            return;
          }
          const coordinationPinMatch = path.match(
            /^\/api\/project-sessions\/([^/]+)\/coordination\/pins\/([^/]+)$/,
          );
          if (coordinationPinMatch?.[1] === reviewCodingSession.id && method === "DELETE") {
            const pin = reviewAgentIdePins.find(
              (candidate) => candidate.pin_id === coordinationPinMatch[2],
            );
            if (!pin) {
              sendJson(response, 404, { code: "agentide_context_pin_not_found" });
              return;
            }
            pin.state = "Removed";
            reviewAgentIdeVersion += 1;
            sendJson(response, 200, reviewCoordinationView());
            return;
          }
          if (
            path === `/api/project-sessions/${reviewCodingSession.id}/coordination/grants` &&
            method === "POST"
          ) {
            const submitted = await readJson(request);
            const grantId = `grant-review-${String(reviewAgentIdeGrants.length + 1)}`;
            reviewAgentIdeVersion += 1;
            reviewAgentIdeGrants.push({
              grant_id: grantId,
              grantee: submitted.grantee,
              allowed_intents: ["code_edit", "code_create", "code_delete", "code_rename"],
              path_prefixes: ["."],
              maximum_risk: "Medium",
              expires_at: null,
              revision: 1,
              state: "Active",
            });
            sendJson(response, 200, reviewCoordinationView());
            return;
          }
          const coordinationGrantMatch = path.match(
            /^\/api\/project-sessions\/([^/]+)\/coordination\/grants\/([^/]+)$/,
          );
          if (coordinationGrantMatch?.[1] === reviewCodingSession.id && method === "DELETE") {
            const grant = reviewAgentIdeGrants.find(
              (candidate) => candidate.grant_id === coordinationGrantMatch[2],
            );
            if (!grant) {
              sendJson(response, 404, { code: "agentide_grant_not_found" });
              return;
            }
            grant.state = "Revoked";
            reviewAgentIdeVersion += 1;
            sendJson(response, 200, reviewCoordinationView());
            return;
          }
          const coordinationCheckpointMatch = path.match(
            /^\/api\/project-sessions\/([^/]+)\/coordination\/checkpoints\/([^/]+)$/,
          );
          if (coordinationCheckpointMatch?.[1] === reviewCodingSession.id && method === "POST") {
            const checkpoint = reviewAgentIdeCheckpoints.find(
              (candidate) => candidate.checkpoint_id === coordinationCheckpointMatch[2],
            );
            if (!checkpoint) {
              sendJson(response, 404, { code: "agentide_checkpoint_not_found" });
              return;
            }
            const submitted = await readJson(request);
            checkpoint.state = submitted.decision === "approve" ? "Approved" : "Denied";
            reviewAgentIdeVersion += 1;
            sendJson(response, 200, reviewCoordinationView());
            return;
          }
          if (path === `/api/project-sessions/${reviewCodingSession.id}/tree` && method === "GET") {
            sendJson(response, 200, {
              format: "workspace.tree/1",
              entries: [
                { path: "src", kind: "directory", size: null, sha256: null },
                {
                  path: "src/main.rs",
                  kind: "file",
                  size: Buffer.byteLength(reviewCodingSource),
                  sha256: "b".repeat(64),
                },
                { path: "Cargo.toml", kind: "file", size: 162, sha256: "c".repeat(64) },
                { path: "README.md", kind: "file", size: 940, sha256: "d".repeat(64) },
              ],
              truncated: true,
              omitted: 27,
            });
            return;
          }
          if (
            path === `/api/project-sessions/${reviewCodingSession.id}/files/src/main.rs` &&
            (method === "GET" || method === "PUT")
          ) {
            const draft = method === "PUT" ? (await readJson(request)).content : undefined;
            const content = typeof draft === "string" ? draft : reviewCodingSource;
            sendJson(response, 200, {
              format: "workspace.file/1",
              revision: {
                path: "src/main.rs",
                sha256: method === "PUT" ? "e".repeat(64) : "b".repeat(64),
                size: Buffer.byteLength(content),
                language: "rust",
                modification: "modified",
              },
              content,
              binary: false,
              truncated: false,
            });
            return;
          }
          if (
            path === `/api/project-sessions/${reviewCodingSession.id}/diff` &&
            method === "POST"
          ) {
            const submitted = await readJson(request);
            sendJson(response, 200, {
              format: "workspace.diff/1",
              selector: submitted.selector ?? { kind: "workspace" },
              mode: submitted.mode ?? "patch",
              digest: "f".repeat(64),
              source_revision: reviewCodingSession.source_revision,
              files: [
                {
                  old_path: "src/main.rs",
                  new_path: "src/main.rs",
                  status: "modified",
                  additions: 2,
                  deletions: 1,
                  old_sha256: "1".repeat(64),
                  new_sha256: "b".repeat(64),
                  attribution: ["workspace", "review-engineer"],
                  hunks:
                    submitted.mode === "patch"
                      ? [
                          {
                            id: "review-hunk-runtime-boundary",
                            old: { start: 3, lines: 1 },
                            new: { start: 3, lines: 2 },
                            heading: "fn main() -> ExitCode",
                            lines: [
                              {
                                kind: "deletion",
                                old_line: 3,
                                new_line: null,
                                content: '    println!("hello");',
                              },
                              {
                                kind: "addition",
                                old_line: null,
                                new_line: 3,
                                content:
                                  '    println!("DevCenter owns the human surface; Workspace owns files and diffs.");',
                              },
                              {
                                kind: "addition",
                                old_line: null,
                                new_line: 4,
                                content: "    ExitCode::SUCCESS",
                              },
                            ],
                          },
                        ]
                      : [],
                },
              ],
              additions: 2,
              deletions: 1,
              partial: false,
            });
            return;
          }
          if (
            path === `/api/project-sessions/${reviewCodingSession.id}/terminal-profiles` &&
            method === "GET"
          ) {
            sendJson(response, 200, [reviewTerminalProfile]);
            return;
          }
          if (
            path === `/api/project-sessions/${reviewCodingSession.id}/terminals` &&
            method === "GET"
          ) {
            sendJson(response, 200, reviewTerminals);
            return;
          }
          if (
            path === `/api/project-sessions/${reviewCodingSession.id}/terminals` &&
            method === "POST"
          ) {
            if (reviewTerminalUpstream) {
              sendJson(response, 200, reviewTerminals[0]);
              return;
            }
            const submitted = await readJson(request);
            if (
              submitted.profile_id !== reviewTerminalProfile.id ||
              typeof submitted.idempotency_key !== "string"
            ) {
              sendJson(response, 422, { code: "workspace_terminal_request_invalid" });
              return;
            }
            const now = Date.now();
            const created = {
              ...initialReviewTerminal,
              id: `terminal-review-${String(reviewTerminals.length + 1)}`,
              agentide_session_id: reviewAgentIdeSessionId,
              authority_grant_id: ensureReviewTerminalGrant(),
              process_id: `substrate-process-review-${String(reviewTerminals.length + 1)}`,
              state: "running" as const,
              created_at_ms: now,
              updated_at_ms: now,
            };
            reviewTerminals.push(created);
            sendJson(response, 201, created);
            return;
          }
          const reviewTerminalMatch = path.match(/^\/api\/project-terminals\/([^/]+)$/);
          if (reviewTerminalMatch && method === "GET") {
            const terminal = reviewTerminals.find(
              (candidate) => candidate.id === reviewTerminalMatch[1],
            );
            sendJson(
              response,
              terminal ? 200 : 404,
              terminal ?? { code: "workspace_terminal_not_found" },
            );
            return;
          }
          if (reviewTerminalMatch && method === "DELETE") {
            const terminal = reviewTerminals.find(
              (candidate) => candidate.id === reviewTerminalMatch[1],
            );
            if (!terminal) {
              sendJson(response, 404, { code: "workspace_terminal_not_found" });
              return;
            }
            terminal.state = "terminated";
            terminal.exit = { code: null, signal: "SIGKILL" };
            terminal.updated_at_ms = Date.now();
            sendJson(response, 200, terminal);
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
          if (path === "/api/capabilities" && method === "GET") {
            sendJson(response, 200, reviewCapabilities);
            return;
          }
          if (path === "/api/capability-profiles" && method === "GET") {
            sendJson(response, 200, capabilityProfiles);
            return;
          }
          if (path === "/api/capability-profiles" && method === "POST") {
            const submitted = await readJson(request);
            const now = Date.now();
            const created: ReviewCapabilityProfile = {
              id: `profile-review-${String(nextCapabilityProfile++)}`,
              name: typeof submitted.name === "string" ? submitted.name : "Engineering default",
              audience: submitted.audience === "tenant" ? "tenant" : "personal",
              revision: 1,
              mappings: Array.isArray(submitted.mappings)
                ? (submitted.mappings as ReviewCapabilityMapping[])
                : [],
              created_by: "review-engineer",
              created_at_ms: now,
              updated_at_ms: now,
            };
            capabilityProfiles.unshift(created);
            sendJson(response, 201, created);
            return;
          }
          const capabilityProfileMatch = path.match(/^\/api\/capability-profiles\/([^/]+)$/);
          if (capabilityProfileMatch && method === "PATCH") {
            const current = capabilityProfiles.find(
              (profile) => profile.id === capabilityProfileMatch[1],
            );
            if (!current) {
              sendJson(response, 404, { code: "capability_profile_not_found" });
              return;
            }
            const submitted = await readJson(request);
            if (submitted.expected_revision !== current.revision) {
              sendJson(response, 409, { code: "capability_profile_revision_conflict" });
              return;
            }
            if (typeof submitted.name === "string") current.name = submitted.name;
            if (Array.isArray(submitted.mappings)) {
              current.mappings = submitted.mappings as ReviewCapabilityMapping[];
            }
            current.revision += 1;
            current.updated_at_ms = Date.now();
            sendJson(response, 200, current);
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
            const serviceInput: Record<string, unknown> =
              submitted.input &&
              typeof submitted.input === "object" &&
              !Array.isArray(submitted.input)
                ? { ...submitted.input }
                : {};
            if (submitted.operation_ref === "agentide.list_sessions") {
              sendJson(response, 200, {
                output: {
                  items: [reviewCoordinationView().session],
                  through_version: reviewAgentIdeVersion,
                  next_cursor: null,
                  partial: false,
                },
                connector_audit_ref: "audit:review:agentide:sessions",
              });
              return;
            }
            if (
              submitted.operation_ref === "agentide.list_grants" ||
              submitted.operation_ref === "agentide.list_context_pins" ||
              submitted.operation_ref === "agentide.list_approval_checkpoints"
            ) {
              const items =
                submitted.operation_ref === "agentide.list_grants"
                  ? reviewAgentIdeGrants
                  : submitted.operation_ref === "agentide.list_context_pins"
                    ? reviewAgentIdePins
                    : [];
              sendJson(response, 200, {
                output: {
                  items,
                  through_version: reviewAgentIdeVersion,
                  next_cursor: null,
                  partial: false,
                },
                connector_audit_ref: `audit:review:${submitted.operation_ref}`,
              });
              return;
            }
            if (
              (submitted.operation_ref === "agentide.start_session" ||
                submitted.operation_ref === "agentide.ensure_hosted_session") &&
              submitted.confirmed === true
            ) {
              sendJson(response, 200, {
                output: {
                  outcome: "ensured",
                  events: [
                    {
                      name: "agentide.session.SessionStarted",
                      fields: { session_id: reviewAgentIdeSessionId },
                    },
                  ],
                  through_version: reviewAgentIdeVersion,
                  replayed: false,
                },
                connector_audit_ref: "audit:review:agentide:start",
              });
              return;
            }
            if (
              submitted.operation_ref === "agentide.create_grant" &&
              submitted.confirmed === true
            ) {
              reviewAgentIdeVersion += 1;
              const grantId = `grant-review-${String(reviewAgentIdeGrants.length + 1)}`;
              reviewAgentIdeGrants.push({
                grant_id: grantId,
                grantee: serviceInput.grantee,
                allowed_intents: serviceInput.allowed_intents,
                path_prefixes: serviceInput.path_prefixes,
                maximum_risk: serviceInput.maximum_risk,
                expires_at: null,
                revision: 1,
                state: "Active",
              });
              sendJson(response, 200, {
                output: {
                  outcome: "created",
                  events: [
                    {
                      name: "agentide.coordination.GrantCreated",
                      fields: { grant_id: grantId },
                    },
                  ],
                  through_version: reviewAgentIdeVersion,
                  replayed: false,
                },
                connector_audit_ref: "audit:review:agentide:grant",
              });
              return;
            }
            if (
              submitted.operation_ref === "agentide.pin_context" &&
              submitted.confirmed === true
            ) {
              reviewAgentIdeVersion += 1;
              reviewAgentIdePins.push({
                pin_id: `pin-review-${String(reviewAgentIdePins.length + 1)}`,
                kind: serviceInput.kind,
                reference: serviceInput.reference,
                start_line: serviceInput.start_line,
                end_line: serviceInput.end_line,
                sha256: serviceInput.sha256,
                state: "Active",
              });
              sendJson(response, 200, {
                output: {
                  outcome: "pinned",
                  events: [{ name: "agentide.coordination.ContextPinned", fields: {} }],
                  through_version: reviewAgentIdeVersion,
                  replayed: false,
                },
                connector_audit_ref: "audit:review:agentide:pin",
              });
              return;
            }
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
          if (/^\/api\/agents\/[^/]+\/tasks$/.test(path) && method === "GET") {
            const agentId = path.split("/")[3];
            sendJson(
              response,
              200,
              reviewTasks.filter((task) => task.agent_id === agentId),
            );
            return;
          }
          if (/^\/api\/agents\/[^/]+\/tasks$/.test(path) && method === "POST") {
            const submitted = await readJson(request);
            const task: ReviewTask = {
              id: `task-review-${String(Date.now())}`,
              agent_id: path.split("/")[3],
              status: "accepted",
              attempt_id: "attempt-review",
              prompt: typeof submitted.prompt === "string" ? submitted.prompt : "",
              output: null,
              failure_code: null,
              failure_message: null,
              accepted_at_ms: Date.now(),
              completed_at_ms: null,
            };
            reviewTasks.push(task);
            sendJson(response, 202, task);
            return;
          }
          const codingTurnMatch = path.match(
            /^\/api\/project-sessions\/([^/]+)\/agents\/([^/]+)\/turns$/,
          );
          if (codingTurnMatch && method === "POST") {
            const submitted = await readJson(request);
            if (
              codingTurnMatch[1] !== reviewCodingSession.id ||
              typeof submitted.prompt !== "string"
            ) {
              sendJson(response, 422, { code: "coding_turn_invalid" });
              return;
            }
            const task: ReviewTask = {
              id: `task-review-coding-${String(Date.now())}`,
              agent_id: codingTurnMatch[2],
              status: "accepted",
              attempt_id: "attempt-review-coding",
              prompt: submitted.prompt,
              output: null,
              failure_code: null,
              failure_message: null,
              accepted_at_ms: Date.now(),
              completed_at_ms: null,
              workspace_session_id: reviewCodingSession.id,
              agentide_session_id: reviewAgentIdeSessionId,
            };
            reviewTasks.push(task);
            sendJson(response, 202, task);
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
