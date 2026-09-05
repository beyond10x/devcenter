import { rendererEventFormat } from "@b10x/agentide-ui/protocol";
import type {
  Change,
  ChatMessage,
  Pane,
  RendererEvent,
  TreeProjection,
} from "@b10x/agentide-ui/protocol";
import {
  WorkbenchRefusal,
  type ChangeSet,
  type FileResult,
  type WorkbenchHostPort,
  type WorkbenchSnapshot,
} from "@b10x/agentide-ui/controller";
import type { RendererHandle } from "@b10x/agentide-ui/protocol";
import {
  ApiError,
  api,
  errorMessage,
  type AgentIdeCheckpointSnapshot,
  type AgentIdeWorkbenchAction,
  type AgentIdeWorkbenchPane,
  type AgentIdeWorkbenchPaneKind,
  type AgentIdeWorkbenchView,
  type CodingCoordinationView,
  type CodingSession,
  type DiffFile,
  type Project,
  type TaskApproval,
  type TerminalSession,
} from "@/api/client";
import type { useWorkspaceStore } from "@/stores/workspace";

type WorkspaceStore = ReturnType<typeof useWorkspaceStore>;
type ApprovalTarget =
  | { kind: "checkpoint"; checkpoint: AgentIdeCheckpointSnapshot }
  | { kind: "task"; approval: TaskApproval; agentId: string };

type TerminalLifecycle =
  | { kind: "attached"; replayComplete: boolean }
  | { kind: "exit"; detail: string }
  | { kind: "refused"; detail: string }
  | { kind: "detached"; detail: string };

const terminalReconnectMaximum = 8_000;

/**
 * Devcenter's product boundary for the framework-neutral AgentIDE controller.
 *
 * This class owns URLs, sessions, task streams, and terminal sockets. AgentIDE receives only
 * typed projections and semantic effects; neither its controller nor its renderers know the BFF.
 */
export class DevcenterWorkbenchHost implements WorkbenchHostPort {
  readonly #projectId: string;
  readonly #sessionId: string;
  readonly #workspace: WorkspaceStore;
  readonly #panes: Pane[] = [{ id: "chat", kind: "chat", title: "Agent" }];
  readonly #openFiles = new Map<string, FileResult>();
  readonly #terminalSockets = new Map<string, WebSocket>();
  readonly #terminalSequences = new Map<string, bigint>();
  readonly #terminalDeliveries = new Map<string, number>();
  readonly #terminalReconnects = new Map<string, number>();
  readonly #terminalReconnectTimers = new Map<string, number>();
  readonly #terminalSizes = new Map<string, { columns: number; rows: number }>();
  readonly #stoppedTerminals = new Set<string>();
  readonly #approvalTargets = new Map<string, ApprovalTarget>();
  #focusedPane = "chat";
  #project?: Project;
  #coordination?: CodingCoordinationView;
  #changeSet?: ChangeSet;
  #renderer?: RendererHandle;
  #requestRefresh?: () => void;
  #resumed = false;
  #workbenchHydrated = false;
  #terminalsHydrated = false;
  #destroyed = false;

  constructor(projectId: string, sessionId: string, workspace: WorkspaceStore) {
    this.#projectId = projectId;
    this.#sessionId = sessionId;
    this.#workspace = workspace;
  }

  attachRenderer(renderer: RendererHandle, requestRefresh: () => void): void {
    this.#renderer = renderer;
    this.#requestRefresh = requestRefresh;
  }

