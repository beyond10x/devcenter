<script setup lang="ts">
import {
  Bot,
  Braces,
  ChevronRight,
  CircleAlert,
  File,
  FileDiff,
  Folder,
  GitBranch,
  LoaderCircle,
  PanelBottomClose,
  PanelBottomOpen,
  PanelRight,
  RefreshCw,
  Save,
  Search,
  Send,
  ShieldCheck,
  SplitSquareHorizontal,
  SquareTerminal,
  X,
} from "@lucide/vue";
import { computed, onBeforeUnmount, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  ApiError,
  api,
  errorMessage,
  type CodingSession,
  type CodingCoordinationView,
  type CodingTreeEntry,
  type DiffHunk,
  type DiffMode,
  type DiffProjection,
  type FileConflict,
  type FileProjection,
  type Project,
  type TerminalProfile,
  type TerminalSession,
  type AgentIdeSessionSnapshot,
  type AgentIdeGrantSnapshot,
  type AgentIdeContextPinSnapshot,
  type AgentIdeCheckpointSnapshot,
} from "@/api/client";
import { useWorkspaceStore } from "@/stores/workspace";
import CanonicalDiffViewer from "./CanonicalDiffViewer.vue";
import CodeEditor from "./CodeEditor.vue";
import HostedTerminal from "./HostedTerminal.vue";

type CenterPane = "editor" | "diff" | "agent";
type RightPane = "context" | "agents" | "grants" | "approvals";
type LoadState = "loading" | "ready" | "error" | "refused";

interface OpenFile {
  projection: FileProjection;
  draft: string;
  dirty: boolean;
  saving: boolean;
  error: string;
}

interface ContextAttachment {
  id: string;
  kind: "selection" | "diff_hunk" | "terminal_selection";
  label: string;
  detail: string;
  reference: string;
  sha256: string;
  startLine?: number;
  endLine?: number;
}

const route = useRoute();
const router = useRouter();
const workspace = useWorkspaceStore();
const state = ref<LoadState>("loading");
const error = ref("");
const project = ref<Project>();
const session = ref<CodingSession>();
const tree = ref<CodingTreeEntry[]>([]);
const treeTruncated = ref(false);
const treeOmitted = ref<number | null>();
const treeSearch = ref("");
const treeLoading = ref(false);
const openFiles = ref<OpenFile[]>([]);
const activePath = ref<string>();
const splitPath = ref<string>();
const diff = ref<DiffProjection>();
const diffLoading = ref(false);
const diffError = ref("");
const terminalOpen = ref(false);
const terminalHeight = ref(320);
const terminalProfiles = ref<TerminalProfile[]>([]);
const terminals = ref<TerminalSession[]>([]);
const terminalState = ref<"loading" | "ready" | "refused" | "error">("loading");
const terminalError = ref("");
const terminalMutation = ref("");
const selectedTerminalProfileId = ref<string>();
const detachedTerminalIds = ref<Set<string>>(new Set());
const rightPane = ref<RightPane>("context");
const conflict = ref<{ payload: FileConflict; localDraft: string; path: string }>();
const attachments = ref<ContextAttachment[]>([]);
const selectedRange = ref<{ startLine: number; endLine: number; content: string }>();
const agentIdeSession = ref<AgentIdeSessionSnapshot>();
const agentIdeGrants = ref<AgentIdeGrantSnapshot[]>([]);
const agentIdePins = ref<AgentIdeContextPinSnapshot[]>([]);
const agentIdeCheckpoints = ref<AgentIdeCheckpointSnapshot[]>([]);
const coordinationState = ref<"loading" | "ready" | "degraded">("loading");
const coordinationError = ref("");
const coordinationMutation = ref("");
const agentPrompt = ref("");

const projectId = computed(() => String(route.params.projectId ?? ""));
const sessionId = computed(() => String(route.params.sessionId ?? ""));
const activePane = computed<CenterPane>(() => {
  const pane = route.query.pane;
  return pane === "diff" || pane === "agent" ? pane : "editor";
});
const diffMode = computed<DiffMode>(() => {
  const mode = route.query.mode;
  return mode === "stat" || mode === "files_only" ? mode : "patch";
});
const diffLayout = computed<"unified" | "side_by_side">(() =>
  route.query.layout === "side_by_side" ? "side_by_side" : "unified",
);
const activeFile = computed(() =>
  openFiles.value.find((file) => file.projection.revision.path === activePath.value),
);
const splitFile = computed(() =>
  openFiles.value.find((file) => file.projection.revision.path === splitPath.value),
);
const dirtyCount = computed(() => openFiles.value.filter((file) => file.dirty).length);
const fileEntries = computed(() => tree.value.filter((entry) => !isDirectory(entry)));
const selectedAgentId = computed(() =>
  typeof route.query.agent === "string" ? route.query.agent : workspace.selectedAgentId,
);
const selectedAgent = computed(() =>
  workspace.agents.find((candidate) => candidate.id === selectedAgentId.value),
);
const agentRun = computed(() =>
  selectedAgentId.value ? workspace.runFor(selectedAgentId.value) : undefined,
);
const sessionTasks = computed(() => {
  if (!selectedAgentId.value) return [];
  return workspace
    .historyFor(selectedAgentId.value)
    .filter((task) => task.workspace_session_id === sessionId.value);
});
const agentTurnPending = computed(() =>
  ["submitting", "accepted", "running", "awaiting_approval", "reconnecting"].includes(
    agentRun.value?.status ?? "idle",
  ),
);
const visibleTerminals = computed(() =>
  terminals.value.filter((candidate) => !detachedTerminalIds.value.has(candidate.id)),
);
const activeTerminal = computed(() => {
  const requested = typeof route.query.terminal === "string" ? route.query.terminal : undefined;
  return (
    visibleTerminals.value.find((candidate) => candidate.id === requested) ??
    visibleTerminals.value[0]
  );
});
onMounted(() => {
  const storedHeight = Number(window.sessionStorage.getItem("devcenter:workbench-terminal-height"));
  if (Number.isFinite(storedHeight) && storedHeight >= 180) {
    terminalHeight.value = Math.min(window.innerHeight * 0.65, storedHeight);
  }
  terminalOpen.value = typeof route.query.terminal === "string";
  void load();
});
watch([projectId, sessionId], load);
watch(
  () => route.query.file,
  (path) => {
    if (typeof path === "string" && path !== activePath.value) void openFile(path);
  },
);
watch([activePane, diffMode], ([pane]) => {
  if (pane === "diff") void loadDiff();
});
watch(
  selectedAgentId,
  (agentId) => {
    if (agentId) workspace.selectAgent(agentId);
  },
  { immediate: true },
);
let treeTimer: number | undefined;
let stopTerminalResize: (() => void) | undefined;
watch(treeSearch, () => {
  if (treeTimer !== undefined) window.clearTimeout(treeTimer);
  treeTimer = window.setTimeout(() => void loadTree(), 250);
});
onBeforeUnmount(() => stopTerminalResize?.());

