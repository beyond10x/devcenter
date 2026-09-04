<script setup lang="ts">
import {
  ArrowLeft,
  Boxes,
  CircleDot,
  GitBranch,
  LibraryBig,
  RefreshCw,
  Route,
  Workflow as WorkflowIcon,
} from "@lucide/vue";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  api,
  errorMessage,
  type WorkflowLibraryDetail,
  type WorkflowLibrarySummary,
  type WorkflowRevisionSummary,
} from "@/api/client";

const route = useRoute();
const router = useRouter();
const workflows = ref<WorkflowLibrarySummary[]>([]);
const detail = ref<WorkflowLibraryDetail>();
const loading = ref(false);
const failure = ref("");
const partial = ref(false);
const installing = ref(false);

const selectedId = computed(() =>
  typeof route.params.workflowId === "string" ? route.params.workflowId : "",
);
const activeRevision = computed(() => {
  const activeId = detail.value?.workflow.active_revision_id;
  return detail.value?.revisions.find((revision) => revision.id === activeId);
});

onMounted(loadLibrary);
watch(selectedId, () => void loadSelection());

async function loadLibrary() {
  loading.value = true;
  failure.value = "";
  try {
    const page = await api.workflowLibrary();
    workflows.value = page.workflows;
    partial.value = page.partial;
    await loadSelection();
  } catch (caught) {
    failure.value = errorMessage(caught);
  } finally {
    loading.value = false;
  }
}

async function loadSelection() {
  detail.value = undefined;
  if (!selectedId.value) return;
  loading.value = true;
  failure.value = "";
  try {
    detail.value = await api.workflowLibraryItem(selectedId.value);
  } catch (caught) {
    failure.value = errorMessage(caught);
  } finally {
    loading.value = false;
  }
}

async function installStarterLibrary() {
  installing.value = true;
  failure.value = "";
  try {
    const page = await api.installStarterWorkflowLibrary();
    workflows.value = page.workflows;
    partial.value = page.partial;
  } catch (caught) {
    failure.value = errorMessage(caught);
  } finally {
    installing.value = false;
  }
}

function chooseWorkflow(event: Event) {
  const id = (event.target as HTMLSelectElement).value;
  void router.push(id ? { name: "workflow", params: { workflowId: id } } : { name: "workflows" });
}

function inspectWorkflow(workflow: WorkflowLibrarySummary) {
  void router.push({ name: "workflow", params: { workflowId: workflow.id } });
}

function nodeRecord(node: unknown): Record<string, unknown> | undefined {
  return typeof node === "object" && node !== null ? (node as Record<string, unknown>) : undefined;
}

function nodeKind(node: unknown): string {
  const definition = nodeRecord(nodeRecord(node)?.definition);
  return typeof definition?.kind === "string" ? definition.kind : "node";
}

function nodeDetail(node: unknown): string {
  const definition = nodeRecord(nodeRecord(node)?.definition);
  const value = nodeRecord(definition?.value);
  for (const field of ["operation", "instruction", "condition", "event", "result", "expression"]) {
    if (typeof value?.[field] === "string") return value[field];
  }
  return "Configured step";
}

function nodeId(node: unknown, index: number): string {
  const id = nodeRecord(node)?.node_id;
  return typeof id === "string" ? id : `node-${String(index + 1)}`;
}

function shortId(value?: string | null): string {
  if (!value) return "Not published";
  return value.length > 16 ? `${value.slice(0, 8)}…${value.slice(-6)}` : value;
}

function revisionLabel(revision: WorkflowRevisionSummary): string {
  return revision.id === detail.value?.workflow.active_revision_id
    ? "Active revision"
    : "Published revision";
}
</script>

