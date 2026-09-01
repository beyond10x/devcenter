import { defineStore } from "pinia";
import { computed, ref } from "vue";
import {
  ApiError,
  api,
  errorMessage,
  type Agent,
  type ClaudeOAuthStart,
  type Session,
  type TaskEventEnvelope,
} from "@/api/client";

export type LoadState = "idle" | "loading" | "ready" | "error";
export type RunState =
  "idle" | "submitting" | "accepted" | "running" | "succeeded" | "failed" | "reconnecting";

export interface AgentRun {
  status: RunState;
  output: string;
  error: string;
  taskId?: string;
}

const streams = new Map<string, EventSource>();

export const useWorkspaceStore = defineStore("workspace", () => {
  const sessionState = ref<LoadState>("loading");
  const session = ref<Session>();
  const sessionError = ref("");
  const agentsState = ref<LoadState>("idle");
  const agents = ref<Agent[]>([]);
  const agentsError = ref("");
  const selectedAgentId = ref<string>();
  const drafts = ref<Record<string, string>>({});
  const runs = ref<Record<string, AgentRun>>({});
  const connectionState = ref<LoadState>("idle");
  const connected = ref(false);
  const connectionError = ref("");
  const oauthFlow = ref<ClaudeOAuthStart>();
  const notice = ref("");

  const selectedAgent = computed(() =>
    agents.value.find((agent) => agent.id === selectedAgentId.value),
  );

  async function bootstrap() {
    sessionState.value = "loading";
    sessionError.value = "";
    try {
      session.value = await api.session();
      sessionState.value = "ready";
      await Promise.allSettled([loadAgents(), loadConnection()]);
    } catch (error) {
      if (error instanceof ApiError && error.status === 401) {
        sessionState.value = "idle";
      } else {
        sessionState.value = "error";
        sessionError.value = errorMessage(error);
      }
    }
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
    } catch (error) {
      agentsState.value = "error";
      agentsError.value = errorMessage(error);
    }
  }

  async function loadConnection() {
    connectionState.value = "loading";
    connectionError.value = "";
    try {
      const status = await api.connection();
      connected.value = status.connected;
      connectionState.value = "ready";
      if (status.connected) oauthFlow.value = undefined;
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
      const status = await api.completeOAuth(flowId, code);
      connected.value = status.connected;
      oauthFlow.value = undefined;
      notice.value = "Claude Code is connected and ready for governed attempts.";
    } catch (error) {
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

  async function createAgent(input: { name: string; instructions: string; model: string }) {
    const created = await api.createAgent(input);
    agents.value = [created, ...agents.value.filter((agent) => agent.id !== created.id)];
    selectedAgentId.value = created.id;
    drafts.value[created.id] = "";
    notice.value = `${created.name} was created and activated.`;
    return created;
  }

  function selectAgent(agentId: string) {
    selectedAgentId.value = agentId;
  }

  function draftFor(agentId: string): string {
    return drafts.value[agentId] ?? "";
  }

  function setDraft(agentId: string, value: string) {
    drafts.value[agentId] = value;
  }

  function runFor(agentId: string): AgentRun {
    return runs.value[agentId] ?? { status: "idle", output: "", error: "" };
  }

  async function submitTask(agentId: string) {
    const prompt = draftFor(agentId).trim();
    if (!prompt) return;
    runs.value[agentId] = { status: "submitting", output: "", error: "" };
    try {
      const task = await api.submitTask(agentId, prompt);
      drafts.value[agentId] = "";
      runs.value[agentId] = {
        status: "accepted",
        output: "",
        error: "",
        taskId: task.id,
      };
      streamTask(agentId, task.id);
    } catch (error) {
      runs.value[agentId] = {
        status: "failed",
        output: "",
        error: errorMessage(error),
      };
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
        if (update.kind === "succeeded") {
          current.status = "succeeded";
          current.output = update.output;
          events.close();
          streams.delete(agentId);
        }
        if (update.kind === "failed") {
          current.status = "failed";
          current.error = update.failure?.message ?? "The task failed without a reason.";
          events.close();
          streams.delete(agentId);
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

  function clearNotice() {
    notice.value = "";
  }

  return {
    sessionState,
    session,
    sessionError,
    agentsState,
    agents,
    agentsError,
    selectedAgentId,
    selectedAgent,
    connectionState,
    connected,
    connectionError,
    oauthFlow,
    notice,
    bootstrap,
    loadAgents,
    loadConnection,
    startOAuth,
    completeOAuth,
    cancelOAuth,
    disconnect,
    createAgent,
    selectAgent,
    draftFor,
    setDraft,
    runFor,
    submitTask,
    clearNotice,
  };
});
