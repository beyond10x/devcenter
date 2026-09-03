import { createPinia, setActivePinia } from "pinia";
import { HttpResponse, http } from "msw";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { useWorkspaceStore } from "@/stores/workspace";
import { server } from "./setup";

const session = {
  tenant_id: "tenant-a",
  subject: "engineer-1",
  email: "engineer@example.test",
  groups: [],
  connectors_docs_available: false,
  agentide_workspace_enabled: false,
};
const agent = {
  id: "agent-1",
  tenant_id: "tenant-a",
  name: "Release assistant",
  active_revision: 1,
  latest_revision: 1,
  created_by: "engineer-1",
  created_at_ms: 1_788_260_000_000,
};

function defaultHandlers() {
  server.use(
    http.get("/api/session", () => HttpResponse.json(session)),
    http.get("/api/agents", () => HttpResponse.json([agent])),
    http.get("/api/connectors/claude-code", () =>
      HttpResponse.json({ provider: "claude-code", connected: true }),
    ),
  );
}

describe("workspace store", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    defaultHandlers();
  });

  it("bootstraps the verified session and product state", async () => {
    const workspace = useWorkspaceStore();
    await workspace.bootstrap();
    expect(workspace.sessionState).toBe("ready");
    expect(workspace.session?.email).toBe("engineer@example.test");
    expect(workspace.agents).toEqual([agent]);
    expect(workspace.selectedAgentId).toBe("agent-1");
    expect(workspace.connected).toBe(true);
  });

  it("shows sign in only when the session endpoint returns 401", async () => {
    server.use(
      http.get("/api/session", () =>
        HttpResponse.json({ code: "identity_authentication_required" }, { status: 401 }),
      ),
    );
    const workspace = useWorkspaceStore();
    await workspace.bootstrap();
    expect(workspace.sessionState).toBe("idle");
  });

  it("keeps session outages distinct from signed-out state", async () => {
    server.use(
      http.get("/api/session", () =>
        HttpResponse.json({ code: "identity_unavailable" }, { status: 503 }),
      ),
    );
    const workspace = useWorkspaceStore();
    await workspace.bootstrap();
    expect(workspace.sessionState).toBe("error");
    expect(workspace.sessionError).toContain("identity_unavailable");
  });

  it("holds an OAuth flow in memory and clears it on completion", async () => {
    server.use(
      http.post("/api/connectors/claude-code/oauth/start", () =>
        HttpResponse.json({
          authorization_url: "https://provider.example/authorize",
          flow_id: "flow-1",
          expires_at: 1_788_260_900,
        }),
      ),
      http.post("/api/connectors/claude-code/oauth/complete", async ({ request }) => {
        const body = (await request.json()) as { flow_id: string; code: string };
        expect(body).toEqual({ flow_id: "flow-1", code: "one-time-code" });
        return HttpResponse.json({ provider: "claude-code", connected: true });
      }),
    );
    const workspace = useWorkspaceStore();
    await workspace.startOAuth();
    expect(workspace.oauthFlow?.flow_id).toBe("flow-1");
    await workspace.completeOAuth("one-time-code");
    expect(workspace.oauthFlow).toBeUndefined();
    expect(workspace.connected).toBe(true);
  });

  it("creates and selects an activated agent", async () => {
    server.use(
      http.post("/api/agents", async ({ request }) => {
        expect(await request.json()).toEqual({
          name: "Evidence reviewer",
          instructions: "Review the evidence.",
          model: "claude-opus-5",
        });
        return HttpResponse.json(
          { ...agent, id: "agent-2", name: "Evidence reviewer" },
          { status: 201 },
        );
      }),
    );
    const workspace = useWorkspaceStore();
    await workspace.createAgent({
      name: "Evidence reviewer",
      instructions: "Review the evidence.",
      model: "claude-opus-5",
    });
    expect(workspace.selectedAgentId).toBe("agent-2");
    expect(workspace.agents[0]?.name).toBe("Evidence reviewer");
  });

  it("streams ordered task events into per-agent output", async () => {
    class FakeEventSource {
      static readonly CLOSED = 2;
      readonly CLOSED = 2;
      readyState = 1;
      listeners = new Map<string, (event: MessageEvent<string>) => void>();
      onerror: (() => void) | null = null;
      constructor(readonly url: string) {
        queueMicrotask(() => {
          this.listeners.get("task")?.(
            new MessageEvent("task", {
              data: JSON.stringify({ event: { kind: "text_delta", text: "Gate " } }),
            }),
          );
          this.listeners.get("task")?.(
            new MessageEvent("task", {
              data: JSON.stringify({ event: { kind: "succeeded", output: "Gate passed." } }),
            }),
          );
        });
      }
      addEventListener(type: string, listener: EventListener) {
        this.listeners.set(type, listener);
      }
      close() {
        this.readyState = FakeEventSource.CLOSED;
      }
    }
    vi.stubGlobal("EventSource", FakeEventSource);
    server.use(
      http.post("/api/agents/agent-1/tasks", () =>
        HttpResponse.json(
          { id: "task-1", agent_id: "agent-1", status: "accepted", attempt_id: "attempt-1" },
          { status: 202 },
        ),
      ),
    );
    const workspace = useWorkspaceStore();
    workspace.setDraft("agent-1", "Run the gate.");
    await workspace.submitTask("agent-1");
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(workspace.runFor("agent-1")).toMatchObject({
      status: "succeeded",
      output: "Gate passed.",
    });
    expect(workspace.draftFor("agent-1")).toBe("");
    vi.unstubAllGlobals();
  });

  it("submits a workspace-bound coding turn and records refreshed context and tools", async () => {
    class CodingEventSource {
      static readonly CLOSED = 2;
      readonly CLOSED = 2;
      readyState = 1;
      listeners = new Map<string, (event: MessageEvent<string>) => void>();
      onerror: (() => void) | null = null;
      constructor(readonly url: string) {
        queueMicrotask(() => {
          for (const event of [
            { kind: "context_changed", revision: "context-2" },
            {
              kind: "inventory_changed",
              revision: "inventory-2",
              published_tools: ["code_read", "code_edit"],
            },
            { kind: "succeeded", output: "Saved change reviewed." },
          ]) {
            this.listeners.get("task")?.(
              new MessageEvent("task", { data: JSON.stringify({ event }) }),
            );
          }
        });
      }
      addEventListener(type: string, listener: EventListener) {
        this.listeners.set(type, listener);
      }
      close() {
        this.readyState = CodingEventSource.CLOSED;
      }
    }
    vi.stubGlobal("EventSource", CodingEventSource);
    server.use(
      http.post("/api/project-sessions/session-1/agents/agent-1/turns", async ({ request }) => {
        const body = (await request.json()) as Record<string, unknown>;
        expect(body).toMatchObject({
          prompt: "Review the saved change.",
          agentide_session_id: "agentide-1",
          active_diff: { kind: "workspace" },
        });
        expect(body.open_files).toEqual([
          { path: "src/main.rs", sha256: "a".repeat(64), cursor: null, dirty: true },
        ]);
        return HttpResponse.json(
          {
            id: "task-coding-1",
            agent_id: "agent-1",
            status: "accepted",
            attempt_id: "attempt-coding-1",
            prompt: "Review the saved change.",
            accepted_at_ms: 1,
            workspace_session_id: "session-1",
            agentide_session_id: "agentide-1",
          },
          { status: 202 },
        );
      }),
    );
    const workspace = useWorkspaceStore();
    const task = await workspace.submitCodingTurn("session-1", "agent-1", {
      prompt: "Review the saved change.",
      messages: [],
      agentide_session_id: "agentide-1",
      focused_selections: [],
      open_files: [{ path: "src/main.rs", sha256: "a".repeat(64), cursor: null, dirty: true }],
      active_diff: { kind: "workspace" },
      idempotency_key: "coding-turn-1",
    });
    await new Promise((resolve) => setTimeout(resolve, 0));
    expect(task?.workspace_session_id).toBe("session-1");
    expect(workspace.runFor("agent-1")).toMatchObject({
      status: "succeeded",
      output: "Saved change reviewed.",
      contextRevision: "context-2",
      inventoryRevision: "inventory-2",
      publishedTools: ["code_read", "code_edit"],
    });
    vi.unstubAllGlobals();
  });

  it("shows the exact pending call and sends only the human decision", async () => {
    class ApprovalEventSource {
      static readonly CLOSED = 2;
      readonly CLOSED = 2;
      readyState = 1;
      listeners = new Map<string, (event: MessageEvent<string>) => void>();
      onerror: (() => void) | null = null;
      constructor(readonly url: string) {
        queueMicrotask(() => {
          this.listeners.get("task")?.(
            new MessageEvent("task", {
              data: JSON.stringify({
                event: {
                  kind: "approval_requested",
                  approval_id: "approval-1",
                  call_id: "call-1",
                  operation_ref: "todo.item.create",
                  connection_ref: "todo",
                },
              }),
            }),
          );
        });
      }
      addEventListener(type: string, listener: EventListener) {
        this.listeners.set(type, listener);
      }
      close() {
        this.readyState = ApprovalEventSource.CLOSED;
      }
    }
    vi.stubGlobal("EventSource", ApprovalEventSource);
    server.use(
      http.post("/api/agents/agent-1/tasks", () =>
        HttpResponse.json(
          { id: "task-1", agent_id: "agent-1", status: "accepted", attempt_id: "attempt-1" },
          { status: 202 },
        ),
      ),
      http.get("/api/tasks/task-1/approvals", () =>
        HttpResponse.json([
          {
            id: "approval-1",
            task_id: "task-1",
            attempt_id: "attempt-1",
            call_id: "call-1",
            tool_name: "todo_item_create",
            operation_ref: "todo.item.create",
            connection_ref: "todo",
            description_ref: "description-1",
            input: { list_id: "list-1", title: "Ship it" },
            context: {
              tenant_id: "tenant-a",
              agent_id: "agent-1",
              agent_revision: 1,
              authority_snapshot_id: "request-1",
              authority_snapshot_sha256: "a".repeat(64),
            },
            requested_at_ms: 1,
          },
        ]),
      ),
      http.post("/api/tasks/task-1/approvals/approval-1", async ({ request }) => {
        expect(await request.json()).toEqual({ decision: "approve" });
        return HttpResponse.json({
          id: "approval-1",
          task_id: "task-1",
          attempt_id: "attempt-1",
          call_id: "call-1",
          tool_name: "todo_item_create",
          operation_ref: "todo.item.create",
          connection_ref: "todo",
          description_ref: "description-1",
          input: { list_id: "list-1", title: "Ship it" },
          context: {
            tenant_id: "tenant-a",
            agent_id: "agent-1",
            agent_revision: 1,
            authority_snapshot_id: "request-1",
            authority_snapshot_sha256: "a".repeat(64),
          },
          requested_at_ms: 1,
        });
      }),
    );
    const workspace = useWorkspaceStore();
    workspace.setDraft("agent-1", "Add the release Todo.");
    await workspace.submitTask("agent-1");
    await vi.waitFor(() => {
      expect(workspace.runFor("agent-1").approvals?.[0]?.input).toEqual({
        list_id: "list-1",
        title: "Ship it",
      });
    });
    expect(workspace.runFor("agent-1").status).toBe("awaiting_approval");
    await workspace.resolveTaskApproval("agent-1", "approval-1", "approve");
    expect(workspace.runFor("agent-1").approvals).toEqual([]);
    expect(workspace.runFor("agent-1").status).toBe("running");
    vi.unstubAllGlobals();
  });
});