<template>
  <main class="workflow-library-view">
    <header class="page-header">
      <div>
        <p class="eyebrow">Standalone service</p>
        <h1>Workflow library</h1>
        <p>
          Inspect reusable definitions and immutable published graphs. Project-bound workflows stay
          with their Workspace project.
        </p>
      </div>
      <button class="button quiet" type="button" :disabled="loading" @click="loadLibrary">
        <RefreshCw :size="16" :class="{ spinning: loading }" /> Refresh
      </button>
    </header>

    <section v-if="workflows.length" class="workflow-selector-shell" aria-label="Workflow selector">
      <WorkflowIcon :size="24" aria-hidden="true" />
      <label>
        <span>Current workflow</span>
        <select :value="selectedId" @change="chooseWorkflow">
          <option value="">Browse all workflows</option>
          <option v-for="workflow in workflows" :key="workflow.id" :value="workflow.id">
            {{ workflow.name }}
          </option>
        </select>
      </label>
      <span v-if="detail" class="status-pill"
        ><CircleDot :size="13" /> {{ detail.workflow.state }}</span
      >
    </section>

    <div v-if="failure" class="empty-state" role="alert">
      <strong>Workflow library unavailable</strong>
      <span>{{ failure }}</span>
    </div>

    <div v-else-if="loading && workflows.length === 0" class="empty-state" role="status">
      <span class="spinner"></span>
      <span>Loading workflow definitions…</span>
    </div>

    <div v-else-if="workflows.length === 0" class="empty-state">
      <LibraryBig :size="32" />
      <strong>Start with the engineering workflow library</strong>
      <span>Install Code review, Security review, and Reverse AEP + ESS as published graphs.</span>
      <button
        class="button primary"
        type="button"
        :disabled="installing"
        @click="installStarterLibrary"
      >
        <span v-if="installing" class="spinner"></span>
        {{ installing ? "Publishing starter graphs…" : "Install starter library" }}
      </button>
    </div>

    <template v-else-if="!selectedId">
      <div class="section-heading">
        <div>
          <p class="eyebrow">Available definitions</p>
          <h2>{{ workflows.length }} workflow{{ workflows.length === 1 ? "" : "s" }}</h2>
        </div>
        <span v-if="partial" class="muted-note">Showing the first result window</span>
      </div>
      <section class="workflow-card-grid">
        <button
          v-for="workflow in workflows"
          :key="workflow.id"
          class="workflow-card"
          type="button"
          @click="inspectWorkflow(workflow)"
        >
          <span class="workflow-card-icon"><Route :size="22" /></span>
          <span class="workflow-card-copy">
            <span class="workflow-card-kicker">{{ workflow.state }}</span>
            <strong>{{ workflow.name }}</strong>
            <small>{{ shortId(workflow.active_revision_id) }}</small>
          </span>
          <span class="inspect-label">Inspect →</span>
        </button>
      </section>
    </template>

    <template v-else-if="detail">
      <button class="back-link" type="button" @click="router.push({ name: 'workflows' })">
        <ArrowLeft :size="15" /> All workflows
      </button>

      <section class="workflow-hero">
        <div>
          <p class="eyebrow">Definition</p>
          <h2>{{ detail.workflow.name }}</h2>
          <code>{{ detail.workflow.id }}</code>
        </div>
        <div class="workflow-stat-grid">
          <div>
            <span>Published</span>
            <strong>{{ detail.revisions.length }}</strong>
          </div>
          <div>
            <span>Drafts</span>
            <strong>{{ detail.drafts.length }}</strong>
          </div>
          <div>
            <span>Active graph</span>
            <strong>{{ activeRevision?.node_count ?? 0 }} nodes</strong>
          </div>
        </div>
      </section>

      <section v-if="activeRevision" class="active-graph">
        <header>
          <div>
            <p class="eyebrow">Active revision</p>
            <h2>Published graph</h2>
          </div>
          <span class="graph-count"
            ><GitBranch :size="15" /> {{ activeRevision.edge_count }} edges</span
          >
        </header>
        <div class="node-strip">
          <article
            v-for="(node, index) in activeRevision.nodes"
            :key="nodeId(node, index)"
            class="node-card"
          >
            <span>{{ String(index + 1).padStart(2, "0") }}</span>
            <strong>{{ nodeKind(node) }}</strong>
            <small>{{ nodeDetail(node) }}</small>
            <code>{{ shortId(nodeId(node, index)) }}</code>
          </article>
        </div>
        <footer>
          <code>sha256:{{ activeRevision.digest }}</code>
        </footer>
      </section>

      <section class="workflow-detail-columns">
        <article class="detail-panel">
          <header>
            <div>
              <p class="eyebrow">Immutable history</p>
              <h3>Published revisions</h3>
            </div>
            <Boxes :size="20" />
          </header>
          <ul v-if="detail.revisions.length">
            <li v-for="revision in detail.revisions" :key="revision.id">
              <span
                ><strong>{{ revisionLabel(revision) }}</strong
                ><code>{{ shortId(revision.id) }}</code></span
              >
              <small>{{ revision.node_count }} nodes · {{ revision.edge_count }} edges</small>
            </li>
          </ul>
          <p v-else class="panel-empty">Nothing has been published yet.</p>
        </article>

        <article class="detail-panel">
          <header>
            <div>
              <p class="eyebrow">Editable heads</p>
              <h3>Drafts</h3>
            </div>
            <GitBranch :size="20" />
          </header>
          <ul v-if="detail.drafts.length">
            <li v-for="draft in detail.drafts" :key="draft.id">
              <span
                ><strong>{{ draft.name }}</strong
                ><code>{{ shortId(draft.id) }}</code></span
              >
              <small>{{ draft.state }} · from {{ shortId(draft.based_on_revision_id) }}</small>
            </li>
          </ul>
          <p v-else class="panel-empty">No active drafts.</p>
        </article>
      </section>

      <p v-if="detail.partial" class="muted-note">
        This view contains the first bounded result window.
      </p>
    </template>
  </main>