async function load() {
  error.value = "";
  if (!workspace.session?.agentide_workspace_enabled) {
    state.value = "refused";
    return;
  }
  state.value = "loading";
  try {
    const [loadedProject, loadedSession] = await Promise.all([
      api.project(projectId.value),
      api.codingSession(sessionId.value),
    ]);
    if (loadedSession.project_id !== loadedProject.id) {
      state.value = "refused";
      error.value = "The session does not belong to this project route.";
      return;
    }
    project.value = loadedProject;
    session.value = loadedSession;
    if (loadedSession.state !== "ready") {
      state.value = loadedSession.state === "refused" ? "refused" : "error";
      error.value = loadedSession.failure_code ?? `Workspace session is ${loadedSession.state}.`;
      return;
    }
    await loadTree();
    const requestedFile =
      typeof route.query.file === "string" ? route.query.file : fileEntries.value[0]?.path;
    if (requestedFile) await openFile(requestedFile, false, activePane.value === "editor");
    if (activePane.value === "diff") await loadDiff();
    state.value = "ready";
    void Promise.allSettled([loadCoordination(), loadTerminals()]);
  } catch (caught) {
    state.value = "error";
    error.value = errorMessage(caught);
  }
}

async function loadCoordination() {
  coordinationState.value = "loading";
  coordinationError.value = "";
  agentIdeSession.value = undefined;
  agentIdeGrants.value = [];
  agentIdePins.value = [];
  agentIdeCheckpoints.value = [];
  try {
    const resumed = await api.resumeCodingSession(sessionId.value);
    session.value = resumed;
    if (resumed.coordination?.state !== "ready") {
      coordinationState.value = "degraded";
      coordinationError.value =
        resumed.coordination?.failure_code ?? "Agent features are temporarily unavailable.";
      return;
    }
    applyCoordinationView(await api.codingCoordination(sessionId.value));
  } catch (caught) {
    coordinationState.value = "degraded";
    coordinationError.value = errorMessage(caught);
  }
}

function applyCoordinationView(view: CodingCoordinationView) {
  agentIdeSession.value = view.session;
  agentIdeGrants.value = view.grants;
  agentIdePins.value = view.pins;
  agentIdeCheckpoints.value = view.checkpoints;
  coordinationState.value = "ready";
  coordinationError.value = "";
}

async function loadTerminals() {
  if (!sessionId.value) return;
  terminalState.value = "loading";
  terminalError.value = "";
  try {
    const [profiles, sessions] = await Promise.all([
      api.terminalProfiles(sessionId.value),
      api.terminals(sessionId.value),
    ]);
    terminalProfiles.value = profiles;
    terminals.value = sessions;
    if (
      !selectedTerminalProfileId.value ||
      !profiles.some((profile) => profile.id === selectedTerminalProfileId.value)
    ) {
      selectedTerminalProfileId.value = profiles[0]?.id;
    }
    terminalState.value = profiles.length ? "ready" : "refused";
    if (activeTerminal.value && typeof route.query.terminal !== "string") {
      await setRoute({ terminal: activeTerminal.value.id });
      terminalOpen.value = true;
    }
  } catch (caught) {
    terminalState.value =
      caught instanceof ApiError && (caught.status === 403 || caught.status === 404)
        ? "refused"
        : "error";
    terminalError.value = errorMessage(caught);
  }
}

async function openTerminal() {
  const profileId = selectedTerminalProfileId.value;
  if (coordinationState.value !== "ready" || !profileId || terminalMutation.value) return;
  terminalError.value = "";
  terminalMutation.value = "create";
  try {
    const created = await api.createTerminal(sessionId.value, {
      profile_id: profileId,
      columns: 100,
      rows: 28,
      idempotency_key: globalThis.crypto.randomUUID(),
    });
    terminals.value = [...terminals.value.filter((item) => item.id !== created.id), created];
    const detached = new Set(detachedTerminalIds.value);
    detached.delete(created.id);
    detachedTerminalIds.value = detached;
    await setRoute({ terminal: created.id });
    terminalOpen.value = true;
    await loadCoordination();
  } catch (caught) {
    terminalError.value = errorMessage(caught);
  } finally {
    terminalMutation.value = "";
  }
}

async function killTerminal(terminalId: string) {
  if (terminalMutation.value) return;
  terminalMutation.value = `kill:${terminalId}`;
  terminalError.value = "";
  try {
    const terminated = await api.terminateTerminal(terminalId);
    terminals.value = terminals.value.map((item) => (item.id === terminalId ? terminated : item));
  } catch (caught) {
    terminalError.value = errorMessage(caught);
  } finally {
    terminalMutation.value = "";
  }
}

async function detachTerminal(terminalId: string) {
  detachedTerminalIds.value = new Set([...detachedTerminalIds.value, terminalId]);
  const next = visibleTerminals.value[0]?.id;
  await setRoute({ terminal: next });
}

function startTerminalResize(event: PointerEvent) {
  event.preventDefault();
  stopTerminalResize?.();
  const startY = event.clientY;
  const startHeight = terminalHeight.value;
  const resize = (moveEvent: PointerEvent) => {
    terminalHeight.value = Math.round(
      Math.min(window.innerHeight * 0.65, Math.max(180, startHeight + startY - moveEvent.clientY)),
    );
    window.sessionStorage.setItem(
      "devcenter:workbench-terminal-height",
      String(terminalHeight.value),
    );
  };
  const stop = () => {
    window.removeEventListener("pointermove", resize);
    window.removeEventListener("pointerup", stop);
    stopTerminalResize = undefined;
  };
  stopTerminalResize = stop;
  window.addEventListener("pointermove", resize);
  window.addEventListener("pointerup", stop);
}

