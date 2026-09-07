<script setup lang="ts">
import { nextTick, onBeforeUnmount, onMounted, ref } from "vue";

const props = defineProps<{
  title: string;
  description: string;
  action: string;
  pending: boolean;
  error?: string;
}>();
const emit = defineEmits<{ confirm: []; close: [] }>();
const dialog = ref<HTMLElement>();
const cancel = ref<HTMLButtonElement>();
const previousFocus = document.activeElement;
function keydown(event: KeyboardEvent) {
  if (event.key === "Escape" && !props.pending) emit("close");
  if (event.key !== "Tab") return;
  const buttons = Array.from(
    dialog.value?.querySelectorAll<HTMLButtonElement>("button:not(:disabled)") ?? [],
  );
  if (!buttons.length) {
    event.preventDefault();
    return;
  }
  if (event.shiftKey && document.activeElement === buttons[0]) {
    event.preventDefault();
    buttons.at(-1)?.focus();
  } else if (!event.shiftKey && document.activeElement === buttons.at(-1)) {
    event.preventDefault();
    buttons[0]?.focus();
  }
}
onMounted(() => {
  document.addEventListener("keydown", keydown);
  void nextTick(() => cancel.value?.focus());
});
onBeforeUnmount(() => {
  document.removeEventListener("keydown", keydown);
  if (previousFocus instanceof HTMLElement) previousFocus.focus();
});
</script>

<template>
  <div class="dialog-layer" role="presentation" @mousedown.self="!pending && emit('close')">
    <section
      ref="dialog"
      class="dialog"
      role="dialog"
      aria-modal="true"
      aria-labelledby="confirm-title"
      aria-describedby="confirm-description"
    >
      <header class="dialog-header">
        <h2 id="confirm-title">{{ title }}</h2>
      </header>
      <div class="form-stack">
        <p id="confirm-description">{{ description }}</p>
        <p v-if="error" class="form-error" role="alert">{{ error }}</p>
        <footer class="dialog-actions">
          <button
            ref="cancel"
            class="button quiet"
            type="button"
            :disabled="pending"
            @click="emit('close')"
          >
            Cancel
          </button>
          <button
            class="button danger-quiet"
            type="button"
            :disabled="pending"
            @click="emit('confirm')"
          >
            {{ pending ? "Saving…" : action }}
          </button>
        </footer>
      </div>
    </section>
  </div>
</template>