</template>

<style scoped>
.workflow-library-view {
  display: grid;
  gap: 1.25rem;
  padding: clamp(1rem, 3vw, 2.25rem);
}

.page-header,
.workflow-hero,
.active-graph > header,
.detail-panel > header {
  display: flex;
  align-items: end;
  justify-content: space-between;
  gap: 1.5rem;
}

.page-header h1,
.workflow-hero h2,
.section-heading h2,
.active-graph h2,
.detail-panel h3 {
  margin: 0.2rem 0;
}

.page-header h1 {
  font-size: clamp(2rem, 4vw, 3.25rem);
}

.page-header p:last-child {
  max-width: 66ch;
  margin-bottom: 0;
  color: var(--muted);
}

.workflow-selector-shell {
  display: flex;
  align-items: center;
  gap: 0.9rem;
  padding: 0.85rem 1rem;
  background: color-mix(in srgb, var(--surface) 90%, var(--accent) 10%);
  border: 1px solid color-mix(in srgb, var(--line) 70%, var(--accent) 30%);
  border-radius: 14px;
  box-shadow: var(--shadow);
}

.workflow-selector-shell label {
  display: grid;
  flex: 1;
  gap: 0.25rem;
}

.workflow-selector-shell label > span {
  color: var(--muted);
  font-size: 0.68rem;
  font-weight: 800;
  letter-spacing: 0.1em;
  text-transform: uppercase;
}

.workflow-selector-shell select {
  width: 100%;
  padding: 0;
  color: var(--ink);
  font: inherit;
  font-size: 1rem;
  font-weight: 750;
  background: transparent;
  border: 0;
  outline: 0;
  cursor: pointer;
}

.status-pill,
.graph-count {
  display: inline-flex;
  align-items: center;
  gap: 0.35rem;
  padding: 0.35rem 0.6rem;
  color: var(--accent);
  font-size: 0.76rem;
  font-weight: 750;
  background: color-mix(in srgb, var(--accent) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--accent) 28%, var(--line));
  border-radius: 999px;
}

.section-heading {
  display: flex;
  align-items: end;
  justify-content: space-between;
}

.workflow-card-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(17rem, 1fr));
  gap: 0.8rem;
}