function adjustTerminalHeight(delta: number) {
  terminalHeight.value = Math.round(
    Math.min(window.innerHeight * 0.65, Math.max(180, terminalHeight.value + delta)),
  );
  window.sessionStorage.setItem(
    "devcenter:workbench-terminal-height",
    String(terminalHeight.value),
  );
}

async function attachTerminalSelection(selection: { content: string; terminalId: string }) {
  const attachmentId = globalThis.crypto.randomUUID();
  attachments.value.push({
    id: attachmentId,
    kind: "terminal_selection",
    label: `Terminal ${selection.terminalId.slice(0, 10)} selection`,
    detail: selection.content,
    reference: `terminal/${selection.terminalId}/selection/${attachmentId}`,
    sha256: await sha256(selection.content),
  });
  rightPane.value = "context";
}

async function loadTree() {
  if (!sessionId.value) return;
  treeLoading.value = true;
  try {
    const projection = await api.codingTree(sessionId.value, treeSearch.value, 500);
    tree.value = projection.entries;
    treeTruncated.value = projection.truncated;
    treeOmitted.value = projection.omitted;
  } catch (caught) {
    error.value = errorMessage(caught);
  } finally {
    treeLoading.value = false;
  }
}

async function openFile(path: string, split = false, navigate = true) {
  let opened = openFiles.value.find((file) => file.projection.revision.path === path);
  if (!opened) {
    try {
      const projection = await api.codingFile(sessionId.value, path);
      opened = {
        projection,
        draft: projection.content ?? "",
        dirty: false,
        saving: false,
        error: "",
      };
      openFiles.value.push(opened);
    } catch (caught) {
      error.value = errorMessage(caught);
      return;
    }
  }
  if (split) splitPath.value = path;
  else activePath.value = path;
  if (navigate) await setRoute({ pane: "editor", file: activePath.value });
}

function closeFile(path: string) {
  const index = openFiles.value.findIndex((file) => file.projection.revision.path === path);
  if (index < 0) return;
  openFiles.value.splice(index, 1);
  if (splitPath.value === path) splitPath.value = undefined;
  if (activePath.value === path) {
    activePath.value = openFiles.value[Math.max(0, index - 1)]?.projection.revision.path;
    void setRoute({ file: activePath.value });
  }
}

function updateDraft(file: OpenFile, value: string) {
  file.draft = value;
  file.dirty = value !== (file.projection.content ?? "");
}

async function saveFile(file = activeFile.value) {
  if (!file || !file.dirty || file.projection.binary || file.projection.truncated) return;
  file.saving = true;
  file.error = "";
  try {
    const saved = await api.saveCodingFile(
      sessionId.value,
      file.projection.revision.path,
      file.draft,
      file.projection.revision.sha256,
    );
    file.projection = saved;
    file.draft = saved.content ?? "";
    file.dirty = false;
    await Promise.all([loadTree(), loadDiff()]);
  } catch (caught) {
    if (caught instanceof ApiError && caught.status === 409 && isFileConflict(caught.details)) {
      conflict.value = {
        payload: caught.details,
        localDraft: file.draft,
        path: file.projection.revision.path,
      };
    } else {
      file.error = errorMessage(caught);
    }
  } finally {
    file.saving = false;
  }
}

async function saveAll() {
  for (const file of openFiles.value.filter((candidate) => candidate.dirty)) {
    await saveFile(file);
    if (conflict.value) return;
  }
}

async function loadDiff() {
  if (!session.value || session.value.state !== "ready") return;
  diffLoading.value = true;
  diffError.value = "";
  try {
    diff.value = await api.codingDiff(sessionId.value, { kind: "workspace" }, diffMode.value);
  } catch (caught) {
    diffError.value = errorMessage(caught);
  } finally {
    diffLoading.value = false;
  }
}

async function closeSession() {
  if (!session.value || dirtyCount.value > 0) return;
  try {
    await api.closeCodingSession(session.value.id);
    await router.push({ name: "project", params: { projectId: projectId.value } });
  } catch (caught) {
    error.value = errorMessage(caught);
  }
}

async function attachSelection() {
  if (!activePath.value || !selectedRange.value) return;
  const selection = selectedRange.value;
  attachments.value.push({
    id: globalThis.crypto.randomUUID(),
    kind: "selection",
    label: `${activePath.value}:${String(selection.startLine)}-${String(selection.endLine)}`,
    detail: selection.content,
    reference: activePath.value,
    sha256: await sha256(selection.content),
    startLine: selection.startLine,
    endLine: selection.endLine,
  });
  rightPane.value = "context";
}

async function attachHunk(hunk: DiffHunk, path: string) {
  const content = hunk.lines.map((line) => line.content).join("\n");
  attachments.value.push({
    id: globalThis.crypto.randomUUID(),
    kind: "diff_hunk",
    label: `${path} · ${hunk.id.slice(0, 10)}`,
    detail: content,
    reference: `workspace-diff/${diff.value?.digest ?? "unknown"}/${path}/${hunk.id}`,
    sha256: await sha256(content),
    startLine: hunk.new.start,
    endLine: hunk.new.start + Math.max(0, hunk.new.lines - 1),
  });
  rightPane.value = "context";
}

async function submitAgentTurn() {
  const agentId = selectedAgentId.value;
  const prompt = agentPrompt.value.trim();
  if (!agentId || !prompt || coordinationState.value !== "ready") return;
  const priorMessages = sessionTasks.value
    .filter((task) => task.status === "succeeded" && task.output)
    .slice(-10)
    .flatMap((task) => [
      { role: "user" as const, content: task.prompt },
      { role: "assistant" as const, content: task.output ?? "" },
    ]);
  const task = await workspace.submitCodingTurn(sessionId.value, agentId, {
    prompt,
    messages: priorMessages,
    focused_selections: attachments.value.map((attachment) => ({
      id: attachment.id,
      kind:
        attachment.kind === "selection"
          ? ("editor" as const)
          : attachment.kind === "diff_hunk"
            ? ("diff_hunk" as const)
            : ("terminal" as const),
      reference: attachment.reference,
      start_line: attachment.startLine ?? null,
      end_line: attachment.endLine ?? null,
      content: attachment.detail,
      sha256: attachment.sha256,
      truncated: false,
    })),
    open_files: openFiles.value.map((file) => ({
      path: file.projection.revision.path,
      sha256: file.projection.revision.sha256,
      cursor: null,
      dirty: file.dirty,
    })),
    active_diff: activePane.value === "diff" ? { kind: "workspace" } : null,
    idempotency_key: globalThis.crypto.randomUUID(),
  });
  if (task) {
    agentPrompt.value = "";
    attachments.value = [];
  }
}

