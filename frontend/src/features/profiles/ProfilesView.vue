<script setup lang="ts">
import {
  Check,
  CheckCircle2,
  CircleAlert,
  CircleOff,
  Plus,
  RefreshCw,
  ShieldAlert,
  ShieldCheck,
} from "@lucide/vue";
import { computed, onMounted, ref } from "vue";
import {
  api,
  errorMessage,
  type Capability,
  type CapabilityMapping,
  type CapabilityPosture,
  type CapabilityProfile,
} from "@/api/client";
import { useWorkspaceStore } from "@/stores/workspace";

const workspace = useWorkspaceStore();
const capabilities = ref<Capability[]>([]);
const profiles = ref<CapabilityProfile[]>([]);
const selectedId = ref<string>();
const loading = ref(true);
const mutating = ref(false);
const error = ref("");
const notice = ref("");
const postureOptions: { value: CapabilityPosture; label: string }[] = [
  { value: "allow", label: "Allow" },
  { value: "approval_required", label: "Approval" },
  { value: "deny", label: "Deny" },
];

const selected = computed(() => profiles.value.find((profile) => profile.id === selectedId.value));
const postureByOperation = computed(
  () =>
    new Map(
      selected.value?.mappings.map((mapping) => [mapping.operation_ref, mapping.posture]) ?? [],
    ),
);
const postureCounts = computed(() => {
  const counts: Record<CapabilityPosture, number> = {
    allow: 0,
    approval_required: 0,
    deny: 0,
  };
  for (const capability of capabilities.value) counts[postureFor(capability)] += 1;
  return counts;
});

function postureFor(capability: Capability): CapabilityPosture {
  return postureByOperation.value.get(capability.operation_ref) ?? "deny";
}

function toolName(operationRef: string): string {
  return operationRef
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "_")
    .replace(/^_+|_+$/g, "");
}

function mapping(capability: Capability, posture: CapabilityPosture): CapabilityMapping {
  const existing = selected.value?.mappings.find(
    (item) => item.operation_ref === capability.operation_ref,
  );
  return {
    ...existing,
    operation_ref: capability.operation_ref,
    tool_name: existing?.tool_name ?? toolName(capability.operation_ref),
    connection_ref: existing?.connection_ref ?? capability.connections[0]?.connection_ref,
    posture,
  };
}

async function load() {
  loading.value = true;
  error.value = "";
  try {
    [capabilities.value, profiles.value] = await Promise.all([
      api.capabilities(),
      api.capabilityProfiles(),
    ]);
    if (!profiles.value.some((profile) => profile.id === selectedId.value)) {
      selectedId.value = profiles.value[0]?.id;
    }
    workspace.capabilityProfiles = profiles.value;
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    loading.value = false;
  }
}

async function createProfile() {
  mutating.value = true;
  error.value = "";
  try {
    const mappings = capabilities.value.map((capability) =>
      mapping(capability, capability.approval === "required" ? "approval_required" : "allow"),
    );
    const profile = await api.createCapabilityProfile("Engineering default", mappings);
    profiles.value.unshift(profile);
    selectedId.value = profile.id;
    workspace.capabilityProfiles = profiles.value;
    notice.value = "Capability profile created.";
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    mutating.value = false;
  }
}

async function setPosture(capability: Capability, posture: CapabilityPosture) {
  await updatePostures(
    (item) => (item.operation_ref === capability.operation_ref ? posture : postureFor(item)),
    `${capability.title} is now ${posture === "approval_required" ? "approval required" : posture}.`,
  );
}

async function setAllPostures(posture: Extract<CapabilityPosture, "allow" | "deny">) {
  await updatePostures(
    () => posture,
    `All ${String(capabilities.value.length)} capabilities are now ${posture === "allow" ? "allowed" : "denied"}.`,
  );
}

async function updatePostures(
  resolvePosture: (capability: Capability) => CapabilityPosture,
  successNotice: string,
) {
  const profile = selected.value;
  if (!profile) return;
  mutating.value = true;
  error.value = "";
  notice.value = "";
  try {
    const mappings = capabilities.value.map((item) => mapping(item, resolvePosture(item)));
    const changed = await api.updateCapabilityProfile(profile, mappings);
    profiles.value = profiles.value.map((item) => (item.id === changed.id ? changed : item));
    workspace.capabilityProfiles = profiles.value;
    notice.value = successNotice;
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    mutating.value = false;
  }
}

