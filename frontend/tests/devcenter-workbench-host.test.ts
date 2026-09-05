import { createPinia, setActivePinia } from "pinia";
import { HttpResponse, http } from "msw";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import type { RendererHandle } from "@b10x/agentide-ui/protocol";
import {
  DevcenterWorkbenchHost,
  taskApprovalsForCurrentCodingSession,
} from "@/features/workbench/devcenterWorkbenchHost";
import type { AgentIdeWorkbenchView } from "@/api/client";
import { useWorkspaceStore } from "@/stores/workspace";
import { server } from "./setup";

const session = {
  id: "session-1",
  project_id: "project-1",
  source_revision: "a".repeat(40),
  materialization_ref: "substrate:git:one",
  manifest_sha256: "b".repeat(64),
  state: "ready",
  failure_code: null,
  limits: { max_files: 100, max_total_bytes: 1_000_000, max_file_bytes: 100_000 },
  created_at_ms: 1,
  updated_at_ms: 1,
};

const terminal = {
  id: "terminal-1",
  coding_session_id: session.id,
  agentide_session_id: session.id,
  authority_grant_id: "grant-1",
  profile: {
    id: "shell",
    label: "Workspace shell",
    runtime_ref: "runtime",
    shell: "/bin/sh",
    arguments: [],
    working_directory: "/workspace",
    environment: {},
    workspace_access: "read_write",
    network: "none",
    limits: {
      timeout_ms: 60_000,
      cpu_millis: 100,
      memory_bytes: 1_000_000,
      processes: 4,
      output_bytes: 1_000_000,
      input_bytes: 1_000_000,
      frame_bytes: 65_536,
      queued_frames: 16,
      lease_ttl_ms: 60_000,
    },
  },
  actor: "human:one",
  process_id: "process-1",
  state: "running",
  exit: null,
  failure_code: null,
  created_at_ms: 2,
  updated_at_ms: 2,
};

class FakeWebSocket {
  static readonly OPEN = 1;
  static instances: FakeWebSocket[] = [];
  binaryType = "blob";
  readyState = 0;
  onopen?: () => void;
  onmessage?: (event: MessageEvent<ArrayBuffer | string>) => void;
  onclose?: () => void;
  readonly send = vi.fn();

  constructor(readonly url: string) {
    FakeWebSocket.instances.push(this);
  }

  close() {
    this.readyState = 3;
    this.onclose?.();
  }

  receive(payload: string) {
    this.onmessage?.(new MessageEvent("message", { data: payload }));
  }
}

function useWorkspaceHandlers() {
  let workbench: AgentIdeWorkbenchView = {
    session_id: session.id,
    panes: [
      {
        id: "chat",
        kind: "Chat",
        title: "Agent",
        path: null,
        line: null,
        column: null,
      },
      {
        id: "terminal:terminal-1",
        kind: "Terminal",
        title: "Workspace shell",
        path: null,
        line: null,
        column: null,
      },
    ],
    focused_pane: "chat",
    open_files: [] as string[],
    through_version: 2,
  };
  const actions: Array<Record<string, unknown>> = [];
  server.use(
    http.get("/api/projects/project-1", () =>
      HttpResponse.json({
        id: "project-1",
        forge_instance_ref: "gitlab",
        project_ref: "project",
        path_with_namespace: "foundation/devcenter",
        name: "devcenter",
        default_branch: "trunk",
        selected_branch: "trunk",
        pinned_commit: session.source_revision,
        web_url: "https://git.example.invalid/foundation/devcenter",
      }),
    ),
    http.get("/api/project-sessions/session-1", () => HttpResponse.json(session)),
    http.post("/api/project-sessions/session-1/resume", () =>
      HttpResponse.json({
        ...session,
        coordination: { state: "ready", through_version: 1, failure_code: null, retryable: false },
      }),
    ),
    http.get("/api/project-sessions/session-1/coordination", () =>
      HttpResponse.json({
        summary: { state: "ready", through_version: 1, failure_code: null, retryable: false },
        session: {},
        grants: [],
        pins: [],
        checkpoints: [],
      }),
    ),
    http.get("/api/project-sessions/session-1/workbench", () => HttpResponse.json(workbench)),
    http.post("/api/project-sessions/session-1/workbench", async ({ request }) => {
      const mutation = (await request.json()) as typeof workbench & {
        action: Record<string, unknown>;
      };
      actions.push(mutation.action);
      workbench = {
        session_id: session.id,
        panes: mutation.panes,
        focused_pane: mutation.focused_pane,
        open_files: mutation.open_files,
        through_version: workbench.through_version + 1,
      };
      return HttpResponse.json(workbench);
    }),
    http.get("/api/project-sessions/session-1/terminals", () => HttpResponse.json([terminal])),
    http.get("/api/project-terminals/terminal-1", () => HttpResponse.json(terminal)),
  );
  return { actions, setWorkbench: (next: typeof workbench) => (workbench = next) };
}

