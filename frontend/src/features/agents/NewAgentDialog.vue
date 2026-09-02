<script setup lang="ts">
import { Bot, X } from "@lucide/vue";
import { nextTick, onBeforeUnmount, onMounted, ref } from "vue";
import { z } from "zod";
import { errorMessage } from "@/api/client";
import { useWorkspaceStore } from "@/stores/workspace";

const emit = defineEmits<{ close: [] }>();
const workspace = useWorkspaceStore();
const name = ref("");
const instructions = ref("");
const model = ref("claude-opus-5");
const capabilityProfileId = ref("");
const submitting = ref(false);
const formError = ref("");
const nameField = ref<HTMLInputElement>();

const schema = z.object({
  name: z.string().trim().min(1, "Give the agent a name.").max(160),
  instructions: z.string().trim().min(1, "Describe how this agent should work."),
  model: z.string().trim().min(1, "Choose a model route."),
  capability_profile_id: z.string().trim().optional(),
});

async function submit() {
  formError.value = "";
  const parsed = schema.safeParse({
    name: name.value,
    instructions: instructions.value,
    model: model.value,
    capability_profile_id: capabilityProfileId.value || undefined,
  });
  if (!parsed.success) {
    formError.value = parsed.error.issues[0]?.message ?? "Check the agent details.";
    return;
  }
  submitting.value = true;
  try {
    await workspace.createAgent(parsed.data);
    emit("close");
  } catch (error) {
    formError.value = errorMessage(error);
  } finally {
    submitting.value = false;
  }
}

function onKeydown(event: KeyboardEvent) {
  if (event.key === "Escape" && !submitting.value) emit("close");
}

onMounted(() => {
  document.addEventListener("keydown", onKeydown);
  void nextTick(() => nameField.value?.focus());
});
onBeforeUnmount(() => {
  document.removeEventListener("keydown", onKeydown);
});
</script>

<template>
  <div class="dialog-layer" role="presentation" @mousedown.self="emit('close')">
    <section class="dialog" role="dialog" aria-modal="true" aria-labelledby="new-agent-title">
      <header class="dialog-header">
        <span class="dialog-icon"><Bot :size="22" /></span>
        <div>
          <p class="eyebrow">New agent</p>
          <h2 id="new-agent-title">Define a governed worker</h2>
        </div>
        <button
          class="icon-button"
          type="button"
          aria-label="Close"
          :disabled="submitting"
          @click="emit('close')"
        >
          <X :size="19" />
        </button>
      </header>
      <form class="form-stack" @submit.prevent="submit">
        <div class="field">
          <label for="agent-name">Name</label>
          <input
            id="agent-name"
            ref="nameField"
            v-model="name"
            maxlength="160"
            placeholder="Release assistant"
            autocomplete="off"
          />
          <small>Make it recognizable in the agent roster.</small>
        </div>
        <div class="field">
          <label for="agent-instructions">Instructions</label>
          <textarea
            id="agent-instructions"
            v-model="instructions"
            rows="6"
            placeholder="You prepare safe, reviewable releases and show your evidence."
          ></textarea>
        </div>
        <div class="field">
          <label for="agent-model">Model route</label>
          <input id="agent-model" v-model="model" placeholder="claude-opus-5" autocomplete="off" />
          <small>The route is recorded in the agent's immutable revision.</small>
        </div>
        <div class="field">
          <label for="agent-profile">Capability profile <span>(optional)</span></label>
          <select id="agent-profile" v-model="capabilityProfileId">
            <option value="">No external capabilities</option>
            <option
              v-for="profile in workspace.capabilityProfiles"
              :key="profile.id"
              :value="profile.id"
            >
              {{ profile.name }} · revision {{ profile.revision }}
            </option>
          </select>
          <small>The profile is pinned to this immutable agent revision.</small>
        </div>
        <p v-if="formError" class="form-error" role="alert">{{ formError }}</p>
        <footer class="dialog-actions">
          <button class="button quiet" type="button" :disabled="submitting" @click="emit('close')">
            Cancel
          </button>
          <button class="button primary" type="submit" :disabled="submitting">
            {{ submitting ? "Creating agent…" : "Create and activate" }}
          </button>
        </footer>
      </form>
    </section>
  </div>
</template>
