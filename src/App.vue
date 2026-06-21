<template>
  <router-view />
</template>

<script setup lang="ts">
import { watch, onMounted } from "vue";
import { useConfigStore } from "./stores/config";

const configStore = useConfigStore();

function applyTheme(theme: "dark" | "light") {
  document.documentElement.setAttribute("data-theme", theme);
}

onMounted(() => {
  applyTheme(configStore.config.theme);
});

watch(() => configStore.config.theme, (newTheme) => {
  applyTheme(newTheme);
});
</script>

<style>
:root,
[data-theme="dark"] {
  --bg-primary: #111827;
  --bg-secondary: #1f2937;
  --bg-tertiary: #374151;
  --bg-hover: #4b5563;
  --text-primary: #f9fafb;
  --text-secondary: #e5e7eb;
  --text-muted: #9ca3af;
  --text-dim: #6b7280;
  --border-color: #374151;
  --accent: #3b82f6;
  --accent-hover: #2563eb;
  --danger: #ef4444;
  --success: #22c55e;
  --overlay-bg: rgba(0, 0, 0, 0.4);
}

[data-theme="light"] {
  --bg-primary: #ffffff;
  --bg-secondary: #f3f4f6;
  --bg-tertiary: #e5e7eb;
  --bg-hover: #d1d5db;
  --text-primary: #111827;
  --text-secondary: #374151;
  --text-muted: #6b7280;
  --text-dim: #9ca3af;
  --border-color: #d1d5db;
  --accent: #3b82f6;
  --accent-hover: #2563eb;
  --danger: #ef4444;
  --success: #22c55e;
  --overlay-bg: rgba(0, 0, 0, 0.2);
}

body {
  margin: 0;
  font-family: "Microsoft YaHei", "Segoe UI", system-ui, sans-serif;
  background: var(--bg-primary);
  color: var(--text-primary);
}
</style>
