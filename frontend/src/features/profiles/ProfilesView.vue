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
  X,
} from "@lucide/vue";
import { computed, onMounted, ref, watch } from "vue";
import { useRoute, useRouter } from "vue-router";
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
const route = useRoute();
const router = useRouter();
const capabilities = ref<Capability[]>([]);
const profiles = ref<CapabilityProfile[]>([]);
const selectedId = ref<string | undefined>(
  typeof route.query.profile === "string" ? route.query.profile : undefined,
);
const loading = ref(true);
const mutating = ref(false);
const showCreate = ref(false);
const createName = ref("");
const createAudience = ref<CapabilityProfile["audience"]>("personal");
const createPreset = ref<"guarded" | "read_only" | "empty">("guarded");
const editingName = ref("");
const error = ref("");
const notice = ref("");
const postureOptions: { value: CapabilityPosture; label: string }[] = [
  { value: "allow", label: "Allow" },
  { value: "approval_required", label: "Approval" },
  { value: "deny", label: "Deny" },
];

const selected = computed(() => profiles.value.find((profile) => profile.id === selectedId.value));
const canManageTenantProfiles = computed(() => workspace.session?.groups.includes("operator"));
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
      const requested = typeof route.query.profile === "string" ? route.query.profile : undefined;
      selectedId.value =
        requested && profiles.value.some((profile) => profile.id === requested)
          ? requested
          : profiles.value[0]?.id;
    }
    workspace.capabilityProfiles = profiles.value;
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    loading.value = false;
  }
}

function chooseProfile(profileId: string) {
  selectedId.value = profileId;
  void router.replace({ path: "/profiles", query: { profile: profileId } });
}

async function createProfile() {
  const name = createName.value.trim();
  if (!name) {
    error.value = "Give the capability profile a name.";
    return;
  }
  mutating.value = true;
  error.value = "";
  try {
    const mappings = capabilities.value.map((capability) => {
      let posture: CapabilityPosture = "deny";
      if (createPreset.value !== "empty" && capability.effect === "read_only") posture = "allow";
      if (createPreset.value === "guarded" && capability.effect !== "read_only") {
        posture = "approval_required";
      }
      return mapping(capability, posture);
    });
    const audience = canManageTenantProfiles.value ? createAudience.value : "personal";
    const profile = await api.createCapabilityProfile(name, audience, mappings);
    profiles.value.unshift(profile);
    selectedId.value = profile.id;
    workspace.capabilityProfiles = profiles.value;
    notice.value = "Capability profile created.";
    showCreate.value = false;
    createName.value = "";
    createAudience.value = "personal";
  } catch (cause) {
    error.value = errorMessage(cause);
  } finally {
    mutating.value = false;
  }
}

