<script setup lang="ts">
import {
  ArrowUpRight,
  Check,
  CircleAlert,
  KeyRound,
  Link2,
  RefreshCw,
  ShieldCheck,
  Unlink,
} from "@lucide/vue";
import { computed, nextTick, onMounted, ref } from "vue";
import { useRoute } from "vue-router";
import {
  api,
  errorMessage,
  type ConnectSession,
  type ConnectorConnection,
  type ConnectorProviderDescription,
  type ConnectorSetupProfile,
} from "@/api/client";
import { useWorkspaceStore } from "@/stores/workspace";

const workspace = useWorkspaceStore();
const route = useRoute();
defineProps<{ embedded?: boolean }>();
const code = ref("");
const starting = ref(false);
const completing = ref(false);
const revoking = ref(false);
const confirmRevoke = ref(false);
const popupBlocked = ref(false);
const providerConnections = ref<ConnectorConnection[]>([]);
const providerLoading = ref(false);
const providerError = ref("");
const curatedDetails = ref<Record<string, ConnectorProviderDescription>>({});
const curatedUnavailable = ref<string[]>([]);
const curatedSessions = ref<Record<string, ConnectSession>>({});
const curatedStarting = ref<Record<string, boolean>>({});
const curatedError = ref("");
const curatedProviders = [
  {
    providerRef: "gitlab",
    name: "GitLab",
    mark: "G",
    description: "Repository discovery, source context, reviews, and governed publication.",
  },
  {
    providerRef: "slack",
    name: "Slack",
    mark: "S",
    description: "Live personal conversations and approval-gated collaboration actions.",
  },
  {
    providerRef: "grafana",
    name: "Grafana",
    mark: "G",
    description: "Deployment-managed dashboards, Prometheus metrics, and observability context.",
  },
] as const;
const highlightedConnection = computed(() =>
  typeof route.query.connection === "string" ? route.query.connection : undefined,
);

const connectionLabel = computed(() => {
  if (workspace.connectionState === "loading") return "Checking";
  if (workspace.connectionState === "error") return "Unavailable";
  return workspace.connected ? "Connected" : "Not connected";
});

async function startAuthorization() {
  starting.value = true;
  popupBlocked.value = false;
  try {
    const flow = await workspace.startOAuth();
    const popup = window.open(flow.authorization_url, "_blank", "noopener,noreferrer");
    popupBlocked.value = !popup;
  } catch {
    // The store exposes a browser-safe error.
  } finally {
    starting.value = false;
  }
}

async function completeAuthorization() {
  const submittedCode = code.value;
  code.value = "";
  if (!submittedCode.trim()) return;
  completing.value = true;
  try {
    await workspace.completeOAuth(submittedCode);
  } catch {
    // The store exposes a browser-safe error.
  } finally {
    completing.value = false;
  }
}

async function revoke() {
  revoking.value = true;
  try {
    await workspace.disconnect();
    confirmRevoke.value = false;
  } catch {
    // The store exposes a browser-safe error.
  } finally {
    revoking.value = false;
  }
}

async function loadProviderConnections() {
  providerLoading.value = true;
  providerError.value = "";
  curatedUnavailable.value = [];
  try {
    const [connections, ...providers] = await Promise.allSettled([
      api.connections(),
      ...curatedProviders.map((provider) => api.connectorCatalogProvider(provider.providerRef)),
    ]);
    if (connections.status === "rejected") throw connections.reason;
    providerConnections.value = connections.value;
    curatedDetails.value = Object.fromEntries(
      providers.flatMap((result, index) => {
        const provider = curatedProviders[index];
        return result.status === "fulfilled" && provider
          ? [[provider.providerRef, result.value] as const]
          : [];
      }),
    );
    curatedUnavailable.value = providers.flatMap((result, index) => {
      const provider = curatedProviders[index];
      return result.status === "rejected" && provider ? [provider.providerRef] : [];
    });
    reconcileTerminalCuratedSessions();
    await nextTick();
    if (highlightedConnection.value) {
      document.getElementById(`connection-${highlightedConnection.value}`)?.scrollIntoView({
        block: "center",
      });
    }
  } catch (cause) {
    providerError.value = errorMessage(cause);
  } finally {
    providerLoading.value = false;
  }
}

