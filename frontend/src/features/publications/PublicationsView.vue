<script setup lang="ts">
import {
  Ban,
  Check,
  CircleAlert,
  Clipboard,
  Pause,
  Play,
  Plus,
  RadioTower,
  RefreshCw,
  ShieldCheck,
  Unlink,
} from "@lucide/vue";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
import {
  api,
  errorMessage,
  type Approval,
  type CapabilityProfile,
  type ClientAuthorization,
  type Publication,
  type PublicationState,
} from "@/api/client";

const route = useRoute();
const router = useRouter();
const loading = ref(true);
const publications = ref<Publication[]>([]);
const profiles = ref<CapabilityProfile[]>([]);
const selectedId = ref<string | undefined>(
  typeof route.query.publication === "string" ? route.query.publication : undefined,
);
const clients = ref<ClientAuthorization[]>([]);
const approvals = ref<Approval[]>([]);
const profileId = ref(typeof route.query.profile === "string" ? route.query.profile : "");
const error = ref("");
const notice = ref("");
const mutating = ref(false);

const selected = computed(() =>
  publications.value.find((publication) => publication.publication_id === selectedId.value),
);
const endpoint = computed(() =>
  selected.value ? `${window.location.origin}/mcp/${selected.value.publication_id}` : "",
);
const oauthResource = computed(() => `${window.location.origin}/mcp`);
const codexSetup = computed(
  () =>
    `codex mcp add devcenter --url ${endpoint.value} --oauth-client-id devcenter-cli --oauth-resource ${oauthResource.value} && codex mcp login devcenter --scopes mcp.tools.call`,
);
const claudeSetup = computed(
  () => `claude mcp add --transport http --client-id devcenter-cli devcenter ${endpoint.value}`,
);

async function load() {
  loading.value = true;
  error.value = "";
  try {
    [publications.value, profiles.value] = await Promise.all([
      api.publications(),
      api.capabilityProfiles(),
    ]);
    if (
      !publications.value.some((publication) => publication.publication_id === selectedId.value)
    ) {
      selectedId.value = publications.value[0]?.publication_id;
    }
    await loadDetail();
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    loading.value = false;
  }
}

async function loadDetail() {
  if (!selectedId.value) {
    clients.value = [];
    approvals.value = [];
    return;
  }
  const [nextClients, nextApprovals] = await Promise.all([
    api.publicationClients(selectedId.value),
    api.publicationApprovals(selectedId.value),
  ]);
  clients.value = nextClients;
  approvals.value = nextApprovals;
}

