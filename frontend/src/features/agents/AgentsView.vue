<script setup lang="ts">
import {
  ArrowUp,
  Bot,
  Check,
  ChevronDown,
  CircleAlert,
  Clock3,
  Plus,
  Pencil,
  Trash2,
  Eraser,
  RefreshCw,
  RotateCw,
  ShieldCheck,
  Sparkles,
  X,
} from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import RenderedMarkdown from "@/components/RenderedMarkdown.vue";
import NewAgentDialog from "./NewAgentDialog.vue";
import ConfirmDialog from "@/components/ConfirmDialog.vue";
import { errorMessage, type AgentConversation } from "@/api/client";
import { useWorkspaceStore, type AgentRun } from "@/stores/workspace";

const workspace = useWorkspaceStore();
const route = useRoute();
const router = useRouter();
const showCreate = ref(false);
const editingAgentId = ref<string>();
const conversationError = ref("");
const conversationLoading = ref(false);
const mutating = ref(false);
const editingTitle = ref(false);
const titleDraft = ref("");
const confirmation = ref<{
  title: string;
  description: string;
  action: string;
  agentId?: string;
  conversation?: AgentConversation;
  clear?: boolean;
}>();
const confirmationError = ref("");

const selected = computed(() => workspace.selectedAgent);
const run = computed<AgentRun>(() =>
  selected.value ? workspace.runFor(selected.value.id) : { status: "idle", output: "", error: "" },
);
const taskActive = computed(() =>
  ["submitting", "accepted", "running", "awaiting_approval", "reconnecting"].includes(
    run.value.status,
  ),
);
const approvals = computed(() => run.value.approvals ?? []);
const history = computed(() =>
  selected.value ? workspace.conversationHistoryFor(selected.value.id) : [],
);
const conversation = computed(() =>
  selected.value ? workspace.conversationFor(selected.value.id) : undefined,
);
const conversations = computed(() =>
  selected.value ? (workspace.conversations[selected.value.id] ?? []) : [],
);

async function refreshConversations() {
  const id = selected.value?.id;
  conversationError.value = "";
  conversationLoading.value = Boolean(id);
  editingTitle.value = false;
  if (!id) return;
  try {
    await workspace.loadConversations(id);
  } catch (error) {
    if (workspace.selectedAgentId === id) conversationError.value = errorMessage(error);
  } finally {
    if (workspace.selectedAgentId === id) conversationLoading.value = false;
  }
}

async function refresh() {
  await workspace.loadAgents();
  await refreshConversations();
}

watch(() => selected.value?.id, refreshConversations, { immediate: true });

async function createConversation() {
  if (!selected.value || mutating.value) return;
  mutating.value = true;
  conversationError.value = "";
  try {
    await workspace.newConversation(selected.value.id);
  } catch (error) {
    conversationError.value = errorMessage(error);
  } finally {
    mutating.value = false;
  }
}

async function renameConversation() {
  if (!conversation.value || !titleDraft.value.trim()) return;
  mutating.value = true;
  conversationError.value = "";
  try {
    await workspace.renameConversation(conversation.value, titleDraft.value.trim());
    editingTitle.value = false;
  } catch (error) {
    conversationError.value = errorMessage(error);
  } finally {
    mutating.value = false;
  }
}

function confirmAgentDelete() {
  if (!selected.value) return;
  confirmationError.value = "";
  confirmation.value = {
    title: `Delete ${selected.value.name}?`,
    description: "Remove this agent from your roster. Existing task and revision records are kept.",
    action: "Delete agent",
    agentId: selected.value.id,
  };
}

function confirmConversation(clear: boolean) {
  if (!conversation.value) return;
  confirmationError.value = "";
  confirmation.value = {
    title: clear ? "Clear this conversation?" : "Delete this conversation?",
    description: clear
      ? "Start again with an empty conversation under the same name. Previous turns will no longer be included in the conversation."
      : "Remove this conversation from your list. Its task records are kept.",
    action: clear ? "Clear conversation" : "Delete conversation",
    conversation: { ...conversation.value },
    clear,
  };
}

async function confirm() {
  const action = confirmation.value;
  if (!action) return;
  mutating.value = true;
  confirmationError.value = "";
  try {
    if (action.agentId) await workspace.deleteAgent(action.agentId);
    else if (action.conversation)
      await workspace.removeConversation(action.conversation, action.clear);
    confirmation.value = undefined;
  } catch (error) {
    confirmationError.value = errorMessage(error);
  } finally {
    mutating.value = false;
  }
}