function providerMatches(connection: ConnectorConnection, providerRef: string) {
  return (
    connection.integration_ref === providerRef ||
    connection.integration_ref.endsWith(`:${providerRef}`) ||
    connection.integration_ref.endsWith(`/${providerRef}`)
  );
}

function curatedConnection(providerRef: string) {
  const connections = providerConnections.value.filter((connection) =>
    providerMatches(connection, providerRef),
  );
  const profiles = curatedDetails.value[providerRef]?.provider.setup_profiles ?? [];
  const personProfile = profiles.find((profile) => profile.actor === "person");
  const preferredProfile = personProfile ?? profiles[0];
  const eligibleConnections = personProfile
    ? connections.filter(
        (connection) =>
          connection.auth_profile === personProfile.auth_profile || connection.actor === "user",
      )
    : connections;
  const preferredConnections = preferredProfile
    ? eligibleConnections.filter(
        (connection) => connection.auth_profile === preferredProfile.auth_profile,
      )
    : eligibleConnections;
  return (
    preferredConnections.find((connection) => connection.state === "callable") ??
    preferredConnections.find((connection) => connection.state !== "revoked") ??
    eligibleConnections.find((connection) => connection.state === "callable") ??
    eligibleConnections.find((connection) => connection.state !== "revoked") ??
    preferredConnections[0] ??
    eligibleConnections[0]
  );
}

function curatedProfile(providerRef: string): ConnectorSetupProfile | undefined {
  const profiles = curatedDetails.value[providerRef]?.provider.setup_profiles ?? [];
  const authProfile = curatedConnection(providerRef)?.auth_profile;
  const personProfiles = profiles.filter((profile) => profile.actor === "person");
  if (personProfiles.length > 0) {
    return (
      personProfiles.find((profile) => profile.auth_profile === authProfile) ?? personProfiles[0]
    );
  }
  const existingProfile = profiles.find((profile) => profile.auth_profile === authProfile);
  if (existingProfile) return existingProfile;
  return profiles[0];
}

function curatedStatus(providerRef: string) {
  const session = curatedSessions.value[providerRef];
  if (session?.state === "pending") return "Authorization pending";
  const connection = curatedConnection(providerRef);
  if (providerLoading.value && !connection) return "Checking";
  if (!connection) return "Not connected";
  return connection.state === "callable" ? "Callable" : "Needs attention";
}

function curatedStatusClass(providerRef: string) {
  const pending = curatedSessions.value[providerRef]?.state === "pending";
  const connection = curatedConnection(providerRef);
  return {
    running: pending,
    succeeded: !pending && connection?.state === "callable",
    failed: !pending && !!connection && connection.state !== "callable",
  };
}

function curatedAction(providerRef: string, name: string) {
  if (curatedStarting.value[providerRef]) return "Starting…";
  if (curatedSessions.value[providerRef]?.state === "pending") return "Waiting…";
  const connection = curatedConnection(providerRef);
  if (!connection) return `Connect ${name}`;
  return connection.state === "callable" ? "Replace authorization" : `Reconnect ${name}`;
}

function curatedSessionMessage(providerRef: string) {
  const state = curatedSessions.value[providerRef]?.state;
  if (state === "failed") return "Authorization failed. Start another session to try again.";
  if (state === "expired") return "Authorization expired. Start another session to try again.";
  return "";
}

function browserSafeUrl(value: string | null | undefined) {
  if (!value) return undefined;
  try {
    const url = new URL(value);
    return url.protocol === "https:" || url.protocol === "http:" ? url.href : undefined;
  } catch {
    return undefined;
  }
}

function curatedCompletionUrl(providerRef: string) {
  const session = curatedSessions.value[providerRef];
  return session?.state === "pending" ? browserSafeUrl(session.browser_completion_url) : undefined;
}

function retainSafeCompletionUrl(session: ConnectSession, previous?: ConnectSession) {
  const suppliedUrl = browserSafeUrl(session.browser_completion_url);
  const retainedUrl =
    session.state === "pending" && previous?.connect_session_ref === session.connect_session_ref
      ? browserSafeUrl(previous.browser_completion_url)
      : undefined;
  return { ...session, browser_completion_url: suppliedUrl ?? retainedUrl };
}