async function shareAttachment(attachment: ContextAttachment) {
  if (coordinationState.value !== "ready") return;
  coordinationError.value = "";
  try {
    const view = await api.pinCodingContext(sessionId.value, {
      kind:
        attachment.kind === "selection"
          ? "Editor"
          : attachment.kind === "diff_hunk"
            ? "DiffHunk"
            : "Terminal",
      reference: attachment.reference,
      start_line: attachment.startLine ?? null,
      end_line: attachment.endLine ?? null,
      sha256: attachment.sha256,
      idempotency_key: globalThis.crypto.randomUUID(),
    });
    applyCoordinationView(view);
    attachments.value = attachments.value.filter((item) => item.id !== attachment.id);
  } catch (caught) {
    coordinationError.value = errorMessage(caught);
  }
}

async function createCodingGrant(grantee: string) {
  if (coordinationState.value !== "ready") return;
  coordinationMutation.value = `grant:${grantee}`;
  coordinationError.value = "";
  try {
    applyCoordinationView(await api.createCodingGrant(sessionId.value, grantee));
  } catch (caught) {
    coordinationError.value = errorMessage(caught);
  } finally {
    coordinationMutation.value = "";
  }
}

async function revokeGrant(grantId: string) {
  coordinationMutation.value = `revoke:${grantId}`;
  try {
    applyCoordinationView(await api.revokeCodingGrant(sessionId.value, grantId));
  } catch (caught) {
    coordinationError.value = errorMessage(caught);
  } finally {
    coordinationMutation.value = "";
  }
}

async function removeContextPin(pinId: string) {
  coordinationMutation.value = `unpin:${pinId}`;
  try {
    applyCoordinationView(await api.removeCodingContextPin(sessionId.value, pinId));
  } catch (caught) {
    coordinationError.value = errorMessage(caught);
  } finally {
    coordinationMutation.value = "";
  }
}

async function decideCheckpoint(checkpointId: string, decision: "approve" | "deny") {
  coordinationMutation.value = `${decision}:${checkpointId}`;
  coordinationError.value = "";
  try {
    applyCoordinationView(
      await api.decideCodingCheckpoint(sessionId.value, checkpointId, decision),
    );
  } catch (caught) {
    coordinationError.value = errorMessage(caught);
  } finally {
    coordinationMutation.value = "";
  }
}

function acceptLatestConflict() {
  const current = conflict.value;
  if (!current) return;
  const file = openFiles.value.find(
    (candidate) => candidate.projection.revision.path === current.path,
  );
  if (file) {
    file.projection = current.payload.latest;
    file.draft = current.payload.latest.content ?? "";
    file.dirty = false;
  }
  conflict.value = undefined;
}

async function setPane(pane: CenterPane) {
  await setRoute({ pane });
}

async function setRoute(values: Record<string, string | undefined>) {
  const query = Object.fromEntries(
    Object.entries({ ...route.query, ...values }).filter(([, value]) => value !== undefined),
  );
  await router.replace({ query });
}

function isDirectory(entry: CodingTreeEntry): boolean {
  return entry.kind === "directory" || entry.kind === "tree";
}

function treeEntryLabel(entry: CodingTreeEntry): string {
  return entry.path.split("/").filter(Boolean).pop() ?? entry.path;
}

function isFileConflict(value: unknown): value is FileConflict {
  if (!value || typeof value !== "object") return false;
  const candidate = value as Partial<FileConflict>;
  return candidate.code === "workspace_file_conflict" && Boolean(candidate.latest?.revision);
}

async function sha256(content: string): Promise<string> {
  const digest = await globalThis.crypto.subtle.digest(
    "SHA-256",
    new TextEncoder().encode(content),
  );
  return [...new Uint8Array(digest)].map((byte) => byte.toString(16).padStart(2, "0")).join("");
}
</script>