.workflow-card {
  display: grid;
  grid-template-columns: auto 1fr auto;
  align-items: center;
  gap: 0.9rem;
  min-height: 7.5rem;
  padding: 1rem;
  color: inherit;
  text-align: left;
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 14px;
  cursor: pointer;
  transition:
    border-color 140ms ease,
    transform 140ms ease,
    box-shadow 140ms ease;
}

.workflow-card:hover {
  border-color: var(--accent);
  box-shadow: var(--shadow);
  transform: translateY(-2px);
}

.workflow-card-icon {
  display: grid;
  width: 2.75rem;
  height: 2.75rem;
  place-items: center;
  color: var(--accent);
  background: color-mix(in srgb, var(--accent) 11%, transparent);
  border-radius: 10px;
}

.workflow-card-copy,
.detail-panel li > span {
  display: grid;
  gap: 0.2rem;
}

.workflow-card-copy strong {
  font-size: 1.05rem;
}

.workflow-card-kicker,
.inspect-label {
  color: var(--muted);
  font-size: 0.7rem;
  font-weight: 750;
  letter-spacing: 0.08em;
  text-transform: uppercase;
}

.workflow-card code,
.workflow-hero code,
.node-card code,
.detail-panel code,
.active-graph footer code {
  color: var(--muted);
  font-size: 0.72rem;
}

.back-link {
  display: inline-flex;
  align-items: center;
  justify-self: start;
  gap: 0.35rem;
  padding: 0;
  color: var(--muted);
  background: none;
  border: 0;
  cursor: pointer;
}

.workflow-hero,
.active-graph,
.detail-panel {
  padding: clamp(1rem, 2vw, 1.5rem);
  background: var(--surface);
  border: 1px solid var(--line);
  border-radius: 16px;
}

.workflow-hero h2 {
  font-size: clamp(1.6rem, 3vw, 2.4rem);
}

.workflow-stat-grid {
  display: grid;
  grid-template-columns: repeat(3, minmax(6.5rem, 1fr));
  gap: 0.5rem;
}

.workflow-stat-grid > div {
  display: grid;
  gap: 0.25rem;
  padding: 0.75rem;
  background: var(--surface-muted);
  border-radius: 10px;
}

.workflow-stat-grid span,
.detail-panel small {
  color: var(--muted);
  font-size: 0.72rem;
}

.node-strip {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(9.5rem, 1fr));
  gap: 0.65rem;
  margin-top: 1rem;
}

.node-card {
  display: grid;
  gap: 0.25rem;
  min-width: 0;
  padding: 0.85rem;
  background: linear-gradient(
    145deg,
    color-mix(in srgb, var(--accent) 8%, var(--surface-muted)),
    var(--surface-muted)
  );
  border: 1px solid color-mix(in srgb, var(--line) 75%, var(--accent) 25%);
  border-radius: 11px;
}

.node-card > span {
  color: var(--accent);
  font-family: var(--font-mono);
  font-size: 0.68rem;
  font-weight: 800;
}

.node-card strong {
  text-transform: capitalize;
}

.node-card code {
  overflow: hidden;
  text-overflow: ellipsis;
}

.node-card small {
  display: -webkit-box;
  overflow: hidden;
  color: var(--muted);
  font-size: 0.76rem;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 3;
}

.active-graph footer {
  margin-top: 0.9rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.workflow-detail-columns {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.8rem;
}

.detail-panel ul {
  display: grid;
  gap: 0;
  padding: 0;
  margin: 0.8rem 0 0;
  list-style: none;
}

.detail-panel li {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.8rem;
  padding: 0.75rem 0;
  border-top: 1px solid var(--line);
}

.panel-empty,
.muted-note {
  color: var(--muted);
}

.spinning {
  animation: spin 900ms linear infinite;
}

@keyframes spin {
  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 760px) {
  .page-header,
  .workflow-hero,
  .active-graph > header {
    align-items: start;
    flex-direction: column;
  }

  .workflow-detail-columns,
  .workflow-stat-grid {
    grid-template-columns: 1fr;
    width: 100%;
  }

  .workflow-selector-shell {
    align-items: end;
  }

  .status-pill {
    display: none;
  }
}
</style>