function reconcileTerminalCuratedSessions() {
  const sessions = Object.entries(curatedSessions.value);
  const retained = sessions.filter(
    ([providerRef, session]) =>
      curatedUnavailable.value.includes(providerRef) ||
      (session.state !== "failed" && session.state !== "expired") ||
      curatedConnection(providerRef)?.state !== "callable",
  );
  if (retained.length !== sessions.length) curatedSessions.value = Object.fromEntries(retained);
}

async function connectCurated(providerRef: string, name: string) {
  const profile = curatedProfile(providerRef);
  if (!profile) return;
  const connection = curatedConnection(providerRef);
  curatedStarting.value = { ...curatedStarting.value, [providerRef]: true };
  curatedError.value = "";
  try {
    const started = await api.startConnection(
      providerRef,
      connection?.label ?? `My ${name}`,
      profile.auth_profile,
    );
    const session = retainSafeCompletionUrl(started);
    curatedSessions.value = { ...curatedSessions.value, [providerRef]: session };
    if (session.browser_completion_url) {
      window.open(session.browser_completion_url, "_blank", "noopener,noreferrer");
    }
    if (session.state === "pending") void pollCurated(providerRef, session.connect_session_ref, 0);
    if (session.state === "completed") await loadProviderConnections();
  } catch (cause) {
    curatedError.value = errorMessage(cause);
  } finally {
    curatedStarting.value = { ...curatedStarting.value, [providerRef]: false };
  }
}

async function pollCurated(providerRef: string, sessionRef: string, attempt: number) {
  if (attempt >= 60) {
    failPendingCuratedSession(providerRef, sessionRef);
    return;
  }
  await new Promise((resolve) => window.setTimeout(resolve, 2_000));
  try {
    const current = curatedSessions.value[providerRef];
    if (current?.connect_session_ref !== sessionRef) return;
    const session = retainSafeCompletionUrl(await api.connectionSession(sessionRef), current);
    curatedSessions.value = { ...curatedSessions.value, [providerRef]: session };
    if (session.state === "pending") void pollCurated(providerRef, sessionRef, attempt + 1);
    if (session.state === "completed") await loadProviderConnections();
  } catch (cause) {
    failPendingCuratedSession(providerRef, sessionRef);
    curatedError.value = errorMessage(cause);
  }
}

function failPendingCuratedSession(providerRef: string, sessionRef: string) {
  const session = curatedSessions.value[providerRef];
  if (session?.connect_session_ref !== sessionRef || session.state !== "pending") return;
  curatedSessions.value = {
    ...curatedSessions.value,
    [providerRef]: { ...session, state: "failed" },
  };
}

onMounted(() => void loadProviderConnections());
</script>