watch(
  [() => route.params.agentId, () => workspace.agentsState],
  ([agentId, agentsState]) => {
    if (
      agentsState === "ready" &&
      typeof agentId === "string" &&
      workspace.agents.some((agent) => agent.id === agentId)
    ) {
      workspace.selectAgent(agentId);
    }
  },
  { immediate: true },
);

watch(
  () => workspace.selectedAgentId,
  (agentId) => {
    const routedAgentId = route.params.agentId;
    if (
      workspace.agentsState === "loading" &&
      typeof routedAgentId === "string" &&
      workspace.agents.some((agent) => agent.id === routedAgentId)
    ) {
      return;
    }
    if (agentId && route.params.agentId !== agentId) {
      void router.replace({ name: "agent", params: { agentId } });
    } else if (!agentId && workspace.agentsState === "ready" && route.params.agentId) {
      void router.replace("/agents");
    }
  },
  { flush: "sync" },
);

function choose(agentId: string) {
  workspace.selectAgent(agentId);
  void router.push({ name: "agent", params: { agentId } });
}

function formatDate(value: number) {
  return new Intl.DateTimeFormat(undefined, {
    month: "short",
    day: "numeric",
    year: "numeric",
  }).format(value);
}

function statusLabel(status: string) {
  const labels: Record<string, string> = {
    idle: "Ready",
    submitting: "Submitting",
    accepted: "Accepted",
    running: "Running",
    awaiting_approval: "Awaiting approval",
    reconnecting: "Reconnecting",
    succeeded: "Succeeded",
    failed: "Failed",
  };
  return labels[status] ?? "Ready";
}

function runLabel() {
  return statusLabel(run.value.status);
}

function taskOutput(task: (typeof history.value)[number]) {
  return task.id === run.value.taskId ? run.value.output || task.output : task.output;
}

function taskError(task: (typeof history.value)[number]) {
  if (task.id === run.value.taskId && run.value.error) return run.value.error;
  return task.failure_message;
}

function formatInput(input: unknown) {
  return JSON.stringify(input, null, 2);
}
</script>