onMounted(() => void load());
</script>

<template>
  <div class="view profiles-view">
    <header class="view-header">
      <div>
        <p class="eyebrow">Capability profiles</p>
        <h1>Set the agent authority posture</h1>
        <p>Every setting stays beneath your current Connector grants and connected accounts.</p>
      </div>
      <button class="button quiet" type="button" :disabled="loading" @click="load">
        <RefreshCw :size="16" :class="{ spinning: loading }" /> Refresh
      </button>
    </header>

    <p v-if="error" class="form-error" role="alert"><CircleAlert :size="16" /> {{ error }}</p>
    <p v-if="notice" class="publication-notice" role="status"><Check :size="16" /> {{ notice }}</p>

    <section v-if="!profiles.length && !loading" class="publication-empty">
      <ShieldCheck :size="30" />
      <h2>No capability profile yet</h2>
      <p>Create a revisioned profile from the capabilities callable through your connections.</p>
      <button
        class="button primary"
        type="button"
        :disabled="mutating || !capabilities.length"
        @click="createProfile"
      >
        <Plus :size="16" /> Create engineering default
      </button>
    </section>

    <div v-else-if="profiles.length" class="profile-layout">
      <aside class="publication-list" aria-label="Capability profiles">
        <button
          v-for="profile in profiles"
          :key="profile.id"
          type="button"
          :class="{ selected: profile.id === selectedId }"
          @click="selectedId = profile.id"
        >
          <span class="publication-icon"><ShieldCheck :size="17" /></span>
          <span
            ><strong>{{ profile.name }}</strong
            ><small>Revision {{ profile.revision }}</small></span
          >
        </button>
      </aside>

      <main v-if="selected" class="profile-capabilities">
        <header>
          <div>
            <p class="eyebrow">{{ selected.id }}</p>
            <h2>{{ selected.name }}</h2>
          </div>
          <span class="status-pill active">revision {{ selected.revision }}</span>
        </header>
        <div class="permission-toolbar">
          <div class="permission-toolbar-row">
            <div class="permission-summary" aria-label="Effective permission totals">
              <span class="permission-count allow">
                <CheckCircle2 :size="15" /> {{ postureCounts.allow }} allowed
              </span>
              <span class="permission-count approval-required">
                <ShieldAlert :size="15" /> {{ postureCounts.approval_required }} approval
              </span>
              <span class="permission-count deny">
                <CircleOff :size="15" /> {{ postureCounts.deny }} denied
              </span>
            </div>
            <div class="permission-bulk-actions" aria-label="Bulk permission actions">
              <button
                class="button quiet small bulk-allow"
                type="button"
                :disabled="mutating || !capabilities.length"
                @click="setAllPostures('allow')"
              >
                <CheckCircle2 :size="15" /> Allow all
              </button>
              <button
                class="button danger-quiet small"
                type="button"
                :disabled="mutating || !capabilities.length"
                @click="setAllPostures('deny')"
              >
                <CircleOff :size="15" /> Deny all
              </button>
            </div>
          </div>
          <p>
            Allow exposes a capability while preserving any approval the Connector itself requires.
          </p>
        </div>
        <div
          v-for="capability in capabilities"
          :key="capability.operation_ref"
          class="capability-row"
          :class="`posture-${postureFor(capability)}`"
        >
          <div>
            <strong>{{ capability.title }}</strong>
            <span>{{ capability.operation_ref }} · {{ capability.effect.replace("_", " ") }}</span>
          </div>
          <div class="posture-control" :aria-label="`${capability.title} posture`">
            <button
              v-for="option in postureOptions"
              :key="option.value"
              type="button"
              class="posture-button"
              :class="[option.value, { active: postureFor(capability) === option.value }]"
              :aria-pressed="postureFor(capability) === option.value"
              :disabled="mutating"
              @click="setPosture(capability, option.value)"
            >
              <Check v-if="postureFor(capability) === option.value" :size="13" />
              {{ option.label }}
            </button>
          </div>
        </div>
        <p v-if="!capabilities.length" class="section-empty">
          No callable capabilities are visible through the current connections.
        </p>
      </main>
    </div>
  </div>
</template>