<template>
  <main class="hosted-workbench">
    <section v-if="state === 'loading'" class="workbench-gate-state">
      <LoaderCircle class="spinning" :size="26" />
      <strong>Resuming the exact workspace materialization…</strong>
      <p>Workspace is revalidating the project and current browser authority.</p>
    </section>
    <section v-else-if="state === 'refused'" class="workbench-gate-state refused-state">
      <ShieldCheck :size="28" />
      <strong>Hosted coding workspace unavailable</strong>
      <p>{{ error || "This deployment has not enabled the AgentIDE workbench feature." }}</p>
      <RouterLink class="button" :to="`/projects/${projectId}`">Return to project</RouterLink>
    </section>
    <section v-else-if="state === 'error'" class="workbench-gate-state error-state">
      <CircleAlert :size="28" />
      <strong>Workspace could not be resumed</strong>
      <p>{{ error }}</p>
      <button class="button" type="button" @click="load">Try again</button>
    </section>

    <template v-else>
      <header class="workbench-titlebar">
        <RouterLink :to="`/projects/${projectId}`" class="workbench-project-link">
          <strong>{{ project?.path_with_namespace }}</strong>
        </RouterLink>
        <span><GitBranch :size="14" /> {{ project?.selected_branch }}</span>
        <code><Braces :size="13" /> {{ session?.source_revision.slice(0, 10) }}</code>
        <span class="status-pill">{{ session?.state }}</span>
        <span class="status-pill" :class="{ refused: coordinationState === 'degraded' }"
          >AgentIDE {{ coordinationState }}</span
        >
        <div class="workbench-title-actions">
          <button class="button small" type="button" :disabled="dirtyCount === 0" @click="saveAll">
            <Save :size="14" /> Save all <span v-if="dirtyCount">({{ dirtyCount }})</span>
          </button>
          <button
            class="button quiet small"
            type="button"
            :disabled="dirtyCount > 0"
            @click="closeSession"
          >
            Close session
          </button>
        </div>
      </header>

      <div v-if="error" class="workbench-notice error-state" role="alert">{{ error }}</div>
      <div
        v-if="coordinationState === 'degraded'"
        class="workbench-notice error-state coordination-notice"
        role="alert"
      >
        <div>
          <strong>Agent features are temporarily unavailable</strong>
          <p>
            {{ coordinationError }} Files, saves, diffs, and existing terminal sessions remain
            available. New grants, agent turns, context pins, and terminals stay disabled.
          </p>
        </div>
        <button class="button" type="button" @click="loadCoordination">Retry agent features</button>
      </div>
      <div
        class="workbench-grid"
        :class="{ 'terminal-collapsed': !terminalOpen }"
        :style="{
          '--terminal-height': terminalOpen ? `${String(terminalHeight)}px` : '2.45rem',
        }"
      >
        <aside class="workbench-explorer" aria-label="Project explorer and context pins">
          <header>
            <strong>Explorer</strong
            ><button class="icon-button" type="button" title="Refresh tree" @click="loadTree">
              <RefreshCw :size="14" :class="{ spinning: treeLoading }" />
            </button>
          </header>
          <label class="workbench-tree-search">
            <Search :size="14" /><span class="sr-only">Search files</span>
            <input v-model="treeSearch" type="search" placeholder="Search files" />
          </label>
          <div v-if="treeTruncated" class="tree-boundary-state">
            Showing a bounded result.
            {{
              treeOmitted == null
                ? "Additional count is unknown."
                : `${treeOmitted} entries omitted.`
            }}
          </div>
          <div class="workbench-tree" role="tree">
            <button
              v-for="entry in tree"
              :key="entry.path"
              type="button"
              role="treeitem"
              :disabled="isDirectory(entry)"
              :class="{ active: activePath === entry.path, directory: isDirectory(entry) }"
              :style="{
                paddingLeft: `${10 + Math.min(entry.path.split('/').length - 1, 6) * 10}px`,
              }"
              @click="!isDirectory(entry) && openFile(entry.path)"
              @dblclick="!isDirectory(entry) && openFile(entry.path, true)"
            >
              <Folder v-if="isDirectory(entry)" :size="14" /><File v-else :size="14" />
              <span :title="entry.path">{{ treeEntryLabel(entry) }}</span>
              <small v-if="entry.size != null">{{ entry.size }}</small>
            </button>
            <div v-if="!tree.length && !treeLoading" class="workbench-empty compact">
              No matching files.
            </div>
          </div>
          <section class="explorer-pins">
            <header>
              <strong>Local attachment tray</strong><span>{{ attachments.length }}</span>
            </header>
            <button
              v-if="selectedRange"
              class="button small"
              type="button"
              @click="attachSelection"
            >
              Attach editor selection
            </button>
            <p v-if="!attachments.length">
              Selections and diff hunks stay browser-local until you share their references.
            </p>
            <article v-for="attachment in attachments" :key="attachment.id">
              <span>{{ attachment.kind.replace("_", " ") }}</span
              ><strong>{{ attachment.label }}</strong>
              <button
                type="button"
                class="attachment-share"
                :disabled="coordinationState !== 'ready'"
                @click="shareAttachment(attachment)"
              >
                Pin for session
              </button>
              <button
                type="button"
                aria-label="Remove attachment"
                @click="attachments = attachments.filter((item) => item.id !== attachment.id)"
              >
                <X :size="12" />
              </button>
            </article>
          </section>
        </aside>

        <section class="workbench-center">
          <nav class="workbench-pane-tabs" aria-label="Workspace panes">
            <button
              type="button"
              :class="{ active: activePane === 'editor' }"
              @click="setPane('editor')"
            >
              <File :size="14" /> Editor
            </button>
            <button
              type="button"
              :class="{ active: activePane === 'diff' }"
              @click="setPane('diff')"
            >
              <FileDiff :size="14" /> Diff
            </button>
            <button
              type="button"
              :class="{ active: activePane === 'agent' }"
              @click="setPane('agent')"
            >
              <Bot :size="14" /> Agent
            </button>
          </nav>

          <template v-if="activePane === 'editor'">
            <nav v-if="openFiles.length" class="editor-file-tabs" aria-label="Open files">
              <button
                v-for="file in openFiles"
                :key="file.projection.revision.path"
                type="button"
                :class="{ active: activePath === file.projection.revision.path }"
                @click="openFile(file.projection.revision.path)"
              >
                <span v-if="file.dirty" class="dirty-dot" title="Unsaved"></span>
                {{ file.projection.revision.path.split("/").at(-1) }}
                <X :size="12" @click.stop="closeFile(file.projection.revision.path)" />
              </button>
              <button
                v-if="activeFile"
                class="editor-split-action"
                type="button"
                title="Open active file in split"
                @click="openFile(activeFile.projection.revision.path, true)"
              >
                <SplitSquareHorizontal :size="14" />
              </button>
            </nav>
            <div v-if="activeFile" class="editor-surface" :class="{ split: splitFile }">
              <section class="editor-column">
                <header>
                  <code>{{ activeFile.projection.revision.path }}</code>
                  <span
                    >{{ activeFile.projection.revision.language || "plain text" }} ·
                    {{ activeFile.projection.revision.modification }}</span
                  >
                  <button
                    class="button small"
                    type="button"
                    :disabled="
                      !activeFile.dirty ||
                      activeFile.saving ||
                      activeFile.projection.binary ||
                      activeFile.projection.truncated
                    "
                    @click="saveFile(activeFile)"
                  >
                    <LoaderCircle v-if="activeFile.saving" class="spinning" :size="13" /><Save
                      v-else
                      :size="13"
                    />
                    Save
                  </button>
                </header>
                <div
                  v-if="activeFile.projection.binary || activeFile.projection.truncated"
                  class="workbench-gate-state compact"
                >
                  <CircleAlert :size="20" /><strong>Read-only file</strong>
                  <p>
                    {{
                      activeFile.projection.binary
                        ? "Workspace identified this as binary content."
                        : "Workspace refused incomplete editing."
                    }}
                  </p>
                </div>
                <CodeEditor
                  v-else
                  :model-value="activeFile.draft"
                  :path="activeFile.projection.revision.path"
                  :language="activeFile.projection.revision.language"
                  @update:model-value="updateDraft(activeFile, $event)"
                  @save="saveFile(activeFile)"
                  @selection="selectedRange = $event"
                />
                <p v-if="activeFile.error" class="editor-error">{{ activeFile.error }}</p>
              </section>
              <section v-if="splitFile" class="editor-column">
                <header>
                  <code>{{ splitFile.projection.revision.path }}</code
                  ><button
                    class="icon-button"
                    type="button"
                    aria-label="Close split"
                    @click="splitPath = undefined"
                  >
                    <X :size="14" />
                  </button>
                </header>
                <CodeEditor
                  :model-value="splitFile.draft"
                  :path="splitFile.projection.revision.path"
                  :language="splitFile.projection.revision.language"
                  :read-only="splitFile.projection.binary || splitFile.projection.truncated"
                  @update:model-value="updateDraft(splitFile, $event)"
                  @save="saveFile(splitFile)"
                />
              </section>
            </div>
            <div v-else class="workbench-empty">
              <File :size="28" /><strong>Choose a file</strong>
              <p>The explorer is bounded and reflects the working materialization.</p>
            </div>
          </template>

          <template v-else-if="activePane === 'diff'">
            <div v-if="diffLoading" class="workbench-empty">
              <LoaderCircle class="spinning" :size="22" /> Resolving canonical diff…
            </div>
            <div v-else-if="diffError" class="workbench-empty error-state">
              <CircleAlert :size="22" /><strong>Diff unavailable</strong>
              <p>{{ diffError }}</p>
              <button class="button" type="button" @click="loadDiff">Retry</button>
            </div>
            <CanonicalDiffViewer
              v-else-if="diff"
              :projection="diff"
              :layout="diffLayout"
              @mode="setRoute({ mode: $event })"
              @layout="setRoute({ layout: $event })"
              @attach="attachHunk"
            />
          </template>

          <section v-else-if="activePane === 'agent'" class="coding-agent-pane">
            <header class="coding-agent-header">
              <div>
                <span class="eyebrow">Agent Platform · coding session turn</span>
                <strong>{{ selectedAgent?.name ?? "Choose an agent" }}</strong>
              </div>
              <div class="coding-agent-revisions" aria-label="Current agent context revisions">
                <span>context {{ agentRun?.contextRevision?.slice(0, 12) ?? "pending" }}</span>
                <span>tools {{ agentRun?.inventoryRevision?.slice(0, 12) ?? "pending" }}</span>
              </div>
            </header>

            <div class="coding-agent-transcript" aria-live="polite">
              <div v-if="!sessionTasks.length" class="workbench-empty compact">
                <Bot :size="24" />
                <strong>No turns in this workspace yet</strong>
                <p>
                  The agent will refresh its server-derived ActorView immediately before every model
                  turn.
                </p>
              </div>
              <article v-for="task in sessionTasks" :key="task.id" class="coding-agent-turn">
                <div class="coding-agent-message human-message">
                  <span>You</span>
                  <p>{{ task.prompt }}</p>
                </div>
                <div class="coding-agent-message assistant-message">
                  <span>{{ selectedAgent?.name ?? "Agent" }} · {{ task.status }}</span>
                  <pre>{{
                    task.output ||
                    (task.id === agentRun?.taskId ? agentRun.output : "") ||
                    "Waiting for output…"
                  }}</pre>
                  <p v-if="task.failure_message" class="editor-error">
                    {{ task.failure_message }}
                  </p>
                </div>
              </article>
              <p v-if="agentRun?.error" class="coding-agent-error" role="alert">
                {{ agentRun.error }}
              </p>
              <section
                v-for="approval in agentRun?.approvals ?? []"
                :key="approval.id"
                class="coding-agent-approval"
              >
                <div>
                  <span class="eyebrow">Exact operation approval</span>
                  <strong>{{ approval.operation_ref }}</strong>
                  <code>{{ JSON.stringify(approval.input, null, 2) }}</code>
                </div>
                <div class="approval-actions">
                  <button
                    class="button quiet small"
                    type="button"
                    :disabled="agentRun?.resolvingApprovalId === approval.id"
                    @click="workspace.resolveTaskApproval(selectedAgentId!, approval.id, 'deny')"
                  >
                    Deny
                  </button>
                  <button
                    class="button primary small"
                    type="button"
                    :disabled="agentRun?.resolvingApprovalId === approval.id"
                    @click="workspace.resolveTaskApproval(selectedAgentId!, approval.id, 'approve')"
                  >
                    Approve exact call
                  </button>
                </div>
              </section>
            </div>

            <form class="coding-agent-composer" @submit.prevent="submitAgentTurn">
              <textarea
                v-model="agentPrompt"
                rows="3"
                maxlength="131072"
                placeholder="Ask the agent to inspect, edit, or verify this workspace…"
                aria-label="Coding agent prompt"
                @keydown.meta.enter.prevent="submitAgentTurn"
                @keydown.ctrl.enter.prevent="submitAgentTurn"
              ></textarea>
              <div class="coding-agent-composer-context">
                <span
                  v-for="attachment in attachments"
                  :key="attachment.id"
                  class="composer-context-chip"
                  :title="attachment.reference"
                  >{{ attachment.label }}</span
                >
                <span v-if="!attachments.length">No prompt attachments</span>
                <span>{{ openFiles.length }} open file digest(s)</span>
                <span v-if="dirtyCount" class="warning-copy">
                  {{ dirtyCount }} unsaved buffer(s) excluded
                </span>
                <span v-if="agentRun?.publishedTools?.length">
                  {{ agentRun.publishedTools.join(", ") }}
                </span>
              </div>
              <footer>
                <p>Saved Workspace content and explicit selections only. Ctrl/⌘ + Enter sends.</p>
                <button
                  class="button primary"
                  type="submit"
                  :disabled="
                    !agentPrompt.trim() ||
                    !selectedAgentId ||
                    !agentIdeSession ||
                    coordinationState !== 'ready' ||
                    agentTurnPending
                  "
                >
                  <LoaderCircle v-if="agentTurnPending" class="spinning" :size="14" />
                  <Send v-else :size="14" />
                  {{ agentTurnPending ? "Agent working…" : "Send turn" }}
                </button>
              </footer>
            </form>
          </section>
        </section>

        <aside class="workbench-inspector">
          <nav aria-label="Session inspector">
            <button
              v-for="pane in ['context', 'agents', 'grants', 'approvals'] as const"
              :key="pane"
              type="button"
              :class="{ active: rightPane === pane }"
              @click="rightPane = pane"
            >
              {{ pane }}
            </button>
          </nav>
          <section v-if="rightPane === 'context'">
            <h2>Session context</h2>
            <dl>
              <dt>Actor</dt>
              <dd>{{ workspace.session?.subject }}</dd>
              <dt>Source</dt>
              <dd>
                <code>{{ session?.source_revision }}</code>
              </dd>
              <dt>Manifest</dt>
              <dd>
                <code>{{ session?.manifest_sha256?.slice(0, 16) || "unknown" }}</code>
              </dd>
              <dt>Files limit</dt>
              <dd>{{ session?.limits.max_files }}</dd>
              <dt>File bytes</dt>
              <dd>{{ session?.limits.max_file_bytes }}</dd>
              <dt>AgentIDE</dt>
              <dd>
                <code>{{ agentIdeSession?.session_id || coordinationState }}</code>
              </dd>
            </dl>
            <h3>Local attachment tray</h3>
            <article
              v-for="attachment in attachments"
              :key="attachment.id"
              class="context-attachment"
            >
              <strong>{{ attachment.label }}</strong>
              <pre>{{ attachment.detail }}</pre>
            </article>
            <p v-if="!attachments.length" class="muted">No unshared browser-local selections.</p>
            <h3>Shared context references</h3>
            <article
              v-for="pin in agentIdePins.filter((item) => item.state === 'Active')"
              :key="pin.pin_id"
              class="context-reference"
            >
              <div>
                <strong>{{ pin.kind }}</strong>
                <code>{{ pin.reference }}</code>
                <small>
                  {{ pin.start_line ?? "?" }}–{{ pin.end_line ?? "?" }} ·
                  {{ pin.sha256.slice(0, 12) }}
                </small>
              </div>
              <button
                class="button quiet small"
                type="button"
                :disabled="coordinationMutation === `unpin:${pin.pin_id}`"
                @click="removeContextPin(pin.pin_id)"
              >
                Remove
              </button>
            </article>
            <p v-if="!agentIdePins.some((item) => item.state === 'Active')" class="muted">
              No references are shared with the session.
            </p>
          </section>
          <section v-else-if="rightPane === 'agents'">
            <h2>Agents</h2>
            <article
              v-for="agent in workspace.agents"
              :key="agent.id"
              class="inspector-agent"
              :class="{ active: selectedAgentId === agent.id }"
            >
              <button
                class="inspector-agent-select"
                type="button"
                @click="setRoute({ agent: agent.id })"
              >
                <Bot :size="15" /><span
                  ><strong>{{ agent.name }}</strong
                  ><small>Agent Platform identity {{ agent.id }}</small></span
                ><ChevronRight :size="14" />
              </button>
              <button
                v-if="coordinationState === 'ready'"
                class="button small"
                type="button"
                :disabled="coordinationMutation === `grant:${agent.id}`"
                @click.stop="createCodingGrant(agent.id)"
              >
                Grant coding edits
              </button>
            </article>
            <p v-if="!workspace.agents.length" class="muted">
              No agents are available to this actor.
            </p>
          </section>
          <section v-else-if="rightPane === 'grants'">
            <h2>Authority grants</h2>
            <article v-for="grant in agentIdeGrants" :key="grant.grant_id" class="authority-card">
              <header>
                <strong>{{ grant.grantee }}</strong
                ><span class="status-pill">{{ grant.state }}</span>
              </header>
              <p>{{ grant.allowed_intents.join(", ") }}</p>
              <small>
                {{ grant.maximum_risk }} risk · paths {{ grant.path_prefixes.join(", ") }} ·
                revision
                {{ grant.revision }}
              </small>
              <button
                v-if="grant.state === 'Active'"
                class="button quiet small"
                type="button"
                :disabled="coordinationMutation === `revoke:${grant.grant_id}`"
                @click="revokeGrant(grant.grant_id)"
              >
                Revoke
              </button>
            </article>
            <p v-if="!agentIdeGrants.length" class="muted">No bounded grants exist.</p>
          </section>
          <section v-else-if="rightPane === 'approvals'">
            <h2>Exact-plan approvals</h2>
            <article
              v-for="checkpoint in agentIdeCheckpoints"
              :key="checkpoint.checkpoint_id"
              class="authority-card"
            >
              <header>
                <strong>{{ checkpoint.attempt_ref }}</strong
                ><span class="status-pill">{{ checkpoint.state }}</span>
              </header>
              <code>{{ checkpoint.plan_digest }}</code>
              <div v-if="checkpoint.state === 'Pending'" class="approval-actions">
                <button
                  class="button quiet small"
                  type="button"
                  :disabled="Boolean(coordinationMutation)"
                  @click="decideCheckpoint(checkpoint.checkpoint_id, 'deny')"
                >
                  Deny
                </button>
                <button
                  class="button primary small"
                  type="button"
                  :disabled="Boolean(coordinationMutation)"
                  @click="decideCheckpoint(checkpoint.checkpoint_id, 'approve')"
                >
                  Approve exact plan
                </button>
              </div>
            </article>
            <p v-if="!agentIdeCheckpoints.length" class="muted">No approval checkpoints exist.</p>
          </section>
        </aside>

        <section class="workbench-terminal" :class="{ collapsed: !terminalOpen }">
          <div
            v-if="terminalOpen"
            class="terminal-resize-handle"
            role="separator"
            aria-label="Resize terminal panel"
            aria-orientation="horizontal"
            :aria-valuenow="terminalHeight"
            aria-valuemin="180"
            aria-valuemax="1000"
            tabindex="0"
            @pointerdown="startTerminalResize"
            @keydown.up.prevent="adjustTerminalHeight(20)"
            @keydown.down.prevent="adjustTerminalHeight(-20)"
          ></div>
          <header>
            <button type="button" @click="terminalOpen = !terminalOpen">
              <PanelBottomClose v-if="terminalOpen" :size="15" /><PanelBottomOpen
                v-else
                :size="15"
              />
              Terminal
            </button>
            <span
              v-if="terminalOpen"
              class="status-pill"
              :class="{ refused: terminalState === 'refused' || terminalState === 'error' }"
            >
              {{ activeTerminal?.state ?? terminalState }}
            </span>
            <button
              v-if="terminalOpen"
              class="icon-button"
              type="button"
              aria-label="Terminal authority details"
              @click="rightPane = 'grants'"
            >
              <PanelRight :size="14" />
            </button>
          </header>
          <div v-if="terminalOpen" class="terminal-panel">
            <div v-if="terminalState === 'loading'" class="terminal-refused" role="status">
              <LoaderCircle class="spinning" :size="20" />
              <div>
                <strong>Loading admitted terminal profiles…</strong>
                <p>Workspace is deriving terminal availability for the current human actor.</p>
              </div>
            </div>
            <div v-else-if="terminalState === 'refused'" class="terminal-refused">
              <SquareTerminal :size="22" />
              <div>
                <strong>Interactive terminal refused</strong>
                <p>
                  {{
                    terminalError ||
                    "No deployment-declared terminal profile is available. DevCenter never falls back to its host shell."
                  }}
                </p>
              </div>
            </div>
            <div v-else class="terminal-ready-panel">
              <div class="terminal-profile-actions">
                <label>
                  <span>Profile</span>
                  <select v-model="selectedTerminalProfileId">
                    <option
                      v-for="profile in terminalProfiles"
                      :key="profile.id"
                      :value="profile.id"
                    >
                      {{ profile.label }} · {{ profile.workspace_access.replace("_", " ") }} ·
                      network {{ profile.network }}
                    </option>
                  </select>
                </label>
                <button
                  class="button primary small"
                  type="button"
                  :disabled="
                    terminalState !== 'ready' ||
                    coordinationState !== 'ready' ||
                    Boolean(terminalMutation)
                  "
                  @click="openTerminal"
                >
                  <LoaderCircle v-if="terminalMutation === 'create'" class="spinning" :size="13" />
                  Open terminal
                </button>
                <small>
                  {{
                    terminalProfiles.find((profile) => profile.id === selectedTerminalProfileId)
                      ?.working_directory
                  }}
                  · network
                  {{
                    terminalProfiles.find((profile) => profile.id === selectedTerminalProfileId)
                      ?.network
                  }}
                  ·
                  {{
                    terminalProfiles
                      .find((profile) => profile.id === selectedTerminalProfileId)
                      ?.workspace_access.replace("_", " ")
                  }}
                </small>
              </div>
              <p v-if="terminalError" class="terminal-detail" role="alert">
                {{ terminalError }}
              </p>
              <nav
                v-if="visibleTerminals.length"
                class="terminal-tabs"
                aria-label="Terminal sessions"
              >
                <article
                  v-for="terminal in visibleTerminals"
                  :key="terminal.id"
                  class="terminal-tab"
                  :class="{ active: activeTerminal?.id === terminal.id }"
                >
                  <button type="button" @click="setRoute({ terminal: terminal.id })">
                    <SquareTerminal :size="12" />
                    <span>{{ terminal.profile.label }}</span>
                    <small>{{ terminal.state }}</small>
                  </button>
                  <button
                    type="button"
                    :aria-label="`Detach ${terminal.profile.label}`"
                    title="Detach terminal tab"
                    @click="detachTerminal(terminal.id)"
                  >
                    <X :size="12" />
                  </button>
                </article>
              </nav>
              <template v-if="activeTerminal">
                <div class="terminal-session-metadata">
                  <span><strong>Actor</strong> {{ activeTerminal.actor }}</span>
                  <span
                    ><strong>Process</strong> {{ activeTerminal.process_id ?? "allocating" }}</span
                  >
                  <span><strong>Network</strong> {{ activeTerminal.profile.network }}</span>
                  <span><strong>Runtime</strong> {{ activeTerminal.profile.runtime_ref }}</span>
                </div>
                <HostedTerminal
                  v-if="activeTerminal.state === 'running' || activeTerminal.state === 'preparing'"
                  :key="activeTerminal.id"
                  :terminal="activeTerminal"
                  @attach="attachTerminalSelection"
                  @kill="killTerminal(activeTerminal.id)"
                  @lifecycle="loadTerminals"
                />
                <div v-else class="terminal-refused">
                  <SquareTerminal :size="20" />
                  <div>
                    <strong>Terminal {{ activeTerminal.state }}</strong>
                    <p>
                      {{
                        activeTerminal.failure_code ||
                        activeTerminal.exit?.signal ||
                        (activeTerminal.exit?.code == null
                          ? "No exit detail was reported."
                          : `Process exited with ${String(activeTerminal.exit.code)}.`)
                      }}
                    </p>
                  </div>
                </div>
              </template>
              <div v-else class="terminal-refused">
                <SquareTerminal :size="20" />
                <div>
                  <strong>No attached terminals</strong>
                  <p>Open a confined terminal, or reattach one by refreshing the session list.</p>
                </div>
              </div>
            </div>
          </div>
        </section>
      </div>

      <dialog
        :open="Boolean(conflict)"
        class="file-conflict-dialog"
        aria-labelledby="file-conflict-title"
      >
        <header>
          <div>
            <p class="eyebrow">Digest conflict</p>
            <h2 id="file-conflict-title">{{ conflict?.path }} changed after loading</h2>
          </div>
          <button
            class="icon-button"
            type="button"
            aria-label="Close conflict"
            @click="conflict = undefined"
          >
            <X :size="17" />
          </button>
        </header>
        <p>
          Workspace refused the save. Compare the immutable base, your local draft, and the latest
          working content. There is no blind overwrite action.
        </p>
        <div class="conflict-columns">
          <section>
            <strong>Immutable base</strong>
            <pre>{{ conflict?.payload.base?.content ?? "File absent from base" }}</pre>
          </section>
          <section>
            <strong>Local draft</strong>
            <pre>{{ conflict?.localDraft }}</pre>
          </section>
          <section>
            <strong>Latest workspace</strong>
            <pre>{{ conflict?.payload.latest.content ?? "Binary content" }}</pre>
          </section>
        </div>
        <footer>
          <button class="button" type="button" @click="conflict = undefined">
            Keep local draft
          </button>
          <button class="button primary" type="button" @click="acceptLatestConflict">
            Replace draft with latest
          </button>
        </footer>
      </dialog>
    </template>
  </main>
</template>