async function choose(publicationId: string) {
  selectedId.value = publicationId;
  await router.replace({ path: "/publications", query: { publication: publicationId } });
  error.value = "";
  try {
    await loadDetail();
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function publish() {
  if (!profileId.value.trim()) return;
  mutating.value = true;
  error.value = "";
  try {
    const publication = await api.publishProfile(profileId.value.trim());
    publications.value.unshift(publication);
    selectedId.value = publication.publication_id;
    profileId.value = "";
    notice.value = "Capability profile published.";
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    mutating.value = false;
  }
}

async function changeState(state: PublicationState) {
  if (!selected.value) return;
  mutating.value = true;
  error.value = "";
  try {
    const changed = await api.changePublicationState(selected.value.publication_id, state);
    publications.value = publications.value.map((publication) =>
      publication.publication_id === changed.publication_id ? changed : publication,
    );
    notice.value =
      state === "active"
        ? "Publication resumed."
        : state === "revoked"
          ? "Publication revoked."
          : "Publication suspended.";
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    mutating.value = false;
  }
}

async function revokeClient(client: ClientAuthorization) {
  if (!selected.value) return;
  error.value = "";
  try {
    await api.revokePublicationClient(selected.value.publication_id, client.authorization_id);
    await loadDetail();
  } catch (cause) {
    error.value = errorMessage(cause);
  }
}

async function copy(value: string, message: string) {
  await window.navigator.clipboard.writeText(value);
  notice.value = message;
}

function shortDigest(digest: string) {
  return `${digest.slice(0, 12)}…${digest.slice(-8)}`;
}

function profileName(id: string) {
  return profiles.value.find((profile) => profile.id === id)?.name ?? id;
}

onMounted(() => void load());
watch(
  () => route.query.publication,
  (publicationId) => {
    if (
      typeof publicationId === "string" &&
      publicationId !== selectedId.value &&
      publications.value.some((publication) => publication.publication_id === publicationId)
    ) {
      selectedId.value = publicationId;
      void loadDetail();
    }
  },
);
</script>

<template>
  <div class="view publications-view">
    <header class="view-header">
      <div>
        <p class="eyebrow">Personal remote MCP</p>
        <h1>Publish governed tools</h1>
        <p>Reuse one immutable capability profile through separately authorized MCP clients.</p>
      </div>
      <button class="button quiet" type="button" :disabled="loading" @click="load">
        <RefreshCw :size="16" :class="{ spinning: loading }" /> Refresh
      </button>
    </header>

    <p v-if="error" class="form-error publication-error" role="alert">
      <CircleAlert :size="16" /> {{ error }}
    </p>
    <p v-if="notice" class="publication-notice" role="status"><Check :size="16" /> {{ notice }}</p>

    <section class="publish-bar">
      <div>
        <strong>Publish a capability profile</strong>
        <span>The active immutable profile revision becomes this endpoint’s exact toolset.</span>
      </div>
      <form v-if="profiles.length" @submit.prevent="publish">
        <label class="sr-only" for="profile-id">Capability profile ID</label>
        <select id="profile-id" v-model="profileId">
          <option value="" disabled>Choose a capability profile</option>
          <option v-for="profile in profiles" :key="profile.id" :value="profile.id">
            {{ profile.name }} · revision {{ profile.revision }}
          </option>
        </select>
        <button class="button primary" type="submit" :disabled="mutating || !profileId.trim()">
          <Plus :size="16" /> Publish
        </button>
      </form>
      <button v-else class="button primary" type="button" @click="router.push('/profiles')">
        <Plus :size="16" /> Create a profile
      </button>
    </section>

    <div v-if="publications.length" class="publication-layout">
      <aside class="publication-list" aria-label="MCP publications">
        <button
          v-for="publication in publications"
          :key="publication.publication_id"
          type="button"
          :class="{ selected: publication.publication_id === selectedId }"
          @click="choose(publication.publication_id)"
        >
          <span class="publication-icon"><RadioTower :size="17" /></span>
          <span
            ><strong>{{ profileName(publication.profile_id) }}</strong
            ><small>Revision {{ publication.active_revision }}</small></span
          >
          <span class="status-dot" :class="publication.state"></span>
        </button>
      </aside>

      <main v-if="selected" class="publication-detail">
        <header>
          <div>
            <p class="eyebrow">Stable endpoint</p>
            <h2>{{ profileName(selected.profile_id) }}</h2>
          </div>
          <span class="status-pill" :class="selected.state">{{ selected.state }}</span>
        </header>

        <div class="endpoint-row">
          <code>{{ endpoint }}</code>
          <button
            class="icon-button"
            type="button"
            aria-label="Copy endpoint"
            @click="copy(endpoint, 'Endpoint copied.')"
          >
            <Clipboard :size="17" />
          </button>
        </div>
        <dl class="publication-facts">
          <div>
            <dt>Publication</dt>
            <dd>{{ selected.publication_id }}</dd>
          </div>
          <div>
            <dt>Active revision</dt>
            <dd>{{ selected.active_revision }}</dd>
          </div>
          <div>
            <dt>Toolset digest</dt>
            <dd :title="selected.toolset_digest">{{ shortDigest(selected.toolset_digest) }}</dd>
          </div>
        </dl>

        <div class="publication-actions">
          <button
            v-if="selected.state === 'active'"
            class="button quiet"
            type="button"
            :disabled="mutating"
            @click="changeState('suspended')"
          >
            <Pause :size="16" /> Suspend every client
          </button>
          <button
            v-if="selected.state === 'suspended'"
            class="button quiet"
            type="button"
            :disabled="mutating"
            @click="changeState('active')"
          >
            <Play :size="16" /> Resume publication
          </button>
          <button
            v-if="selected.state !== 'revoked'"
            class="button danger-quiet"
            type="button"
            :disabled="mutating"
            @click="changeState('revoked')"
          >
            <Ban :size="16" /> Revoke permanently
          </button>
        </div>

        <section class="client-setup">
          <h3>Client setup</h3>
          <div>
            <strong>Codex</strong><code>{{ codexSetup }}</code
            ><button
              class="icon-button"
              type="button"
              aria-label="Copy Codex setup"
              @click="copy(codexSetup, 'Codex setup copied.')"
            >
              <Clipboard :size="15" />
            </button>
          </div>
          <div>
            <strong>Claude Code</strong><code>{{ claudeSetup }}</code
            ><button
              class="icon-button"
              type="button"
              aria-label="Copy Claude setup"
              @click="copy(claudeSetup, 'Claude Code setup copied.')"
            >
              <Clipboard :size="15" />
            </button>
          </div>
          <p>
            <ShieldCheck :size="15" /> Each client completes its own OAuth authorization. Browser
            logout does not revoke it.
          </p>
        </section>

        <section class="publication-section">
          <h3>
            Authorized clients <span>{{ clients.length }}</span>
          </h3>
          <p v-if="!clients.length" class="section-empty">
            No Claude or Codex client has used this publication.
          </p>
          <div v-for="client in clients" :key="client.authorization_id" class="client-row">
            <div>
              <strong>{{ client.display_name }}</strong
              ><span>{{ client.client_id }} · {{ client.state }}</span>
            </div>
            <button
              v-if="client.state === 'active'"
              class="button danger-quiet small"
              type="button"
              @click="revokeClient(client)"
            >
              <Unlink :size="14" /> Revoke
            </button>
          </div>
        </section>

        <section class="publication-section">
          <h3>
            Pending approvals <span>{{ approvals.length }}</span>
          </h3>
          <p v-if="!approvals.length" class="section-empty">
            No effect-bearing tool call is waiting for approval.
          </p>
          <div v-for="approval in approvals" :key="approval.approval_id" class="approval-row">
            <div>
              <strong>{{ approval.tool_name }}</strong
              ><span
                >{{ approval.operation_ref }} · expires
                {{ new Date(approval.expires_at_ms).toLocaleTimeString() }}</span
              >
            </div>
          </div>
        </section>
      </main>
    </div>
    <section v-else-if="!loading" class="publication-empty">
      <RadioTower :size="30" />
      <h2>No profiles are published</h2>
      <p>Publish a granted capability profile to create its stable, opaque MCP endpoint.</p>
    </section>
  </div>
</template>
