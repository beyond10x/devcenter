<script setup lang="ts">
import { Keyboard, X } from "@lucide/vue";
import { nextTick, onMounted, ref } from "vue";
import { shortcutGroups } from "@/app/navigation";

const emit = defineEmits<{ close: [] }>();
const dialog = ref<HTMLElement>();
const closeButton = ref<HTMLButtonElement>();

onMounted(() => void nextTick(() => closeButton.value?.focus()));

function trapFocus(event: KeyboardEvent) {
  if (event.key !== "Tab" || !dialog.value) return;
  const focusable = [
    ...dialog.value.querySelectorAll<HTMLElement>("button, [href], input, select"),
  ];
  const first = focusable[0];
  const last = focusable.at(-1);
  if (!first || !last) return;
  if (event.shiftKey && document.activeElement === first) {
    event.preventDefault();
    last.focus();
  } else if (!event.shiftKey && document.activeElement === last) {
    event.preventDefault();
    first.focus();
  }
}
</script>

<template>
  <Teleport to="body">
    <div class="overlay-layer" role="presentation" @click.self="emit('close')">
      <section
        ref="dialog"
        class="shortcut-dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="shortcut-title"
        @keydown="trapFocus"
        @keydown.esc.stop="emit('close')"
      >
        <header class="overlay-header">
          <div class="overlay-title">
            <span class="overlay-icon"><Keyboard :size="20" /></span>
            <div>
              <p class="eyebrow">Move quickly</p>
              <h2 id="shortcut-title">Keyboard shortcuts</h2>
            </div>
          </div>
          <button
            ref="closeButton"
            class="icon-button"
            type="button"
            aria-label="Close keyboard shortcuts"
            @click="emit('close')"
          >
            <X :size="19" />
          </button>
        </header>
        <div class="shortcut-groups">
          <section v-for="group in shortcutGroups" :key="group.label">
            <h3>{{ group.label }}</h3>
            <dl>
              <div v-for="shortcut in group.shortcuts" :key="shortcut.label">
                <dt>{{ shortcut.label }}</dt>
                <dd>
                  <span v-if="shortcut.macKeys" class="shortcut-platform">
                    <kbd v-for="key in shortcut.macKeys" :key="key">{{ key }}</kbd>
                    <span>or</span>
                  </span>
                  <kbd v-for="key in shortcut.keys" :key="key">{{ key }}</kbd>
                </dd>
              </div>
            </dl>
          </section>
        </div>
      </section>
    </div>
  </Teleport>
</template>