<template>
  <div class="connections-view" :class="{ view: !embedded }">
    <header class="view-header">
      <div>
        <p class="eyebrow">Connection custody</p>
        <h1>Model access</h1>
        <p>Authorize a user-owned subscription without placing credential bytes in Devcenter.</p>
      </div>
      <button
        class="button quiet"
        type="button"
        :disabled="workspace.connectionState === 'loading'"
        @click="workspace.loadConnection"
      >
        <RefreshCw :size="16" :class="{ spinning: workspace.connectionState === 'loading' }" />
        Refresh status
      </button>
    </header>

    <div class="connections-layout">
      <section class="connection-card featured">
        <header class="provider-header">
          <span class="provider-mark">C</span>
          <div>
            <p>User-owned model route</p>
            <h2>Claude Code</h2>
          </div>
          <span
            class="status-pill"
            :class="{
              succeeded: workspace.connected,
              failed: workspace.connectionState === 'error',
            }"
          >
            <span class="status-dot"></span>{{ connectionLabel }}
          </span>
        </header>

        <div v-if="workspace.connectionState === 'loading'" class="connection-loading">
          <span></span><span></span><span></span>
        </div>
        <div v-else-if="workspace.connectionState === 'error'" class="connection-state error-state">
          <CircleAlert :size="21" />
          <div>
            <strong>Connection status unavailable</strong>
            <p>{{ workspace.connectionError }}</p>
          </div>
          <button class="button small" type="button" @click="workspace.loadConnection">
            Try again
          </button>
        </div>
        <div v-else-if="workspace.connected" class="connection-state connected-state">
          <span class="success-seal"><Check :size="27" /></span>
          <div>
            <strong>Ready for governed attempts</strong>
            <p>
              Connectors owns refresh, replacement, and revocation. Devcenter receives presence
              metadata only.
            </p>
          </div>
        </div>
        <div v-else-if="workspace.oauthFlow" class="oauth-step">
          <div class="step-banner">
            <span>2</span>
            <div>
              <strong>Finish authorization</strong>
              <p>Approve access in Claude, then return with the one-time code.</p>
            </div>
          </div>
          <a
            class="button quiet full"
            :href="workspace.oauthFlow.authorization_url"
            target="_blank"
            rel="noopener noreferrer"
          >
            Return to Claude authorization <ArrowUpRight :size="16" />
          </a>
          <p v-if="popupBlocked" class="form-hint" role="status">
            Your browser blocked the authorization window. Use the link above to continue without
            losing this flow.
          </p>
          <form class="oauth-form" @submit.prevent="completeAuthorization">
            <div class="field">
              <label for="oauth-code">One-time code</label
              ><input
                id="oauth-code"
                v-model="code"
                type="password"
                autocomplete="off"
                spellcheck="false"
                placeholder="Paste the code from Claude"
                required
              />
            </div>
            <p v-if="workspace.connectionError" class="form-error" role="alert">
              {{ workspace.connectionError }}
            </p>
            <div class="oauth-actions">
              <button
                class="button quiet"
                type="button"
                :disabled="completing"
                @click="workspace.cancelOAuth"
              >
                Cancel
              </button>
              <button class="button primary" type="submit" :disabled="completing || !code.trim()">
                {{ completing ? "Connecting…" : "Finish connection" }}
              </button>
            </div>
          </form>
        </div>
        <div v-else class="connection-state disconnected-state">
          <span class="connection-icon"><Link2 :size="27" /></span>
          <div>
            <strong>Connect your subscription</strong>
            <p>
              Authorization happens with the provider. The resulting credential is stored and
              refreshed by Connectors.
            </p>
          </div>
        </div>

        <footer v-if="workspace.connectionState === 'ready'" class="provider-actions">
          <button
            v-if="!workspace.connected && !workspace.oauthFlow"
            class="button primary"
            type="button"
            :disabled="starting"
            @click="startAuthorization"
          >
            <KeyRound :size="17" /> {{ starting ? "Starting…" : "Connect Claude" }}
          </button>
          <template v-else-if="workspace.connected">
            <button
              v-if="!confirmRevoke"
              class="button danger-quiet"
              type="button"
              @click="confirmRevoke = true"
            >
              <Unlink :size="16" /> Disconnect
            </button>
            <div v-else class="revoke-confirm" role="alert">
              <span>Revoke this connection?</span>
              <button
                class="button quiet small"
                type="button"
                :disabled="revoking"
                @click="confirmRevoke = false"
              >
                Keep connected
              </button>
              <button
                class="button danger small"
                type="button"
                :disabled="revoking"
                @click="revoke"
              >
                {{ revoking ? "Revoking…" : "Revoke access" }}
              </button>
            </div>
          </template>
        </footer>
      </section>

      <aside class="custody-card">
        <p class="eyebrow">Custody boundary</p>
        <h2>What Devcenter can see</h2>
        <ul class="boundary-list">
          <li>
            <span class="boundary-icon allowed"><Check :size="16" /></span>
            <div>
              <strong>Connection presence</strong>
              <p>Whether your route is available for an attempt.</p>
            </div>
          </li>
          <li>
            <span class="boundary-icon allowed"><ShieldCheck :size="16" /></span>
            <div>
              <strong>Lifecycle state</strong>
              <p>Authorization and revocation status only.</p>
            </div>
          </li>
          <li>
            <span class="boundary-icon denied">×</span>
            <div>
              <strong>Credential bytes</strong>
              <p>Never returned to this application or its storage.</p>
            </div>
          </li>
          <li>
            <span class="boundary-icon denied">×</span>
            <div>
              <strong>Refresh tokens</strong>
              <p>Owned exclusively by the Connector custody boundary.</p>
            </div>
          </li>
        </ul>
        <a class="text-link" href="/docs#connect-claude-code"
          >Read the authority model <ArrowUpRight :size="15"
        /></a>
      </aside>
    </div>

    <section class="provider-connection-section curated-connections">
      <header>
        <div>
          <p class="eyebrow">First-class integrations</p>
          <h2>Engineering essentials</h2>
          <p>Guided setup for the integrations Devcenter understands especially well.</p>
        </div>
      </header>
      <p v-if="curatedError" class="form-error" role="alert">
        <CircleAlert :size="16" /> {{ curatedError }}
      </p>
      <div class="provider-connection-grid curated-grid">
        <article
          v-for="provider in curatedProviders"
          :key="provider.providerRef"
          class="provider-connection-card"
        >
          <span class="provider-mark">{{ provider.mark }}</span>
          <div>
            <strong>{{ provider.name }}</strong>
            <p>{{ provider.description }}</p>
            <p v-if="curatedSessionMessage(provider.providerRef)" role="status">
              {{ curatedSessionMessage(provider.providerRef) }}
            </p>
            <a
              v-if="curatedCompletionUrl(provider.providerRef)"
              class="text-link"
              :href="curatedCompletionUrl(provider.providerRef)"
              target="_blank"
              rel="noopener noreferrer"
            >
              Continue authorization <ArrowUpRight :size="15" />
            </a>
            <p v-else-if="curatedUnavailable.includes(provider.providerRef)" role="status">
              Setup status is unavailable. Refresh before starting or recovering this connection.
            </p>
            <p v-else-if="!providerLoading && !curatedProfile(provider.providerRef)">
              Deployment administrator setup required before this provider can be connected or
              recovered here.
            </p>
          </div>
          <div class="oauth-actions">
            <span class="status-pill" :class="curatedStatusClass(provider.providerRef)">
              {{ curatedStatus(provider.providerRef) }}
            </span>
            <button
              v-if="curatedProfile(provider.providerRef)"
              class="button small"
              type="button"
              :disabled="
                curatedStarting[provider.providerRef] ||
                curatedSessions[provider.providerRef]?.state === 'pending'
              "
              @click="connectCurated(provider.providerRef, provider.name)"
            >
              <KeyRound :size="15" /> {{ curatedAction(provider.providerRef, provider.name) }}
            </button>
          </div>
        </article>
      </div>
    </section>

    <section class="provider-connection-section">
      <header>
        <div>
          <p class="eyebrow">Connected things</p>
          <h2>Repositories and provider APIs</h2>
          <p>These are the credential-free connections visible to your current Identity session.</p>
        </div>
        <button
          class="button quiet small"
          type="button"
          :disabled="providerLoading"
          @click="loadProviderConnections"
        >
          <RefreshCw :size="15" :class="{ spinning: providerLoading }" /> Refresh
        </button>
      </header>
      <p v-if="providerError" class="form-error" role="alert">
        <CircleAlert :size="16" /> {{ providerError }}
      </p>
      <div class="provider-connection-grid">
        <p v-if="!providerLoading && providerConnections.length === 0" class="provider-empty">
          No Connector connections are visible yet. Start one from the catalog tab.
        </p>
        <article
          v-for="connection in providerConnections"
          :id="`connection-${connection.connection_ref}`"
          :key="connection.connection_ref"
          class="provider-connection-card"
          :class="{ 'search-highlight': connection.connection_ref === highlightedConnection }"
        >
          <span class="provider-mark">{{ connection.integration_ref[0]?.toUpperCase() }}</span>
          <div>
            <strong>{{ connection.label }}</strong>
            <p>{{ connection.integration_ref }} · {{ connection.actor ?? "bounded" }} authority</p>
            <code>{{ connection.connection_ref }}</code>
          </div>
          <span
            class="status-pill"
            :class="{
              succeeded: connection.state === 'callable',
              failed: connection.state === 'degraded',
            }"
          >
            {{ connection.state }}
          </span>
        </article>
      </div>
    </section>
  </div>
</template>
