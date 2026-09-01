import type { IncomingMessage, ServerResponse } from "node:http";
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

let connected = false;
let nextAgent = 3;
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
    else if (chunk instanceof Uint8Array) body += decoder.decode(chunk, { stream: true });
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