<template>
  <div class="view agents-view">
    <header class="view-header">
      <div>
        <p class="eyebrow">Agent operations</p>
        <h1>Direct the work</h1>
        <p>Choose an active agent, describe the outcome, and follow the attempt as it happens.</p>
      </div>
      <button class="button primary" type="button" @click="showCreate = true">
        <Plus :size="17" /> New agent
      </button>
    </header>

    <div class="workspace-grid agent-chat-workspace">
      <main v-if="selected" class="task-workspace">
        <header class="agent-workspace-header">
          <div class="agent-identity">
            <div class="agent-current-control">
              <h2 class="sr-only">{{ selected.name }}</h2>
              <span class="agent-picker-label-row">
                <label :for="`agent-picker-${selected.id}`">Current agent</label>
                <span>{{ workspace.agents.length }} available</span>
              </span>
              <span class="agent-picker-row">
                <span class="large-agent-avatar"><Bot :size="24" /></span>
                <span class="agent-picker-shell">
                  <select
                    :id="`agent-picker-${selected.id}`"
                    class="agent-picker"
                    :value="selected.id"
                    @change="choose(($event.target as HTMLSelectElement).value)"
                  >
                    <option v-for="agent in workspace.agents" :key="agent.id" :value="agent.id">
                      {{ agent.name }}
                    </option>
                  </select>
                  <span class="agent-picker-chevron" aria-hidden="true">
                    <ChevronDown :size="17" />
                  </span>
                </span>
              </span>
            </div>
          </div>
          <div class="agent-header-actions">
            <button class="button quiet small" type="button" @click="editingAgentId = selected.id">
              <Pencil :size="15" /> Edit agent
            </button>
            <button
              class="button danger-quiet small"
              type="button"
              :disabled="taskActive || mutating"
              @click="confirmAgentDelete"
            >
              <Trash2 :size="15" /> Delete agent
            </button>
            <div class="agent-facts">
              <span
                ><Sparkles :size="15" /> Revision {{ selected.active_revision ?? "inactive" }}</span
              >
              <span><Clock3 :size="15" /> Created {{ formatDate(selected.created_at_ms) }}</span>
            </div>
            <button
              class="icon-button"
              type="button"
              aria-label="Refresh agents"
              :disabled="workspace.agentsState === 'loading'"
              @click="refresh"
            >
              <RefreshCw :size="17" :class="{ spinning: workspace.agentsState === 'loading' }" />
            </button>
          </div>
        </header>

        <div class="conversation-toolbar">
          <div class="conversation-picker">
            <label for="agent-conversation">Conversation</label>
            <select
              id="agent-conversation"
              :value="conversation?.id ?? ''"
              :disabled="taskActive || mutating || conversationLoading"
              @change="
                workspace.selectConversation(
                  selected.id,
                  ($event.target as HTMLSelectElement).value,
                );
                editingTitle = false;
              "
            >
              <option v-if="!conversations.length" value="">New conversation</option>
              <option v-for="item in conversations" :key="item.id" :value="item.id">
                {{ item.title }}
              </option>
            </select>
          </div>
          <div class="conversation-actions">
            <button
              class="button quiet small"
              type="button"
              :disabled="taskActive || mutating || conversationLoading"
              @click="createConversation"
            >
              <Plus :size="15" /> New conversation
            </button>
            <button
              v-if="conversation"
              class="icon-button"
              type="button"
              aria-label="Rename conversation"
              :disabled="mutating"
              @click="
                titleDraft = conversation.title;
                editingTitle = true;
              "
            >
              <Pencil :size="15" />
            </button>
            <button
              v-if="conversation"
              class="icon-button"
              type="button"
              aria-label="Clear conversation"
              :disabled="taskActive || mutating"
              @click="confirmConversation(true)"
            >
              <Eraser :size="15" />
            </button>
            <button
              v-if="conversation"
              class="icon-button"
              type="button"
              aria-label="Delete conversation"
              :disabled="taskActive || mutating"
              @click="confirmConversation(false)"
            >
              <Trash2 :size="15" />
            </button>
          </div>
          <form
            v-if="editingTitle"
            class="conversation-rename"
            @submit.prevent="renameConversation"
          >
            <label for="conversation-title">Conversation name</label>
            <input id="conversation-title" v-model="titleDraft" maxlength="160" />
            <button
              class="button primary small"
              :disabled="mutating || !titleDraft.trim()"
              type="submit"
            >
              Save name
            </button>
            <button
              class="button quiet small"
              type="button"
              :disabled="mutating"
              @click="editingTitle = false"
            >
              Cancel
            </button>
          </form>
        </div>
        <p v-if="conversationLoading" class="section-empty" role="status">Loading conversations…</p>
        <p v-if="conversationError" class="form-error" role="alert">{{ conversationError }}</p>
        <p v-if="run.error && !run.taskId" class="form-error" role="alert">{{ run.error }}</p>

        <p v-if="workspace.agentsState === 'error'" class="agent-load-error" role="alert">
          <CircleAlert :size="16" /> {{ workspace.agentsError }}
          <button type="button" @click="refresh">Try again</button>
        </p>

        <section class="agent-chat-panel" aria-live="polite">
          <div v-if="!history.length" class="agent-chat-empty">
            <Sparkles :size="22" />
            <h3>Start a conversation</h3>
            <p>Ask for an outcome. Each turn retains its exact agent revision and authority.</p>
          </div>
          <div v-else class="agent-chat-transcript">
            <article v-for="task in history" :key="task.id" class="agent-chat-turn">
              <div class="chat-message user-message">
                <span>You</span>
                <p>{{ task.prompt }}</p>
              </div>
              <div class="chat-message assistant-message">
                <span>{{ selected.name }} · {{ statusLabel(task.status) }}</span>
                <RenderedMarkdown
                  v-if="taskOutput(task)"
                  class="assistant-output"
                  :source="taskOutput(task) ?? ''"
                />
                <p v-if="taskError(task)" class="assistant-error" role="alert">
                  {{ taskError(task) }}
                </p>
                <p v-else-if="!taskOutput(task)" class="assistant-waiting">
                  <RotateCw
                    v-if="task.id === run.taskId && taskActive"
                    :size="14"
                    class="spinning"
                  />
                  {{
                    task.id === run.taskId && taskActive ? runLabel() + "…" : "No output recorded."
                  }}
                </p>
                <small>Task {{ task.id.slice(0, 12) }}</small>
              </div>
            </article>
          </div>
        </section>

        <section v-if="approvals.length || run.approvalError" class="task-approvals-card">
          <header>
            <div>
              <ShieldCheck :size="18" />
              <div>
                <h3>Human decision required</h3>
                <p>The task is paused. Review the exact Connector call before it can continue.</p>
              </div>
            </div>
            <span>{{ approvals.length }} pending</span>
          </header>
          <article v-for="approval in approvals" :key="approval.id" class="task-approval">
            <div class="task-approval-heading">
              <div>
                <strong>{{ approval.tool_name }}</strong>
                <span>{{ approval.operation_ref }} · {{ approval.connection_ref }}</span>
              </div>
              <small>Call {{ approval.call_id }}</small>
            </div>
            <pre>{{ formatInput(approval.input) }}</pre>
            <footer>
              <span>Only this displayed input is authorized.</span>
              <div>
                <button
                  class="button small"
                  type="button"
                  :disabled="Boolean(run.resolvingApprovalId)"
                  @click="workspace.resolveTaskApproval(selected.id, approval.id, 'deny')"
                >
                  <X :size="14" /> Deny
                </button>
                <button
                  class="button primary small"
                  type="button"
                  :disabled="Boolean(run.resolvingApprovalId)"
                  @click="workspace.resolveTaskApproval(selected.id, approval.id, 'approve')"
                >
                  <RotateCw
                    v-if="run.resolvingApprovalId === approval.id"
                    :size="14"
                    class="spinning"
                  />
                  <Check v-else :size="14" /> Approve exact call
                </button>
              </div>
            </footer>
          </article>
          <p v-if="run.approvalError" class="task-approval-error" role="alert">
            <CircleAlert :size="16" /> {{ run.approvalError }}
          </p>
        </section>

        <section class="agent-chat-composer">
          <label class="sr-only" :for="`prompt-${selected.id}`">Message {{ selected.name }}</label>
          <textarea
            :id="`prompt-${selected.id}`"
            class="task-prompt"
            :value="workspace.draftFor(selected.id)"
            rows="3"
            placeholder="Message this agent…"
            :disabled="taskActive || mutating || conversationLoading || Boolean(conversationError)"
            @input="workspace.setDraft(selected.id, ($event.target as HTMLTextAreaElement).value)"
            @keydown.ctrl.enter.prevent="workspace.submitTask(selected.id)"
            @keydown.meta.enter.prevent="workspace.submitTask(selected.id)"
          ></textarea>
          <footer class="composer-footer">
            <span class="connection-indicator" :class="{ connected: workspace.connected }">
              <span></span>{{ workspace.connected ? "Model connected" : "Connection unchecked" }} ·
              Authority revalidated every turn
            </span>
            <button
              class="button primary run-button"
              type="button"
              :disabled="
                taskActive ||
                mutating ||
                conversationLoading ||
                Boolean(conversationError) ||
                !workspace.draftFor(selected.id).trim()
              "
              @click="workspace.submitTask(selected.id)"
            >
              <RotateCw v-if="taskActive" :size="17" class="spinning" />
              <ArrowUp v-else :size="17" />
              {{ taskActive ? runLabel() + "…" : "Send" }}
            </button>
          </footer>
        </section>
      </main>

      <main v-else class="task-workspace workspace-placeholder">
        <RefreshCw v-if="workspace.agentsState === 'loading'" :size="30" class="spinning" />
        <CircleAlert v-else-if="workspace.agentsState === 'error'" :size="30" class="error-icon" />
        <span v-else class="empty-icon large"><Bot :size="30" /></span>
        <h2>
          {{
            workspace.agentsState === "loading"
              ? "Loading agents…"
              : workspace.agentsState === "error"
                ? "Agents unavailable"
                : "Create an agent to begin"
          }}
        </h2>
        <p>
          {{
            workspace.agentsState === "error"
              ? workspace.agentsError
              : "Agents hold versioned instructions and model routes. Tasks bind each run to the active revision."
          }}
        </p>
        <button
          v-if="workspace.agentsState === 'error'"
          class="button primary"
          type="button"
          @click="refresh"
        >
          <RefreshCw :size="17" /> Try again
        </button>
        <button
          v-else-if="workspace.agentsState !== 'loading'"
          class="button primary"
          type="button"
          @click="showCreate = true"
        >
          <Plus :size="17" /> New agent
        </button>
      </main>
    </div>
    <NewAgentDialog v-if="showCreate" @close="showCreate = false" />
    <NewAgentDialog
      v-if="editingAgentId"
      :agent-id="editingAgentId"
      @close="editingAgentId = undefined"
    />
    <ConfirmDialog
      v-if="confirmation"
      :title="confirmation.title"
      :description="confirmation.description"
      :action="confirmation.action"
      :pending="mutating"
      :error="confirmationError"
      @close="confirmation = undefined"
      @confirm="confirm"
    />
  </div>
</template>
