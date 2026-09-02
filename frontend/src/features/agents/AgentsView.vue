<script setup lang="ts">
import {
  ArrowUp,
  Bot,
  Check,
  CircleAlert,
  Clock3,
  Plus,
  RefreshCw,
  RotateCw,
  ShieldCheck,
  Sparkles,
  X,
} from "@lucide/vue";
import { computed, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import NewAgentDialog from "./NewAgentDialog.vue";
import { useWorkspaceStore, type AgentRun } from "@/stores/workspace";

const workspace = useWorkspaceStore();
const route = useRoute();
const router = useRouter();
const showCreate = ref(false);

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

function runLabel() {
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
  return labels[run.value.status] ?? "Ready";
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

    <div class="workspace-grid">
      <aside class="roster-card">
        <header class="card-heading">
          <div>
            <h2>Agents</h2>
            <span>{{ workspace.agents.length }} available</span>
          </div>
          <button
            class="icon-button"
            type="button"
            aria-label="Refresh agents"
            :disabled="workspace.agentsState === 'loading'"
            @click="workspace.loadAgents"
          >
            <RefreshCw :size="17" :class="{ spinning: workspace.agentsState === 'loading' }" />
          </button>
        </header>

        <div
          v-if="workspace.agentsState === 'loading' && !workspace.agents.length"
          class="roster-loading"
          aria-label="Loading agents"
        >
          <span v-for="item in 3" :key="item"></span>
        </div>
        <div v-else-if="workspace.agentsState === 'error'" class="inline-state error-state">
          <CircleAlert :size="20" /><strong>Agents unavailable</strong>
          <p>{{ workspace.agentsError }}</p>
          <button class="button small" type="button" @click="workspace.loadAgents">
            Try again
          </button>
        </div>
        <div v-else-if="!workspace.agents.length" class="inline-state empty-state">
          <span class="empty-icon"><Bot :size="25" /></span><strong>No agents yet</strong>
          <p>Create the first governed worker for this workspace.</p>
          <button class="button primary small" type="button" @click="showCreate = true">
            <Plus :size="15" /> Create agent
          </button>
        </div>
        <div v-else class="agent-list" role="listbox" aria-label="Agents">
          <button
            v-for="agent in workspace.agents"
            :key="agent.id"
            class="agent-option"
            :class="{ selected: agent.id === workspace.selectedAgentId }"
            type="button"
            role="option"
            :aria-selected="agent.id === workspace.selectedAgentId"
            @click="choose(agent.id)"
          >
            <span class="agent-avatar"><Bot :size="18" /></span>
            <span class="agent-option-copy"
              ><strong>{{ agent.name }}</strong
              ><small>Revision {{ agent.active_revision ?? "inactive" }}</small></span
            >
            <Check v-if="agent.id === workspace.selectedAgentId" :size="16" />
          </button>
        </div>
        <footer v-if="workspace.agents.length" class="roster-footer">
          <button type="button" @click="showCreate = true">
            <Plus :size="15" /> Add another agent
          </button>
        </footer>
      </aside>

      <main v-if="selected" class="task-workspace">
        <header class="agent-workspace-header">
          <div class="agent-identity">
            <span class="large-agent-avatar"><Bot :size="24" /></span>
            <div>
              <p>Selected agent</p>
              <h2>{{ selected.name }}</h2>
            </div>
          </div>
          <div class="agent-facts">
            <span
              ><Sparkles :size="15" /> Revision {{ selected.active_revision ?? "inactive" }}</span
            >
            <span><Clock3 :size="15" /> Created {{ formatDate(selected.created_at_ms) }}</span>
          </div>
        </header>

        <section class="composer-card">
          <div class="composer-heading">
            <div>
              <h3>What should this agent accomplish?</h3>
              <p>Give it a concrete outcome and the context needed to produce evidence.</p>
            </div>
            <span class="connection-indicator" :class="{ connected: workspace.connected }">
              <span></span>{{ workspace.connected ? "Model connected" : "Connection unchecked" }}
            </span>
          </div>
          <label class="sr-only" :for="`prompt-${selected.id}`">Task instructions</label>
          <textarea
            :id="`prompt-${selected.id}`"
            class="task-prompt"
            :value="workspace.draftFor(selected.id)"
            rows="7"
            placeholder="Prepare the next release. Verify every gate, summarize notable changes, and stop before publication."
            :disabled="taskActive"
            @input="workspace.setDraft(selected.id, ($event.target as HTMLTextAreaElement).value)"
          ></textarea>
          <footer class="composer-footer">
            <span>Authority is revalidated for every attempt.</span>
            <button
              class="button primary run-button"
              type="button"
              :disabled="taskActive || !workspace.draftFor(selected.id).trim()"
              @click="workspace.submitTask(selected.id)"
            >
              <RotateCw v-if="taskActive" :size="17" class="spinning" />
              <ArrowUp v-else :size="17" />
              {{ taskActive ? runLabel() + "…" : "Run task" }}
            </button>
          </footer>
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

        <section class="run-card" :class="`run-${run.status}`">
          <header class="run-header">
            <div>
              <span class="run-light"></span>
              <h3>Latest attempt</h3>
            </div>
            <span class="status-pill" :class="run.status">{{ runLabel() }}</span>
          </header>
          <div v-if="run.status === 'idle'" class="run-empty">
            <div class="terminal-mark"><span></span><span></span><span></span></div>
            <p>Task output will appear here as ordered events arrive.</p>
          </div>
          <div v-else class="run-output-wrap">
            <div class="run-meta" aria-live="polite">
              <span v-if="run.taskId">Task {{ run.taskId.slice(0, 12) }}</span>
              <span>{{ runLabel() }}</span>
            </div>
            <pre class="run-output" tabindex="0">{{
              run.output || (run.error ? "" : "Waiting for output…")
            }}</pre>
            <p v-if="run.error" class="run-error" role="alert">
              <CircleAlert :size="16" /> {{ run.error }}
            </p>
          </div>
        </section>
      </main>

      <main v-else class="task-workspace workspace-placeholder">
        <span class="empty-icon large"><Bot :size="30" /></span>
        <h2>Create an agent to begin</h2>
        <p>
          Agents hold versioned instructions and model routes. Tasks bind each run to the active
          revision.
        </p>
        <button class="button primary" type="button" @click="showCreate = true">
          <Plus :size="17" /> New agent
        </button>
      </main>
    </div>
    <NewAgentDialog v-if="showCreate" @close="showCreate = false" />
  </div>
</template>