async function renameProfile() {
  const profile = selected.value;
  const name = editingName.value.trim();
  if (!profile || !name || name === profile.name) return;
  mutating.value = true;
  error.value = "";
  try {
    const changed = await api.updateCapabilityProfile(profile, profile.mappings, name);
    profiles.value = profiles.value.map((item) => (item.id === changed.id ? changed : item));
    workspace.capabilityProfiles = profiles.value;
    notice.value = "Capability profile renamed.";
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
watch(
  () => route.query.profile,
  (profileId) => {
    if (
      typeof profileId === "string" &&
      profiles.value.some((profile) => profile.id === profileId)
    ) {
      selectedId.value = profileId;
    }
  },
);
watch(
  selected,
  (profile) => {
    editingName.value = profile?.name ?? "";
  },
  { immediate: true },
);
</script>

<template>
  <div class="view profiles-view">
    <header class="view-header">
      <div>
        <p class="eyebrow">Capability profiles</p>
        <h1>Set the agent authority posture</h1>
        <p>Every setting stays beneath your current Connector grants and connected accounts.</p>
      </div>
      <div class="view-header-actions">
        <button class="button quiet" type="button" :disabled="loading" @click="load">
          <RefreshCw :size="16" :class="{ spinning: loading }" /> Refresh
        </button>
        <button class="button primary" type="button" @click="showCreate = true">
          <Plus :size="16" /> New profile
        </button>
      </div>
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
        @click="showCreate = true"
      >
        <Plus :size="16" /> Create a personal profile
      </button>
    </section>

    <div v-else-if="profiles.length" class="profile-layout">
      <aside class="publication-list" aria-label="Capability profiles">
        <button
          v-for="profile in profiles"
          :key="profile.id"
          type="button"
          :class="{ selected: profile.id === selectedId }"
          @click="chooseProfile(profile.id)"
        >
          <span class="publication-icon"><ShieldCheck :size="17" /></span>
          <span
            ><strong>{{ profile.name }}</strong
            ><small
              >{{ profile.audience === "personal" ? "Personal" : "Shared template" }} · Revision
              {{ profile.revision }}</small
            ></span
          >
        </button>
      </aside>

      <main v-if="selected" class="profile-capabilities">
        <header>
          <div>
            <p class="eyebrow">{{ selected.id }}</p>
            <h2>{{ selected.name }}</h2>
            <div class="profile-name-editor">
              <label class="sr-only" for="profile-name-edit">Profile name</label>
              <input
                id="profile-name-edit"
                v-model="editingName"
                maxlength="160"
                :disabled="mutating"
              />
              <button
                class="button quiet small"
                type="button"
                :disabled="mutating || !editingName.trim() || editingName.trim() === selected.name"
                @click="renameProfile"
              >
                Save name
              </button>
            </div>
            <small class="profile-audience-label">
              {{
                selected.audience === "personal"
                  ? "Only you can bind this profile"
                  : "Shared tenant template"
              }}
            </small>
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

    <div
      v-if="showCreate"
      class="dialog-layer"
      role="presentation"
      @mousedown.self="showCreate = false"
    >
      <section class="dialog" role="dialog" aria-modal="true" aria-labelledby="new-profile-title">
        <header class="dialog-header">
          <span class="dialog-icon"><ShieldCheck :size="22" /></span>
          <div>
            <p class="eyebrow">New capability profile</p>
            <h2 id="new-profile-title">Choose a safe starting posture</h2>
          </div>
          <button
            class="icon-button"
            type="button"
            aria-label="Close"
            :disabled="mutating"
            @click="showCreate = false"
          >
            <X :size="19" />
          </button>
        </header>
        <form class="form-stack" @submit.prevent="createProfile">
          <div class="field">
            <label for="profile-name">Name</label>
            <input
              id="profile-name"
              v-model="createName"
              maxlength="160"
              placeholder="My engineering tools"
              autocomplete="off"
            />
          </div>
          <div class="field">
            <label for="profile-preset">Starting posture</label>
            <select id="profile-preset" v-model="createPreset">
              <option value="guarded">Reads allowed; every write requires approval</option>
              <option value="read_only">Read-only; all effectful operations denied</option>
              <option value="empty">Deny everything</option>
            </select>
            <small>You can review every operation before assigning this profile to an agent.</small>
          </div>
          <div v-if="canManageTenantProfiles" class="field">
            <label for="profile-audience">Visibility</label>
            <select id="profile-audience" v-model="createAudience">
              <option value="personal">Personal · only I can bind it</option>
              <option value="tenant">Shared template · visible to the tenant</option>
            </select>
          </div>
          <p v-else class="profile-personal-note">
            New profiles are personal to your verified identity.
          </p>
          <footer class="dialog-actions">
            <button
              class="button quiet"
              type="button"
              :disabled="mutating"
              @click="showCreate = false"
            >
              Cancel
            </button>
            <button
              class="button primary"
              type="submit"
              :disabled="mutating || !capabilities.length"
            >
              {{ mutating ? "Creating…" : "Create profile" }}
            </button>
          </footer>
        </form>
      </section>
    </div>
  </div>
</template>