  async snapshot(signal: AbortSignal): Promise<WorkbenchSnapshot> {
    this.#throwIfAborted(signal);
    const [project, session] = await Promise.all([
      this.#project ? Promise.resolve(this.#project) : api.project(this.#projectId),
      api.codingSession(this.#sessionId),
    ]);
    this.#throwIfAborted(signal);
    if (session.project_id !== project.id || project.id !== this.#projectId) {
      throw new WorkbenchRefusal(
        "devcenter.workspace_route_refused",
        "The coding session does not belong to this project route.",
      );
    }
    this.#project = project;

    let coordinationFailure = "";
    if (session.state === "ready") {
      try {
        if (!this.#resumed) {
          const resumed = await api.resumeCodingSession(this.#sessionId);
          this.#resumed = true;
          if (resumed.coordination?.state !== "ready") {
            coordinationFailure =
              resumed.coordination?.failure_code ??
              "Agent coordination is temporarily unavailable.";
          }
        }
        if (!coordinationFailure) {
          this.#coordination = await api.codingCoordination(this.#sessionId);
        }
      } catch (caught) {
        coordinationFailure = errorMessage(caught);
      }
    }
    if (session.state === "ready" && !coordinationFailure) {
      try {
        if (!this.#workbenchHydrated) await this.#hydrateWorkbench(signal);
        if (!this.#terminalsHydrated) await this.#hydrateTerminals(signal);
      } catch (caught) {
        coordinationFailure = errorMessage(caught);
      }
    }
    this.#throwIfAborted(signal);

    const agentId = this.#selectedAgentId();
    const tasks = agentId
      ? this.#workspace
          .historyFor(agentId)
          .filter((task) => task.workspace_session_id === this.#sessionId)
      : [];
    const run = agentId ? this.#workspace.runFor(agentId) : undefined;
    const currentTaskIds = new Set(tasks.map((task) => task.id));
    const messages: ChatMessage[] = tasks.flatMap((task) => {
      const at = new Date(task.accepted_at_ms).toISOString();
      const assistant = task.output ?? (task.id === run?.taskId ? run.output : "");
      const result: ChatMessage[] = [
        {
          id: `${task.id}:user`,
          role: "user",
          markdown: task.prompt,
          state: "complete",
          created_at: at,
        },
      ];
      if (assistant || task.status === "failed") {
        result.push({
          id: `${task.id}:assistant`,
          role: "assistant",
          markdown: assistant || task.failure_message || "The agent turn failed.",
          state: task.status === "failed" ? "failed" : "complete",
          created_at: new Date(task.completed_at_ms ?? task.accepted_at_ms).toISOString(),
        });
      }
      return result;
    });

    this.#approvalTargets.clear();
    const pendingApprovals: WorkbenchSnapshot["pending_approvals"] = [];
    for (const checkpoint of this.#coordination?.checkpoints ?? []) {
      if (checkpoint.state !== "Pending") continue;
      const digest = `checkpoint:${checkpoint.plan_digest}`;
      this.#approvalTargets.set(digest, { kind: "checkpoint", checkpoint });
      pendingApprovals.push({
        digest,
        intent: checkpoint.attempt_ref,
        risk: "exact AgentIDE plan",
        approval_required: true,
      });
    }
    for (const approval of taskApprovalsForCurrentCodingSession(currentTaskIds, run)) {
      if (!agentId) continue;
      const digest = `task:${approval.id}`;
      this.#approvalTargets.set(digest, { kind: "task", approval, agentId });
      pendingApprovals.push({
        digest,
        intent: approval.operation_ref,
        risk: "Connector operation",
        approval_required: true,
      });
    }

    const activeTerminal = this.#panes.find((pane) => pane.kind === "terminal");
    const projections: WorkbenchSnapshot["projections"] = {
      chat: { kind: "chat", messages },
    };
    for (const [path, file] of this.#openFiles) {
      projections[editorPane(path)] = {
        kind: "editor",
        document: { ...file, dirty: false },
      };
    }
    if (this.#changeSet && this.#panes.some((pane) => pane.id === "diff")) {
      projections.diff = { kind: "diff", ...this.#changeSet };
    }
    if (activeTerminal) {
      const terminalId = activeTerminal.id.slice("terminal:".length);
      const terminal = await this.#terminal(terminalId);
      if (terminal.state === "preparing" || terminal.state === "running") {
        this.#stoppedTerminals.delete(terminalId);
      } else {
        this.#stoppedTerminals.add(terminalId);
      }
      const size = this.#terminalSizes.get(terminalId) ?? { columns: 100, rows: 28 };
      projections[activeTerminal.id] = {
        kind: "terminal",
        terminal_id: terminal.id,
        state: terminalState(terminal),
        ...size,
      };
    }

    return {
      session: {
        id: session.id,
        objective: `${project.path_with_namespace} · ${project.selected_branch} · ${this.#selectedAgentName()}`,
        status: sessionState(session),
        cursor: Math.max(0, tasks.length + (this.#coordination?.summary.through_version ?? 0)),
      },
      panes: [...this.#panes],
      focused_pane: this.#focusedPane,
      open_files: [...this.#openFiles.keys()],
      pending_approvals: pendingApprovals,
      context_pins: (this.#coordination?.pins ?? [])
        .filter((pin) => pin.state === "Active")
        .map((pin) => ({ id: pin.pin_id, label: pin.kind, source: pin.reference })),
      grants: (this.#coordination?.grants ?? []).map((grant) => ({
        id: grant.grant_id,
        capability: grant.allowed_intents.join(", "),
        state: grant.state === "Active" ? "active" : "revoked",
      })),
      activity: tasks.map((task, index) => ({
        sequence: index + 1,
        at: new Date(task.accepted_at_ms).toISOString(),
        kind: `agent.${task.status}`,
        intent: task.prompt,
      })),
      preparation:
        session.state !== "ready"
          ? {
              stage: session.state,
              message: session.failure_code ?? `Workspace materialization is ${session.state}.`,
              retryable: session.state === "preparing" || session.state === "unknown",
            }
          : coordinationFailure
            ? { stage: "coordination", message: coordinationFailure, retryable: true }
            : undefined,
      projections,
    };
  }

  async tree(
    path: string,
    cursor: string | undefined,
    signal: AbortSignal,
  ): Promise<TreeProjection> {
    this.#throwIfAborted(signal);
    const page = await api.codingTree(this.#sessionId, path, cursor, 500);
    return {
      kind: "tree",
      root: page.root,
      entries: page.entries.map((entry) => ({
        path: entry.path,
        name: entry.path.split("/").filter(Boolean).at(-1) ?? entry.path,
        kind:
          entry.kind === "directory" || entry.kind === "tree"
            ? "directory"
            : entry.kind === "symlink"
              ? "symlink"
              : "file",
      })),
      next_cursor: page.next_cursor ?? undefined,
    };
  }

  async openFile(path: string, signal: AbortSignal): Promise<FileResult> {
    this.#throwIfAborted(signal);
    const observed = await api.codingFile(this.#sessionId, path);
    const file = fileResult(observed);
    const paneId = editorPane(path);
    const panes = [...this.#panes];
    if (!panes.some((pane) => pane.id === paneId)) {
      panes.push({
        id: paneId,
        kind: "editor",
        title: path.split("/").at(-1) ?? path,
        path,
        line: 1,
        column: 1,
      });
    }
    await this.#mutateWorkbench(
      { kind: "open_file", path, line: 1, pane_id: paneId },
      panes,
      paneId,
      [...new Set([...this.#openFiles.keys(), path])],
      signal,
    );
    this.#openFiles.set(path, file);
    return file;
  }

  async saveFile(
    path: string,
    content: string,
    version: string,
    signal: AbortSignal,
  ): Promise<FileResult> {
    this.#throwIfAborted(signal);
    try {
      const saved = fileResult(await api.saveCodingFile(this.#sessionId, path, content, version));
      this.#openFiles.set(path, saved);
      return saved;
    } catch (caught) {
      if (caught instanceof ApiError && caught.status === 409) {
        throw new WorkbenchRefusal(
          "devcenter.editor_version_conflict",
          "The file changed after it was opened. Refresh it before applying your draft.",
          true,
        );
      }
      throw caught;
    }
  }

  async focusPane(paneId: string, signal: AbortSignal): Promise<void> {
    this.#throwIfAborted(signal);
    if (!this.#panes.some((pane) => pane.id === paneId)) {
      throw new WorkbenchRefusal("devcenter.pane_absent", "That workbench pane is no longer open.");
    }
    await this.#mutateWorkbench(
      { kind: "focus_pane", pane_id: paneId },
      [...this.#panes],
      paneId,
      [...this.#openFiles.keys()],
      signal,
    );
    if (paneId.startsWith("terminal:")) {
      const terminalId = paneId.slice("terminal:".length);
      window.setTimeout(() => this.#connectTerminal(terminalId), 0);
    }
  }

  async moveCursor(
    paneId: string,
    path: string,
    line: number,
    column: number,
    signal: AbortSignal,
  ): Promise<void> {
    this.#throwIfAborted(signal);
    const panes = this.#panes.map((pane) =>
      pane.id === paneId ? { ...pane, path, line, column } : pane,
    );
    if (!panes.some((pane) => pane.id === paneId && pane.path === path)) {
      throw new WorkbenchRefusal("devcenter.pane_absent", "That editor pane is no longer open.");
    }
    await this.#mutateWorkbench(
      { kind: "move_cursor", pane_id: paneId, path, line, column },
      panes,
      this.#focusedPane,
      [...this.#openFiles.keys()],
      signal,
    );
  }

  async closePane(paneId: string, signal: AbortSignal): Promise<void> {
    this.#throwIfAborted(signal);
    if (paneId === "chat") return;
    const panes = [...this.#panes];
    const index = panes.findIndex((pane) => pane.id === paneId);
    if (index < 0) return;
    const [pane] = panes.splice(index, 1);
    const openFiles = [...this.#openFiles.keys()].filter((path) => path !== pane?.path);
    const focusedPane =
      this.#focusedPane === paneId ? (panes.at(-1)?.id ?? "chat") : this.#focusedPane;
    await this.#mutateWorkbench(
      pane?.kind === "editor" && pane.path
        ? { kind: "close_file", path: pane.path }
        : { kind: "close_pane", pane_id: paneId },
      panes,
      focusedPane,
      openFiles,
      signal,
    );
    if (pane?.path) this.#openFiles.delete(pane.path);
    if (pane?.kind === "terminal") this.#disconnectTerminal(pane.id.slice("terminal:".length));
  }

  async changes(signal: AbortSignal): Promise<ChangeSet> {
    this.#throwIfAborted(signal);
    const observed = await api.codingDiff(this.#sessionId, { kind: "workspace" }, "patch");
    const changeSet = {
      baseline_commit: observed.source_revision,
      changes: observed.files.map(change),
      truncated: observed.partial,
    };
    const panes = [...this.#panes];
    if (!panes.some((pane) => pane.id === "diff")) {
      panes.push({ id: "diff", kind: "diff", title: "Changes" });
    }
    await this.#mutateWorkbench(
      { kind: "show_diff", pane_id: "diff", base: observed.source_revision },
      panes,
      "diff",
      [...this.#openFiles.keys()],
      signal,
    );
    this.#changeSet = changeSet;
    return this.#changeSet;
  }

  async submitPrompt(
    content: string,
    onDelta: (markdownDelta: string) => void,
    signal: AbortSignal,
  ): Promise<ChatMessage> {
    const agentId = this.#selectedAgentId();
    if (!agentId) {
      throw new WorkbenchRefusal(
        "devcenter.agent_absent",
        "Create or select an agent before starting a coding turn.",
      );
    }
    const history = this.#workspace
      .historyFor(agentId)
      .filter((task) => task.workspace_session_id === this.#sessionId);
    const created = await this.#workspace.submitCodingTurn(this.#sessionId, agentId, {
      prompt: content,
      messages: history
        .filter((task) => task.status === "succeeded" && task.output)
        .slice(-10)
        .flatMap((task) => [
          { role: "user" as const, content: task.prompt },
          { role: "assistant" as const, content: task.output ?? "" },
        ]),
      focused_selections: [],
      open_files: [...this.#openFiles.values()].map((file) => ({
        path: file.path,
        sha256: file.version,
        cursor: null,
        dirty: false,
      })),
      active_diff: this.#focusedPane === "diff" ? { kind: "workspace" } : null,
      idempotency_key: crypto.randomUUID(),
    });
    if (!created) {
      throw new WorkbenchRefusal(
        "devcenter.agent_turn_refused",
        this.#workspace.runFor(agentId).error || "The coding turn was refused.",
        true,
      );
    }

    let delivered = "";
    for (;;) {
      this.#throwIfAborted(signal);
      const run = this.#workspace.runFor(agentId);
      if (run.taskId === created.id && run.output.startsWith(delivered)) {
        const delta = run.output.slice(delivered.length);
        if (delta) onDelta(delta);
        delivered = run.output;
      }
      if (run.taskId === created.id && run.status === "succeeded") {
        const markdown = run.output;
        if (markdown.startsWith(delivered)) {
          const delta = markdown.slice(delivered.length);
          if (delta) onDelta(delta);
        }
        return {
          id: `${created.id}:assistant`,
          role: "assistant",
          markdown,
          state: "complete",
          created_at: new Date().toISOString(),
        };
      }
      if (run.taskId === created.id && run.status === "failed") {
        throw new WorkbenchRefusal(
          "devcenter.agent_turn_failed",
          run.error || "The coding turn failed.",
          true,
        );
      }
      await abortableDelay(50, signal);
    }
  }

  async approve(planDigest: string, signal: AbortSignal): Promise<void> {
    await this.#decide(planDigest, "approve", signal);
  }

  async deny(planDigest: string, signal: AbortSignal): Promise<void> {
    await this.#decide(planDigest, "deny", signal);
  }

  async pinContext(source: string, signal: AbortSignal): Promise<void> {
    this.#throwIfAborted(signal);
    const content = this.#openFiles.get(source)?.content;
    if (content === undefined) {
      throw new WorkbenchRefusal(
        "devcenter.context_source_unobserved",
        "Open the file before pinning it to this session.",
      );
    }
    const digest = await sha256(content);
    this.#coordination = await api.pinCodingContext(this.#sessionId, {
      kind: "Editor",
      reference: source,
      sha256: digest,
      idempotency_key: crypto.randomUUID(),
    });
  }

  async removeContextPin(pinId: string, signal: AbortSignal): Promise<void> {
    this.#throwIfAborted(signal);
    this.#coordination = await api.removeCodingContextPin(this.#sessionId, pinId);
  }

  async openTerminal(columns: number, rows: number, signal: AbortSignal): Promise<string> {
    this.#throwIfAborted(signal);
    const profiles = await api.terminalProfiles(this.#sessionId);
    const profile = profiles[0];
    if (!profile) {
      throw new WorkbenchRefusal(
        "devcenter.terminal_profile_absent",
        "No deployment-admitted terminal profile is available.",
      );
    }
    const terminal = await api.createTerminal(this.#sessionId, {
      profile_id: profile.id,
      columns,
      rows,
      idempotency_key: crypto.randomUUID(),
    });
    const paneId = `terminal:${terminal.id}`;
    const panes = [...this.#panes, { id: paneId, kind: "terminal" as const, title: profile.label }];
    try {
      await this.#mutateWorkbench(
        { kind: "open_pane", pane_id: paneId, pane_kind: "Terminal", split: "Vertical" },
        panes,
        paneId,
        [...this.#openFiles.keys()],
        signal,
      );
    } catch (caught) {
      await api.terminateTerminal(terminal.id).catch(() => undefined);
      throw caught;
    }
    this.#terminalSizes.set(terminal.id, { columns, rows });
    this.#stoppedTerminals.delete(terminal.id);
    window.setTimeout(() => this.#connectTerminal(terminal.id), 0);
    return terminal.id;
  }

  terminalInput(terminalId: string, data: string, signal: AbortSignal): Promise<void> {
    this.#throwIfAborted(signal);
    const socket = this.#terminalSockets.get(terminalId);
    if (socket?.readyState !== WebSocket.OPEN) {
      throw new WorkbenchRefusal(
        "devcenter.terminal_detached",
        "The terminal transport is reconnecting.",
        true,
      );
    }
    socket.send(new TextEncoder().encode(data));
    return Promise.resolve();
  }

  terminalResize(
    terminalId: string,
    columns: number,
    rows: number,
    signal: AbortSignal,
  ): Promise<void> {
    this.#throwIfAborted(signal);
    this.#terminalSizes.set(terminalId, { columns, rows });
    const socket = this.#terminalSockets.get(terminalId);
    if (socket?.readyState === WebSocket.OPEN) {
      socket.send(JSON.stringify({ kind: "resize", columns, rows }));
    }
    return Promise.resolve();
  }

  destroy(): void {
    this.#destroyed = true;
    for (const timer of this.#terminalReconnectTimers.values()) window.clearTimeout(timer);
    this.#terminalReconnectTimers.clear();
    for (const terminalId of this.#terminalSockets.keys()) this.#disconnectTerminal(terminalId);
    this.#renderer = undefined;
    this.#requestRefresh = undefined;
  }

  async #hydrateWorkbench(signal: AbortSignal): Promise<void> {
    let workbench: AgentIdeWorkbenchView;
    try {
      workbench = await api.codingWorkbench(this.#sessionId);
    } catch (caught) {
      if (!(caught instanceof ApiError) || caught.status !== 404) throw caught;
      workbench = await api.mutateCodingWorkbench(this.#sessionId, {
        action: { kind: "initialize" },
        panes: [workbenchPane({ id: "chat", kind: "chat", title: "Agent" })],
        focused_pane: "chat",
        open_files: [],
        idempotency_key: crypto.randomUUID(),
      });
    }
    this.#throwIfAborted(signal);
    this.#applyWorkbench(workbench);
    const files = await Promise.all(
      workbench.open_files.map(async (path) =>
        fileResult(await api.codingFile(this.#sessionId, path)),
      ),
    );
    this.#throwIfAborted(signal);
    for (const file of files) this.#openFiles.set(file.path, file);
    this.#workbenchHydrated = true;
  }

  async #hydrateTerminals(signal: AbortSignal): Promise<void> {
    const terminals = await api.terminals(this.#sessionId);
    this.#throwIfAborted(signal);
    const resumable = [...terminals]
      .reverse()
      .find((terminal) => terminal.state === "preparing" || terminal.state === "running");
    const stale = this.#panes.filter(
      (pane) => pane.kind === "terminal" && pane.id !== `terminal:${resumable?.id ?? ""}`,
    );
    if (stale.length > 0) {
      const panes = this.#panes.filter((pane) => !stale.includes(pane));
      const focused = panes.some((pane) => pane.id === this.#focusedPane)
        ? this.#focusedPane
        : (panes.at(-1)?.id ?? "chat");
      await this.#mutateWorkbench(
        { kind: "close_pane", pane_id: stale[0]?.id ?? "terminal:stale" },
        panes,
        focused,
        [...this.#openFiles.keys()],
        signal,
      );
    }
    if (resumable) {
      const paneId = `terminal:${resumable.id}`;
      if (!this.#panes.some((pane) => pane.id === paneId)) {
        await this.#mutateWorkbench(
          { kind: "open_pane", pane_id: paneId, pane_kind: "Terminal", split: "Vertical" },
          [...this.#panes, { id: paneId, kind: "terminal", title: resumable.profile.label }],
          this.#focusedPane,
          [...this.#openFiles.keys()],
          signal,
        );
      }
      this.#terminalSizes.set(resumable.id, { columns: 100, rows: 28 });
    }
    this.#terminalsHydrated = true;
  }

  async #mutateWorkbench(
    action: AgentIdeWorkbenchAction,
    panes: Pane[],
    focusedPane: string | undefined,
    openFiles: string[],
    signal: AbortSignal,
  ): Promise<void> {
    this.#throwIfAborted(signal);
    const workbench = await api.mutateCodingWorkbench(this.#sessionId, {
      action,
      panes: panes.map(workbenchPane),
      focused_pane: focusedPane,
      open_files: openFiles,
      idempotency_key: crypto.randomUUID(),
    });
    this.#throwIfAborted(signal);
    this.#applyWorkbench(workbench);
  }

  #applyWorkbench(workbench: AgentIdeWorkbenchView): void {
    const panes = workbench.panes.map(rendererPane);
    this.#panes.splice(0, this.#panes.length, ...panes);
    this.#focusedPane = workbench.focused_pane ?? panes[0]?.id ?? "chat";
    const open = new Set(workbench.open_files);
    for (const path of this.#openFiles.keys()) {
      if (!open.has(path)) this.#openFiles.delete(path);
    }
  }

  async #decide(digest: string, decision: "approve" | "deny", signal: AbortSignal): Promise<void> {
    this.#throwIfAborted(signal);
    const target = this.#approvalTargets.get(digest);
    if (!target) {
      throw new WorkbenchRefusal(
        "devcenter.approval_stale",
        "That approval is no longer pending.",
        true,
      );
    }
    if (target.kind === "checkpoint") {
      this.#coordination = await api.decideCodingCheckpoint(
        this.#sessionId,
        target.checkpoint.checkpoint_id,
        decision,
      );
    } else {
      await this.#workspace.resolveTaskApproval(target.agentId, target.approval.id, decision);
    }
  }

  #selectedAgentId(): string | undefined {
    const current = this.#workspace.selectedAgentId;
    if (current) return current;
    const first = this.#workspace.agents[0]?.id;
    if (first) this.#workspace.selectAgent(first);
    return first;
  }

  #selectedAgentName(): string {
    const id = this.#selectedAgentId();
    return this.#workspace.agents.find((agent) => agent.id === id)?.name ?? "No agent selected";
  }

  async #terminal(terminalId: string): Promise<TerminalSession> {
    return api.terminal(terminalId);
  }

  #connectTerminal(terminalId: string): void {
    if (
      this.#destroyed ||
      this.#stoppedTerminals.has(terminalId) ||
      this.#terminalSockets.has(terminalId) ||
      !this.#panes.some((pane) => pane.id === `terminal:${terminalId}`)
    ) {
      return;
    }
    const reconnect = this.#terminalReconnectTimers.get(terminalId);
    if (reconnect !== undefined) window.clearTimeout(reconnect);
    this.#terminalReconnectTimers.delete(terminalId);
    const last = this.#terminalSequences.get(terminalId);
    const socket = new WebSocket(api.terminalSocketUrl(terminalId, last));
    socket.binaryType = "arraybuffer";
    this.#terminalSockets.set(terminalId, socket);
    socket.onopen = () => {
      this.#terminalReconnects.set(terminalId, 0);
      const size = this.#terminalSizes.get(terminalId) ?? { columns: 100, rows: 28 };
      socket.send(JSON.stringify({ kind: "resize", ...size }));
    };
    socket.onmessage = (event: MessageEvent<ArrayBuffer | string>) => {
      if (typeof event.data === "string") {
        this.#receiveTerminalLifecycle(terminalId, socket, event.data);
        return;
      }
      if (event.data.byteLength < 8) {
        this.#renderer?.deliver({
          format: rendererEventFormat,
          kind: "notice",
          message: "The terminal returned an invalid output frame.",
        });
        return;
      }
      const sequence = new DataView(event.data).getBigUint64(0, false);
      const previous = this.#terminalSequences.get(terminalId) ?? 0n;
      if (sequence <= previous) return;
      this.#terminalSequences.set(terminalId, sequence);
      const deliverySequence = (this.#terminalDeliveries.get(terminalId) ?? 0) + 1;
      this.#terminalDeliveries.set(terminalId, deliverySequence);
      this.#renderer?.deliver({
        format: rendererEventFormat,
        kind: "terminal_output",
        terminal_id: terminalId,
        sequence: deliverySequence,
        bytes: new Uint8Array(event.data, 8),
      } satisfies RendererEvent);
    };
    socket.onclose = () => {
      if (this.#terminalSockets.get(terminalId) !== socket) return;
      this.#terminalSockets.delete(terminalId);
      this.#scheduleTerminalReconnect(terminalId);
    };
  }

  #receiveTerminalLifecycle(terminalId: string, socket: WebSocket, payload: string): void {
    const lifecycle = parseTerminalLifecycle(payload);
    if (!lifecycle) {
      this.#stoppedTerminals.add(terminalId);
      this.#notice("The terminal returned an invalid lifecycle frame.");
      socket.close();
      return;
    }
    if (lifecycle.kind === "attached") {
      if (!lifecycle.replayComplete) {
        this.#notice("Earlier terminal output is outside the bounded replay window.");
      }
      return;
    }
    this.#notice(lifecycle.detail);
    if (lifecycle.kind === "detached") return;
    this.#stoppedTerminals.add(terminalId);
    this.#requestRefresh?.();
  }

  #scheduleTerminalReconnect(terminalId: string): void {
    if (
      this.#destroyed ||
      this.#stoppedTerminals.has(terminalId) ||
      this.#terminalReconnectTimers.has(terminalId) ||
      !this.#panes.some((pane) => pane.id === `terminal:${terminalId}`)
    ) {
      return;
    }
    const attempt = (this.#terminalReconnects.get(terminalId) ?? 0) + 1;
    this.#terminalReconnects.set(terminalId, attempt);
    const timer = window.setTimeout(
      () => {
        this.#terminalReconnectTimers.delete(terminalId);
        void this.#reconnectTerminal(terminalId);
      },
      Math.min(terminalReconnectMaximum, 500 * 2 ** Math.min(attempt - 1, 4)),
    );
    this.#terminalReconnectTimers.set(terminalId, timer);
  }

  async #reconnectTerminal(terminalId: string): Promise<void> {
    if (this.#destroyed || this.#stoppedTerminals.has(terminalId)) return;
    try {
      const terminal = await api.terminal(terminalId);
      if (this.#terminalReconnectStopped(terminalId)) return;
      if (terminal.state === "preparing" || terminal.state === "running") {
        this.#connectTerminal(terminalId);
        return;
      }
      this.#stoppedTerminals.add(terminalId);
      this.#notice(terminalExitDetail(terminal));
      this.#requestRefresh?.();
    } catch {
      this.#scheduleTerminalReconnect(terminalId);
    }
  }

  #terminalReconnectStopped(terminalId: string): boolean {
    return this.#destroyed || this.#stoppedTerminals.has(terminalId);
  }

  #notice(message: string): void {
    this.#renderer?.deliver({
      format: rendererEventFormat,
      kind: "notice",
      message,
    });
  }

  #disconnectTerminal(terminalId: string): void {
    const reconnect = this.#terminalReconnectTimers.get(terminalId);
    if (reconnect !== undefined) window.clearTimeout(reconnect);
    this.#terminalReconnectTimers.delete(terminalId);
    const socket = this.#terminalSockets.get(terminalId);
    this.#terminalSockets.delete(terminalId);
    this.#terminalDeliveries.delete(terminalId);
    this.#terminalReconnects.delete(terminalId);
    this.#terminalSequences.delete(terminalId);
    this.#terminalSizes.delete(terminalId);
    this.#stoppedTerminals.delete(terminalId);
    socket?.close();
  }

  #throwIfAborted(signal: AbortSignal): void {
    if (signal.aborted || this.#destroyed) throw new DOMException("Aborted", "AbortError");
  }
}

function editorPane(path: string): string {
  return `editor:${path}`;
}

export function taskApprovalsForCurrentCodingSession(
  taskIds: ReadonlySet<string>,
  run: { taskId?: string; approvals?: TaskApproval[] } | undefined,
): TaskApproval[] {
  return run?.taskId && taskIds.has(run.taskId) ? (run.approvals ?? []) : [];
}

function workbenchPane(pane: Pane): AgentIdeWorkbenchPane {
  return {
    id: pane.id,
    kind: workbenchPaneKind(pane.kind),
    title: pane.title,
    path: pane.path ?? null,
    line: pane.line ?? null,
    column: pane.column ?? null,
  };
}

function rendererPane(pane: AgentIdeWorkbenchPane): Pane {
  return {
    id: pane.id,
    kind: pane.kind.toLowerCase() as Pane["kind"],
    title: pane.title,
    path: pane.path ?? undefined,
    line: pane.line ?? undefined,
    column: pane.column ?? undefined,
  };
}

function workbenchPaneKind(kind: Pane["kind"]): AgentIdeWorkbenchPaneKind {
  return `${kind[0]?.toUpperCase() ?? ""}${kind.slice(1)}` as AgentIdeWorkbenchPaneKind;
}

function fileResult(observed: Awaited<ReturnType<typeof api.codingFile>>): FileResult {
  return {
    path: observed.revision.path,
    language: observed.revision.language ?? "plaintext",
    content: observed.content ?? "",
    version: observed.revision.sha256,
    read_only: observed.binary || observed.truncated,
  };
}

function change(file: DiffFile): Change {
  const path = file.new_path ?? file.old_path ?? "unknown";
  const status: Change["status"] =
    file.status === "added" ||
    file.status === "modified" ||
    file.status === "deleted" ||
    file.status === "renamed" ||
    file.status === "untracked"
      ? file.status
      : file.status === "copied"
        ? "renamed"
        : "modified";
  const patch = file.hunks
    .map((hunk) => {
      const heading = hunk.heading ? ` ${hunk.heading}` : "";
      const header = `@@ -${String(hunk.old.start)},${String(hunk.old.lines)} +${String(hunk.new.start)},${String(hunk.new.lines)} @@${heading}`;
      return [
        header,
        ...hunk.lines.map(
          (line) =>
            `${line.kind === "addition" ? "+" : line.kind === "deletion" ? "-" : " "}${line.content}`,
        ),
      ].join("\n");
    })
    .join("\n");
  return { path, status, patch: patch || undefined };
}

function sessionState(session: CodingSession): WorkbenchSnapshot["session"]["status"] {
  switch (session.state) {
    case "preparing":
    case "unknown":
      return "preparing";
    case "ready":
      return "active";
    case "closed":
      return "completed";
    case "closing":
      return "superseded";
    case "refused":
      return "failed";
  }
}

function terminalState(terminal: TerminalSession): "opening" | "open" | "closed" | "failed" {
  if (terminal.state === "preparing") return "opening";
  if (terminal.state === "running") return "open";
  if (terminal.state === "exited" || terminal.state === "terminated") return "closed";
  return "failed";
}

function parseTerminalLifecycle(payload: string): TerminalLifecycle | undefined {
  let value: unknown;
  try {
    value = JSON.parse(payload);
  } catch {
    return undefined;
  }
  if (!value || typeof value !== "object") return undefined;
  const lifecycle = value as Record<string, unknown>;
  if (lifecycle.kind === "attached") {
    const replay = lifecycle.replay;
    if (!replay || typeof replay !== "object") return undefined;
    const complete = (replay as Record<string, unknown>).complete;
    return typeof complete === "boolean"
      ? { kind: "attached", replayComplete: complete }
      : undefined;
  }
  if (lifecycle.kind === "exit") {
    return { kind: "exit", detail: terminalExitDetail(lifecycle) };
  }
  if (lifecycle.kind === "refused" || lifecycle.kind === "detached") {
    const code = lifecycle.code;
    if (typeof code !== "string" || !code) return undefined;
    return {
      kind: lifecycle.kind,
      detail:
        lifecycle.kind === "refused"
          ? `The terminal was refused (${code}).`
          : `The terminal transport detached (${code}); reconnecting.`,
    };
  }
  return undefined;
}

function terminalExitDetail(value: TerminalSession | Record<string, unknown>): string {
  const exit = value.exit;
  if (exit && typeof exit === "object") {
    const observed = exit as Record<string, unknown>;
    if (typeof observed.signal === "string") return `The terminal ended by ${observed.signal}.`;
    if (typeof observed.code === "number") {
      return `The terminal exited with code ${String(observed.code)}.`;
    }
  }
  return "The terminal exited.";
}

async function sha256(content: string): Promise<string> {
  const digest = await crypto.subtle.digest("SHA-256", new TextEncoder().encode(content));
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}

function abortableDelay(milliseconds: number, signal: AbortSignal): Promise<void> {
  return new Promise((resolve, reject) => {
    const aborted = () => {
      window.clearTimeout(timer);
      reject(new DOMException("Aborted", "AbortError"));
    };
    const timer = window.setTimeout(() => {
      signal.removeEventListener("abort", aborted);
      resolve();
    }, milliseconds);
    signal.addEventListener("abort", aborted, { once: true });
  });
}
