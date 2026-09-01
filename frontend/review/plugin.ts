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
            });
            return;
          }
          if (path === "/api/agents" && method === "GET") {
            sendJson(response, 200, agents);
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