describe("Devcenter AgentIDE host", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    FakeWebSocket.instances = [];
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("restores the actor's active terminal as a resumable AgentIDE pane", async () => {
    useWorkspaceHandlers();
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());

    const snapshot = await host.snapshot(new AbortController().signal);

    expect(snapshot.panes).toContainEqual({
      id: "terminal:terminal-1",
      kind: "terminal",
      title: "Workspace shell",
    });
    expect(snapshot.projections?.["terminal:terminal-1"]).toMatchObject({
      kind: "terminal",
      terminal_id: "terminal-1",
      state: "open",
    });
    host.destroy();
  });

  it("stops reconnecting after an authoritative terminal exit lifecycle", async () => {
    vi.useFakeTimers();
    vi.stubGlobal("WebSocket", FakeWebSocket);
    useWorkspaceHandlers();
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    const deliver = vi.fn();
    const refresh = vi.fn();
    const renderer: RendererHandle = { update: vi.fn(), deliver, destroy: vi.fn() };
    await host.snapshot(new AbortController().signal);
    host.attachRenderer(renderer, refresh);

    await host.focusPane("terminal:terminal-1", new AbortController().signal);
    await vi.runOnlyPendingTimersAsync();
    expect(FakeWebSocket.instances).toHaveLength(1);
    const socket = FakeWebSocket.instances[0];
    socket?.receive(JSON.stringify({ kind: "exit", state: "exited", exit: { code: 0 } }));
    socket?.close();
    await vi.advanceTimersByTimeAsync(30_000);

    expect(deliver).toHaveBeenCalledWith(
      expect.objectContaining({ kind: "notice", message: "The terminal exited with code 0." }),
    );
    expect(refresh).toHaveBeenCalledOnce();
    expect(FakeWebSocket.instances).toHaveLength(1);
    host.destroy();
  });

  it("restores files from durable AgentIDE state and persists focus, cursor, and close actions", async () => {
    const handlers = useWorkspaceHandlers();
    handlers.setWorkbench({
      session_id: session.id,
      panes: [
        { id: "chat", kind: "Chat", title: "Agent", path: null, line: null, column: null },
        {
          id: "editor:src/lib.rs",
          kind: "Editor",
          title: "lib.rs",
          path: "src/lib.rs",
          line: 4,
          column: 2,
        },
      ],
      focused_pane: "editor:src/lib.rs",
      open_files: ["src/lib.rs"],
      through_version: 7,
    });
    server.use(
      http.get("/api/project-sessions/session-1/files/src/lib.rs", () =>
        HttpResponse.json({
          revision: {
            path: "src/lib.rs",
            sha256: "c".repeat(64),
            size: 15,
            language: "rust",
            modification: "unchanged",
          },
          content: "pub fn one() {}",
          binary: false,
          truncated: false,
        }),
      ),
    );
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());

    const snapshot = await host.snapshot(new AbortController().signal);
    expect(snapshot.focused_pane).toBe("editor:src/lib.rs");
    expect(snapshot.projections?.["editor:src/lib.rs"]).toMatchObject({
      kind: "editor",
      document: { path: "src/lib.rs", content: "pub fn one() {}" },
    });

    const signal = new AbortController().signal;
    await host.focusPane("chat", signal);
    await host.moveCursor("editor:src/lib.rs", "src/lib.rs", 8, 3, signal);
    await host.closePane("editor:src/lib.rs", signal);

    expect(handlers.actions.map((action) => action.kind)).toEqual([
      "open_pane",
      "focus_pane",
      "move_cursor",
      "close_file",
    ]);
    host.destroy();
  });

  it("projects task approvals only when their task belongs to the coding session", () => {
    const approval = { id: "approval-1" } as never;
    expect(
      taskApprovalsForCurrentCodingSession(new Set(["task-current"]), {
        taskId: "task-other",
        approvals: [approval],
      }),
    ).toEqual([]);
    expect(
      taskApprovalsForCurrentCodingSession(new Set(["task-current"]), {
        taskId: "task-current",
        approvals: [approval],
      }),
    ).toEqual([approval]);
  });
});
