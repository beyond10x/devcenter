import { createPinia, setActivePinia } from "pinia";
import { HttpResponse, http } from "msw";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { WorkbenchController } from "@b10x/agentide-ui/controller";
import { rendererActionFormat } from "@b10x/agentide-ui/protocol";
import type { RendererHandle } from "@b10x/agentide-ui/protocol";
import {
  DevcenterWorkbenchHost,
  taskApprovalsForCurrentCodingSession,
  type StartupProgress,
} from "@/features/workbench/devcenterWorkbenchHost";
import {
  api,
  ApiError,
  type AgentIdeWorkbenchView,
  type CodingSession,
  type FileProjection,
} from "@/api/client";
import { useWorkspaceStore } from "@/stores/workspace";
import { server } from "./setup";

const session: CodingSession = {
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
    http.get("/api/project-sessions/session-1/tree", () =>
      HttpResponse.json({
        root: "",
        entries: [{ path: "src/lib.rs", kind: "file" }],
        next_cursor: null,
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
    vi.restoreAllMocks();
    vi.unstubAllGlobals();
    vi.useRealTimers();
  });

  it("restores the actor's active terminal as a resumable AgentIDE pane", async () => {
    useWorkspaceHandlers();
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());

    await host.snapshot(new AbortController().signal);
    await vi.waitFor(async () =>
      expect(
        (await host.snapshot(new AbortController().signal)).projections?.["terminal:terminal-1"],
      ).toMatchObject({ kind: "terminal" }),
    );
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
    await vi.waitFor(async () =>
      expect(
        (await host.snapshot(new AbortController().signal)).projections?.["terminal:terminal-1"],
      ).toMatchObject({ kind: "terminal" }),
    );
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

    await host.snapshot(new AbortController().signal);
    await vi.waitFor(async () =>
      expect(
        (await host.snapshot(new AbortController().signal)).projections?.["editor:src/lib.rs"],
      ).toMatchObject({ kind: "editor" }),
    );
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

    await vi.waitFor(() =>
      expect(handlers.actions.map((action) => action.kind)).toEqual([
        "open_pane",
        "focus_pane",
        "move_cursor",
        "close_file",
      ]),
    );
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

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (reason: unknown) => void;
  const promise = new Promise<T>((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

function savedWorkbench(paths: string[], focused = paths[0]): AgentIdeWorkbenchView {
  return {
    session_id: session.id,
    panes: [
      { id: "chat", kind: "Chat", title: "Agent" },
      ...paths.map((path) => ({
        id: `editor:${path}`,
        kind: "Editor" as const,
        title: path,
        path,
      })),
    ],
    focused_pane: focused ? `editor:${focused}` : "chat",
    open_files: paths,
    through_version: 4,
  };
}

function sourceFile(path: string, content = "source"): FileProjection {
  return {
    format: "workspace.file/1",
    revision: {
      path,
      sha256: "c".repeat(64),
      size: content.length,
      language: "rust",
      modification: "unchanged",
    },
    content,
    binary: false,
    truncated: false,
  };
}

describe("progressive workspace startup", () => {
  beforeEach(() => {
    setActivePinia(createPinia());
    useWorkspaceHandlers();
  });
  afterEach(() => {
    vi.restoreAllMocks();
    vi.useRealTimers();
  });

  it("makes the tree and editable file available before any ancillary API responds", async () => {
    const resume = deferred<CodingSession>();
    const layout = deferred<AgentIdeWorkbenchView>();
    const terminals = deferred<Awaited<ReturnType<typeof api.terminals>>>();
    vi.spyOn(api, "resumeCodingSession").mockReturnValue(resume.promise);
    vi.spyOn(api, "codingWorkbench").mockReturnValue(layout.promise);
    vi.spyOn(api, "terminals").mockReturnValue(terminals.promise);
    vi.spyOn(api, "codingFile").mockImplementation((_id, path) =>
      Promise.resolve(sourceFile(path)),
    );
    const mutation = vi.spyOn(api, "mutateCodingWorkbench");
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    const controller = new WorkbenchController(host);
    await controller.start();
    await vi.waitFor(async () =>
      expect((await host.snapshot(new AbortController().signal)).tree?.entries).toHaveLength(1),
    );
    await controller.dispatch({
      format: rendererActionFormat,
      kind: "open_file",
      path: "src/lib.rs",
    });
    expect(controller.frame()?.workbench.projections["editor:src/lib.rs"]).toMatchObject({
      kind: "editor",
      document: { read_only: false, content: "source" },
    });
    expect(mutation).not.toHaveBeenCalled();
    controller.destroy();
    host.destroy();
  });

  it("loads the focused file first and restores the rest with at most four requests in flight", async () => {
    const paths = Array.from({ length: 8 }, (_, i) => `src/file${String(i)}.rs`);
    const focused = "src/file6.rs";
    vi.spyOn(api, "codingWorkbench").mockResolvedValue(savedWorkbench(paths, focused));
    const reads = new Map<string, ReturnType<typeof deferred<FileProjection>>>();
    let active = 0;
    let maximum = 0;
    vi.spyOn(api, "codingFile").mockImplementation((_id, path) => {
      const read = deferred<FileProjection>();
      reads.set(path, read);
      active += 1;
      maximum = Math.max(maximum, active);
      return read.promise.finally(() => {
        active -= 1;
      });
    });
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    await host.snapshot(new AbortController().signal);
    await vi.waitFor(() => expect([...reads.keys()]).toEqual([focused]));
    reads.get(focused)?.resolve(sourceFile(focused));
    await vi.waitFor(() => expect(reads.size).toBe(5));
    expect(
      (await host.snapshot(new AbortController().signal)).projections?.[`editor:${focused}`],
    ).toMatchObject({ kind: "editor" });
    for (const [path, read] of reads) if (path !== focused) read.resolve(sourceFile(path));
    await vi.waitFor(() => expect(reads.size).toBe(8));
    for (const [path, read] of reads) read.resolve(sourceFile(path));
    expect(maximum).toBe(4);
    host.destroy();
  });

  it("merges delayed layout without losing a dirty draft, changing focus, or reopening a closed tab", async () => {
    const layout = deferred<AgentIdeWorkbenchView>();
    vi.spyOn(api, "codingWorkbench").mockReturnValue(layout.promise);
    vi.spyOn(api, "codingFile").mockImplementation((_id, path) =>
      Promise.resolve(sourceFile(path)),
    );
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    const controller = new WorkbenchController(host);
    await controller.start();
    await controller.dispatch({
      format: rendererActionFormat,
      kind: "open_file",
      path: "closed.rs",
    });
    await controller.dispatch({
      format: rendererActionFormat,
      kind: "close_pane",
      pane_id: "editor:closed.rs",
    });
    await controller.dispatch({
      format: rendererActionFormat,
      kind: "open_file",
      path: "draft.rs",
    });
    await controller.dispatch({
      format: rendererActionFormat,
      kind: "edit_file",
      path: "draft.rs",
      content: "unsaved draft",
      version: "c".repeat(64),
    });
    layout.resolve(savedWorkbench(["closed.rs", "draft.rs", "saved.rs"], "closed.rs"));
    await vi.waitFor(async () =>
      expect((await host.snapshot(new AbortController().signal)).open_files).toContain("saved.rs"),
    );
    await controller.refresh();
    const frame = controller.frame();
    expect(frame?.workbench.focused_pane).toBe("editor:draft.rs");
    expect(frame?.workbench.panes.some((pane) => pane.id === "editor:closed.rs")).toBe(false);
    expect(frame?.workbench.projections["editor:draft.rs"]).toMatchObject({
      kind: "editor",
      document: { content: "unsaved draft", dirty: true },
    });
    controller.destroy();
    host.destroy();
  });

  it("does not restore a closed file from a late read and isolates another file's failure", async () => {
    vi.spyOn(api, "codingWorkbench").mockResolvedValue(
      savedWorkbench(["closed.rs", "failed.rs", "good.rs"]),
    );
    const closed = deferred<FileProjection>();
    vi.spyOn(api, "codingFile").mockImplementation(async (_id, path) => {
      if (path === "closed.rs") return closed.promise;
      if (path === "failed.rs") throw new ApiError(404, "file_not_found");
      return sourceFile(path);
    });
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    await host.snapshot(new AbortController().signal);
    await vi.waitFor(() =>
      expect(api.codingFile).toHaveBeenCalledWith(
        "session-1",
        "closed.rs",
        expect.any(AbortSignal),
      ),
    );
    await host.closePane("editor:closed.rs", new AbortController().signal);
    closed.resolve(sourceFile("closed.rs"));
    await vi.waitFor(async () =>
      expect((await host.snapshot(new AbortController().signal)).open_files).toContain("good.rs"),
    );
    const frame = await host.snapshot(new AbortController().signal);
    expect(frame.open_files).not.toContain("closed.rs");
    expect(frame.projections?.["editor:failed.rs"]).toMatchObject({
      kind: "refusal",
      retryable: true,
    });
    host.destroy();
  });

  it("polls preparing sessions without overlapping requests and loads the root as soon as ready", async () => {
    vi.useFakeTimers();
    const preparation = deferred<CodingSession>();
    const observe = vi
      .spyOn(api, "codingSession")
      .mockResolvedValueOnce({ ...session, state: "preparing" })
      .mockReturnValueOnce(preparation.promise);
    const tree = vi.spyOn(api, "codingTree");
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    expect((await host.snapshot(new AbortController().signal)).session.status).toBe("preparing");
    expect(tree).not.toHaveBeenCalled();
    await vi.advanceTimersByTimeAsync(1_000);
    expect(observe).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(10_000);
    expect(observe).toHaveBeenCalledTimes(2);
    preparation.resolve(session);
    await vi.waitFor(() => expect(tree).toHaveBeenCalledOnce());
    expect((await host.snapshot(new AbortController().signal)).session.status).toBe("active");
    await vi.advanceTimersByTimeAsync(10_000);
    expect(observe).toHaveBeenCalledTimes(2);
    host.destroy();
  });

  it("backs off transient preparation failures, stops after refusal, and cancels on destroy", async () => {
    vi.useFakeTimers();
    const observe = vi
      .spyOn(api, "codingSession")
      .mockResolvedValueOnce({ ...session, state: "preparing" })
      .mockRejectedValueOnce(new ApiError(503, "workspace_unavailable"))
      .mockRejectedValueOnce(new ApiError(403, "workspace_access_refused"));
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    await host.snapshot(new AbortController().signal);
    await vi.advanceTimersByTimeAsync(1_000);
    await vi.waitFor(() => expect(observe).toHaveBeenCalledTimes(2));
    await vi.advanceTimersByTimeAsync(1_000);
    expect(observe).toHaveBeenCalledTimes(2);
    await vi.advanceTimersByTimeAsync(1_000);
    expect(observe).toHaveBeenCalledTimes(3);
    await vi.advanceTimersByTimeAsync(20_000);
    expect(observe).toHaveBeenCalledTimes(3);
    await expect(host.openFile("blocked.rs", new AbortController().signal)).rejects.toMatchObject({
      status: 403,
    });
    host.destroy();
    expect(observe.mock.calls[0]?.[1]?.aborted).toBe(true);
  });

  it("persists pane actions serially without applying old responses over newer local actions", async () => {
    vi.spyOn(api, "codingWorkbench").mockResolvedValue(savedWorkbench([]));
    vi.spyOn(api, "terminals").mockResolvedValue([]);
    vi.spyOn(api, "codingFile").mockImplementation((_id, path) =>
      Promise.resolve(sourceFile(path)),
    );
    const first = deferred<undefined>();
    let inFlight = 0;
    let maximum = 0;
    const writes = vi.spyOn(api, "mutateCodingWorkbench").mockImplementation(async (_id, input) => {
      inFlight += 1;
      maximum = Math.max(maximum, inFlight);
      if (writes.mock.calls.length === 1) await first.promise;
      if (input.action.kind === "open_file") expect(input.open_files).toContain(input.action.path);
      if (input.action.kind === "close_file")
        expect(input.open_files).not.toContain(input.action.path);
      if (input.action.kind === "focus_pane") expect(input.focused_pane).toBe(input.action.pane_id);
      inFlight -= 1;
      return { ...input, session_id: session.id, through_version: 4 + writes.mock.calls.length };
    });
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    const signal = new AbortController().signal;
    await host.snapshot(signal);
    await host.openFile("old.rs", signal);
    await host.closePane("editor:old.rs", signal);
    await host.openFile("new.rs", signal);
    await host.focusPane("chat", signal);
    await vi.waitFor(() => expect(writes).toHaveBeenCalledTimes(1));
    first.resolve(undefined);
    await vi.waitFor(() => expect(writes).toHaveBeenCalledTimes(4));
    const snapshot = await host.snapshot(signal);
    expect(maximum).toBe(1);
    expect(writes.mock.calls.map((call) => call[1].action.kind)).toEqual([
      "open_file",
      "close_file",
      "open_file",
      "focus_pane",
    ]);
    expect(writes.mock.calls.at(-1)?.[1].open_files).toEqual(["new.rs"]);
    expect(snapshot.open_files).toEqual(["new.rs"]);
    expect(snapshot.focused_pane).toBe("chat");
    host.destroy();
  });

  it("ignores a stale layout refresh that arrives after a mutation acknowledgement", async () => {
    const original = savedWorkbench([]);
    const staleRead = deferred<AgentIdeWorkbenchView>();
    const acknowledgement = deferred<undefined>();
    const layouts = vi
      .spyOn(api, "codingWorkbench")
      .mockResolvedValueOnce(original)
      .mockReturnValueOnce(staleRead.promise);
    vi.spyOn(api, "terminals").mockResolvedValue([]);
    vi.spyOn(api, "codingFile").mockImplementation((_id, path) =>
      Promise.resolve(sourceFile(path)),
    );
    let completed = 0;
    const writes = vi.spyOn(api, "mutateCodingWorkbench").mockImplementation(async (_id, input) => {
      if (writes.mock.calls.length === 1) await acknowledgement.promise;
      completed += 1;
      return { ...input, session_id: session.id, through_version: 4 + completed };
    });
    let progress: StartupProgress[] = [];
    const host = new DevcenterWorkbenchHost(
      "project-1",
      "session-1",
      useWorkspaceStore(),
      (next) => {
        progress = next;
      },
    );
    const signal = new AbortController().signal;
    await host.snapshot(signal);
    await vi.waitFor(() =>
      expect(progress.find((part) => part.part === "workbench")?.state).toBe("ready"),
    );
    await host.openFile("acknowledged.rs", signal);
    await vi.waitFor(() => expect(writes).toHaveBeenCalledOnce());
    host.retryStartup();
    await vi.waitFor(() => expect(layouts).toHaveBeenCalledTimes(2));
    acknowledgement.resolve(undefined);
    await vi.waitFor(() => expect(completed).toBe(1));
    staleRead.resolve(original);
    await vi.waitFor(() =>
      expect(progress.find((part) => part.part === "workbench")?.state).toBe("ready"),
    );
    await host.openFile("next.rs", signal);
    await vi.waitFor(() => expect(writes).toHaveBeenCalledTimes(2));
    expect(writes.mock.calls[1]?.[1].open_files).toEqual(["acknowledged.rs", "next.rs"]);
    expect((await host.snapshot(signal)).open_files).toEqual(["acknowledged.rs", "next.rs"]);
    host.destroy();
  });

  it("cancels the sibling request when validation fails", async () => {
    const sessionRead = deferred<CodingSession>();
    vi.spyOn(api, "project").mockRejectedValue(new ApiError(503, "workspace_unavailable"));
    const observe = vi.spyOn(api, "codingSession").mockReturnValue(sessionRead.promise);
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    await expect(host.snapshot(new AbortController().signal)).rejects.toMatchObject({
      status: 503,
    });
    expect(observe.mock.calls[0]?.[1]?.aborted).toBe(true);
    host.destroy();
  });

  it("rejects a mismatched route before starting background requests", async () => {
    vi.spyOn(api, "codingSession").mockResolvedValue({
      ...session,
      project_id: "other-project",
    });
    const tree = vi.spyOn(api, "codingTree");
    const resume = vi.spyOn(api, "resumeCodingSession");
    const host = new DevcenterWorkbenchHost("project-1", "session-1", useWorkspaceStore());
    await expect(host.snapshot(new AbortController().signal)).rejects.toMatchObject({
      code: "devcenter.workspace_route_refused",
    });
    expect(tree).not.toHaveBeenCalled();
    expect(resume).not.toHaveBeenCalled();
    host.destroy();
  });
});
