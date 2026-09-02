<script setup lang="ts">
import { Palette } from "@lucide/vue";
import { useTheme, type ThemePreference } from "@/theme/theme";

const theme = useTheme();

function changeTheme(event: Event) {
  theme.setPreference((event.target as HTMLSelectElement).value as ThemePreference);
}
</script>

<template>
  <label class="theme-picker">
    <Palette :size="16" aria-hidden="true" />
    <span>Theme</span>
    <select :value="theme.preference.value" aria-label="Theme" @change="changeTheme">
      <option v-for="option in theme.options" :key="option.value" :value="option.value">
        {{ option.label }}
      </option>
    </select>
  </label>
</template>
