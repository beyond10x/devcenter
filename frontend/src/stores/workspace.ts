import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  ApiError,
  api,
  errorMessage,
  type Agent,
  type AgentConversation,
  type CreateAgent,
  type CapabilityProfile,
  type ClaudeOAuthStart,
  type IdentityProvider,
  type Session,
  type Task,
  type TaskApproval,
  type TaskEventEnvelope,
  type SubmitCodingTurn,
  taskFailureMessage,
} from "@/api/client";

export type LoadState = "idle" | "loading" | "ready" | "error";
export type RunState =
  | "idle"
  | "submitting"
  | "accepted"
  | "running"
  | "awaiting_approval"
  | "succeeded"
  | "failed"
  | "reconnecting";

export interface AgentRun {
  status: RunState;
  output: string;
  error: string;
  taskId?: string;
  approvals?: TaskApproval[];
  approvalError?: string;
  resolvingApprovalId?: string;
  contextRevision?: string;
  inventoryRevision?: string;
  publishedTools?: string[];
}

const streams = new Map<string, EventSource>();

export const useWorkspaceStore = defineStore("workspace", () => {
  const sessionState = ref<LoadState>("loading");
  const session = ref<Session>();
  const sessionError = ref("");
  const identityProviders = ref<IdentityProvider[]>([]);
  const agentsState = ref<LoadState>("idle");
  const agents = ref<Agent[]>([]);
  const agentsError = ref("");
  const capabilityProfiles = ref<CapabilityProfile[]>([]);
  const selectedAgentId = ref<string>();
  const drafts = ref<Record<string, string>>({});
  const runs = ref<Record<string, AgentRun>>({});
  const taskHistory = ref<Record<string, Task[]>>({});
  const conversations = ref<Record<string, AgentConversation[]>>({});
  const selectedConversationIds = ref<Record<string, string | undefined>>({});
  const connectionState = ref<LoadState>("idle");
  const connected = ref(false);
  const connectionError = ref("");
  const oauthFlow = ref<ClaudeOAuthStart>();
  const notice = ref("");
  let sessionGeneration = 0;
  const conversationGenerations = new Map<string, number>();
  function changeConversations(agentId: string) {
    const generation = (conversationGenerations.get(agentId) ?? 0) + 1;
    conversationGenerations.set(agentId, generation);
    return generation;
  }

  const selectedAgent = computed(() =>
    agents.value.find((agent) => agent.id === selectedAgentId.value),
  );

  async function bootstrap() {
    sessionState.value = "loading";
    sessionError.value = "";
    try {
      session.value = await api.session();
      await Promise.allSettled([loadAgents(), loadCapabilityProfiles(), loadConnection()]);
      sessionState.value = "ready";
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        sessionState.value = "idle";
        try {
          identityProviders.value = await api.identityProviders();
        } catch {
          identityProviders.value = [];
        }
      } else {
        sessionState.value = "error";
        sessionError.value = errorMessage(error);
      }
    }
  }

  async function logout() {
    await api.logout();
    sessionGeneration += 1;
    streams.forEach((stream) => stream.close());
    streams.clear();
    conversationGenerations.clear();
    conversations.value = {};
    selectedConversationIds.value = {};
    drafts.value = {};
    runs.value = {};
    taskHistory.value = {};
    capabilityProfiles.value = [];
    selectedAgentId.value = undefined;
    oauthFlow.value = undefined;
    session.value = undefined;
    sessionState.value = "idle";
    agents.value = [];
    connected.value = false;
    identityProviders.value = await api.identityProviders().catch(() => []);
  }

  async function loadAgents() {
    agentsState.value = "loading";
    agentsError.value = "";
    try {
      const nextAgents = await api.agents();
      agents.value = nextAgents;
      if (!nextAgents.some((agent) => agent.id === selectedAgentId.value)) {
        selectedAgentId.value = nextAgents[0]?.id;
      }
      agentsState.value = "ready";
      if (selectedAgentId.value) await loadAgentTasks(selectedAgentId.value);
    } catch (error) {
      agentsState.value = "error";
      agentsError.value = errorMessage(error);
    }
  }

  async function loadCapabilityProfiles() {
    capabilityProfiles.value = await api.capabilityProfiles();
  }

  async function loadConnection() {
    connectionState.value = "loading";
    connectionError.value = "";
    try {
      const status = await api.connection();
      connected.value = status.connected;
      connectionState.value = "ready";
    } catch (error) {
      connectionState.value = "error";
      connectionError.value = errorMessage(error);
    }
  }

  async function startOAuth(): Promise<ClaudeOAuthStart> {
    connectionError.value = "";
    try {
      oauthFlow.value = await api.startOAuth();
      return oauthFlow.value;
    } catch (error) {
      connectionError.value = errorMessage(error);
      throw error;
    }
  }

  async function completeOAuth(code: string) {
    const flowId = oauthFlow.value?.flow_id;
    if (!flowId) throw new Error("oauth_flow_missing");
    connectionError.value = "";
    try {
      const status = await api.completeOAuth(flowId, code.trim());
      connected.value = status.connected;
      oauthFlow.value = undefined;
      notice.value = "Claude Code authorization saved.";
    } catch (error) {
      // Connectors consumes the pending flow before exchanging the one-time code. Even if
      // the network response is lost, replaying this flow cannot safely complete it.
      oauthFlow.value = undefined;
      connectionError.value = errorMessage(error);
      throw error;
    }
  }

  function cancelOAuth() {
    oauthFlow.value = undefined;
    connectionError.value = "";
  }

  async function disconnect() {
    connectionError.value = "";
    try {
      const status = await api.disconnect();
      connected.value = status.connected;
      oauthFlow.value = undefined;
      notice.value = "Claude Code was disconnected.";
    } catch (error) {
      connectionError.value = errorMessage(error);
      throw error;
    }
  }

  async function createAgent(input: {
    name: string;
    instructions: string;
    model: string;
    capability_profile_id?: string;
  }) {
    const created = await api.createAgent(input);
    agents.value = [created, ...agents.value.filter((agent) => agent.id !== created.id)];
    selectedAgentId.value = created.id;

    notice.value = `${created.name} was created and activated.`;
    return created;
  }

  async function updateAgent(
    id: string,
    input: CreateAgent & { expected_active_revision: number | null },
  ) {
    const updated = await api.updateAgent(id, input);
    agents.value = agents.value.map((agent) => (agent.id === id ? updated : agent));
    notice.value = `${updated.name} was updated.`;
    return updated;
  }

  async function deleteAgent(id: string) {
    await api.deleteAgent(id);
    changeConversations(id);
    for (const key of Object.keys(drafts.value)) {
      if (key.startsWith(`${id}:`)) Reflect.deleteProperty(drafts.value, key);
    }
    streams.get(id)?.close();
    streams.delete(id);
    agents.value = agents.value.filter((agent) => agent.id !== id);
    Reflect.deleteProperty(taskHistory.value, id);
    Reflect.deleteProperty(conversations.value, id);
    Reflect.deleteProperty(selectedConversationIds.value, id);
    Reflect.deleteProperty(runs.value, id);
    if (selectedAgentId.value === id) selectedAgentId.value = agents.value[0]?.id;
    notice.value = "Agent deleted.";
  }

  function conversationFor(agentId: string) {
    return conversations.value[agentId]?.find(
      (item) => item.id === selectedConversationIds.value[agentId],
    );
  }

  async function loadConversations(agentId: string) {
    const generation = changeConversations(agentId);
    const owner = sessionGeneration;
    const items = await api.agentConversations(agentId);
    if (owner !== sessionGeneration || conversationGenerations.get(agentId) !== generation) return;
    conversations.value[agentId] = items;
    if (!items.some((item) => item.id === selectedConversationIds.value[agentId])) {
      selectedConversationIds.value[agentId] = items.at(-1)?.id;
    }
  }

  function selectConversation(agentId: string, id: string) {
    if (conversations.value[agentId]?.some((item) => item.id === id))
      selectedConversationIds.value[agentId] = id;
  }

  async function newConversation(agentId: string, title = "New conversation") {
    const owner = sessionGeneration;
    const item = await api.createAgentConversation(agentId, title);
    if (owner !== sessionGeneration) throw new Error("Session changed. Sign in again.");
    changeConversations(agentId);
    conversations.value[agentId] = [...(conversations.value[agentId] ?? []), item];
    selectedConversationIds.value[agentId] = item.id;
    return item;
  }

  async function renameConversation(item: AgentConversation, title: string) {
    const owner = sessionGeneration;
    const updated = await api.renameAgentConversation(item, title);
    if (owner !== sessionGeneration) return;
    changeConversations(item.agent_id);
    conversations.value[item.agent_id] = (conversations.value[item.agent_id] ?? []).map((entry) =>
      entry.id === item.id ? updated : entry,
    );
  }

  async function removeConversation(item: AgentConversation, clear = false) {
    const owner = sessionGeneration;
    const replacement = clear ? await api.clearAgentConversation(item) : undefined;
    if (!clear) await api.deleteAgentConversation(item);
    if (owner !== sessionGeneration) return;
    changeConversations(item.agent_id);
    conversations.value[item.agent_id] = (conversations.value[item.agent_id] ?? []).filter(
      (entry) => entry.id !== item.id,
    );
    if (replacement) conversations.value[item.agent_id]?.push(replacement);
    selectedConversationIds.value[item.agent_id] =
      replacement?.id ?? conversations.value[item.agent_id]?.at(-1)?.id;
    Reflect.deleteProperty(drafts.value, `${item.agent_id}:${item.id}`);
  }

  function conversationHistoryFor(agentId: string) {
    const ids = new Set(conversationFor(agentId)?.task_ids ?? []);
    return historyFor(agentId).filter((task) => ids.has(task.id));
  }

  function selectAgent(agentId: string) {
    selectedAgentId.value = agentId;
    void loadAgentTasks(agentId);
  }

  async function loadAgentTasks(agentId: string) {
    const owner = sessionGeneration;
    const previous = taskHistory.value[agentId];
    try {
      const tasks = await api.agentTasks(agentId);
      if (owner !== sessionGeneration || taskHistory.value[agentId] !== previous) return;
      taskHistory.value[agentId] = tasks;
      const active = [...tasks]
        .reverse()
        .find((task) => ["accepted", "running", "awaiting_approval"].includes(task.status));
      if (active) {
        runs.value[agentId] = {
          status: active.status as RunState,
          output: active.output ?? "",
          error: "",
          taskId: active.id,
        };
        streamTask(agentId, active.id);
      }
    } catch {
      // The live composer remains usable if history is temporarily unavailable.
    }
  }

  function historyFor(agentId: string): Task[] {
    return taskHistory.value[agentId] ?? [];
  }

  function updateHistory(agentId: string, taskId: string, changes: Partial<Task>) {
    taskHistory.value[agentId] = historyFor(agentId).map((task) =>
      task.id === taskId ? { ...task, ...changes } : task,
    );
  }

  function draftFor(agentId: string): string {
    return drafts.value[`${agentId}:${selectedConversationIds.value[agentId] ?? "new"}`] ?? "";
  }

  function setDraft(agentId: string, value: string) {
    drafts.value[`${agentId}:${selectedConversationIds.value[agentId] ?? "new"}`] = value;
  }

  function runFor(agentId: string): AgentRun {
    return runs.value[agentId] ?? { status: "idle", output: "", error: "" };
  }

  async function submitTask(agentId: string) {
    if (
      ["submitting", "accepted", "running", "awaiting_approval", "reconnecting"].includes(
        runFor(agentId).status,
      )
    )
      return;
    const owner = sessionGeneration;
    const prompt = draftFor(agentId).trim();
    if (!prompt) return;
    runs.value[agentId] = { status: "submitting", output: "", error: "" };
    try {
      const draftKey = `${agentId}:${selectedConversationIds.value[agentId] ?? "new"}`;
      const conversation =
        conversationFor(agentId) ?? (await newConversation(agentId, prompt.slice(0, 80)));
      setDraft(agentId, prompt);
      const task = await api.submitTask(agentId, prompt, conversation.id);
      if (owner !== sessionGeneration) return;
      drafts.value[draftKey] = "";
      setDraft(agentId, "");
      changeConversations(agentId);
      const current = conversations.value[agentId]?.find((item) => item.id === conversation.id);
      if (current && !current.task_ids.includes(task.id)) {
        current.task_ids.push(task.id);
        current.revision += 1;
      }
      runs.value[agentId] = {
        status: "accepted",
        output: "",
        error: "",
        taskId: task.id,
      };
      taskHistory.value[agentId] = [...historyFor(agentId), task];
      streamTask(agentId, task.id);
    } catch (error) {
      if (owner !== sessionGeneration) return;
      runs.value[agentId] = {
        status: "failed",
        output: "",
        error: errorMessage(error),
      };
    }
  }

  async function submitCodingTurn(
    sessionId: string,
    agentId: string,
    input: SubmitCodingTurn,
  ): Promise<Task | undefined> {
    runs.value[agentId] = { status: "submitting", output: "", error: "" };
    try {
      const task = await api.submitCodingTurn(sessionId, agentId, input);
      runs.value[agentId] = {
        status: "accepted",
        output: "",
        error: "",
        taskId: task.id,
      };
      taskHistory.value[agentId] = [...historyFor(agentId), task];
      streamTask(agentId, task.id);
      return task;
    } catch (error) {
      runs.value[agentId] = {
        status: "failed",
        output: "",
        error: errorMessage(error),
      };
      return undefined;
    }
  }

  function streamTask(agentId: string, taskId: string) {
    streams.get(agentId)?.close();
    const events = new EventSource(`/api/tasks/${encodeURIComponent(taskId)}/events`);
    streams.set(agentId, events);
    events.addEventListener("task", (rawEvent) => {
      const event = rawEvent as MessageEvent<string>;
      try {
        const envelope = JSON.parse(event.data) as TaskEventEnvelope;
        const update = envelope.event;
        if (!update) return;
        const current = runFor(agentId);
        if (update.kind === "accepted") current.status = "accepted";
        if (update.kind === "running") current.status = "running";
        if (update.kind === "text_delta") {
          current.status = "running";
          current.output += update.text;
        }
        if (update.kind === "context_changed") {
          current.contextRevision = update.revision;
        }
        if (update.kind === "inventory_changed") {
          current.inventoryRevision = update.revision;
          current.publishedTools = update.published_tools;
        }
        if (update.kind === "approval_requested") {
          current.status = "awaiting_approval";
          void loadTaskApprovals(agentId, taskId);
        }
        if (update.kind === "approval_resolved") {
          current.status = "running";
          current.approvals = (current.approvals ?? []).filter(
            (approval) => approval.id !== update.approval_id,
          );
          current.resolvingApprovalId = undefined;
          current.approvalError = undefined;
        }
        if (update.kind === "succeeded") {
          current.status = "succeeded";
          current.output = update.output;
          current.approvals = [];
          events.close();
          streams.delete(agentId);
          updateHistory(agentId, taskId, { status: "succeeded", output: update.output });
        }
        if (update.kind === "failed") {
          current.status = "failed";
          current.error = taskFailureMessage(update.failure);
          current.approvals = [];
          events.close();
          streams.delete(agentId);
          updateHistory(agentId, taskId, {
            status: "failed",
            failure_code: update.failure?.code,
            failure_message: update.failure?.message,
          });
        }
        if (!["succeeded", "failed"].includes(update.kind)) {
          updateHistory(agentId, taskId, { status: current.status, output: current.output });
        }
        runs.value[agentId] = { ...current };
      } catch {
        const current = runFor(agentId);
        current.error = "Devcenter received an invalid task event.";
        runs.value[agentId] = { ...current };
      }
    });
    events.onerror = () => {
      const current = runFor(agentId);
      if (events.readyState === EventSource.CLOSED) {
        if (current.status !== "succeeded" && current.status !== "failed") {
          current.status = "failed";
          current.error = "The task stream closed before a final result arrived.";
        }
      } else {
        current.status = "reconnecting";
      }
      runs.value[agentId] = { ...current };
    };
  }

  async function loadTaskApprovals(agentId: string, taskId: string) {
    const current = runFor(agentId);
    try {
      current.approvals = await api.taskApprovals(taskId);
      current.approvalError = undefined;
    } catch (error) {
      current.approvalError = errorMessage(error);
    }
    runs.value[agentId] = { ...current };
  }

  async function resolveTaskApproval(
    agentId: string,
    approvalId: string,
    decision: "approve" | "deny",
  ) {
    const current = runFor(agentId);
    if (!current.taskId) return;
    current.resolvingApprovalId = approvalId;
    current.approvalError = undefined;
    runs.value[agentId] = { ...current };
    try {
      await api.resolveTaskApproval(
        current.taskId,
        approvalId,
        decision === "approve"
          ? { decision: "approve" }
          : { decision: "deny", reason: "Denied by the person in Devcenter" },
      );
      current.approvals = (current.approvals ?? []).filter(
        (approval) => approval.id !== approvalId,
      );
      current.status = "running";
    } catch (error) {
      current.approvalError = errorMessage(error);
    } finally {
      current.resolvingApprovalId = undefined;
      runs.value[agentId] = { ...current };
    }
  }

  function clearNotice() {
    notice.value = "";
  }

  return {
    sessionState,
    session,
    sessionError,
    identityProviders,
    agentsState,
    agents,
    agentsError,
    capabilityProfiles,
    taskHistory,
    conversations,
    selectedAgentId,
    selectedAgent,
    connectionState,
    connected,
    connectionError,
    oauthFlow,
    notice,
    bootstrap,
    logout,
    loadAgents,
    loadCapabilityProfiles,
    loadConnection,
    startOAuth,
    completeOAuth,
    cancelOAuth,
    disconnect,
    createAgent,
    updateAgent,
    deleteAgent,
    conversationFor,
    loadConversations,
    selectConversation,
    newConversation,
    renameConversation,
    removeConversation,
    conversationHistoryFor,
    selectAgent,
    loadAgentTasks,
    historyFor,
    draftFor,
    setDraft,
    runFor,
    submitTask,
    submitCodingTurn,
    resolveTaskApproval,
    clearNotice,
  };
});
