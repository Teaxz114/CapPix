<template>
  <div class="color-picker-overlay" @mousemove="onMouseMove" @click="onPick" @keydown.esc="close">
    <div class="color-preview" :style="{ left: cursorX + 16 + 'px', top: cursorY - 40 + 'px' }">
      <div class="color-swatch" :style="{ background: color?.hex || '#000' }"></div>
      <div class="color-info">
        <div class="color-hex">{{ color?.hex || '' }}</div>
        <div class="color-rgb">{{ color?.rgb || '' }}</div>
      </div>
    </div>
    <div class="color-magnifier" v-if="color" :style="{ left: cursorX - 80 + 'px', top: cursorY + 20 + 'px' }">
      <canvas ref="magCanvas" width="160" height="160"></canvas>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

interface ColorInfo {
  hex: string; rgb: string; hsl: string; r: number; g: number; b: number;
}

const color = ref<ColorInfo | null>(null);
const cursorX = ref(0);
const cursorY = ref(0);
const magCanvas = ref<HTMLCanvasElement | null>(null);

const emit = defineEmits<{
  (e: "pick", color: ColorInfo): void;
  (e: "close"): void;
}>();

function onMouseMove(e: MouseEvent) {
  cursorX.value = e.clientX;
  cursorY.value = e.clientY;
  // Pick color at screen coordinates
  invoke<ColorInfo>("pick_color_at_point", { x: e.screenX, y: e.screenY })
    .then(c => { color.value = c; })
    .catch(() => {});
}

function onPick() {
  if (color.value) {
    navigator.clipboard.writeText(color.value.hex);
    emit("pick", color.value);
  }
  close();
}

function close() {
  emit("close");
  getCurrentWindow().close();
}

onMounted(() => {
  document.addEventListener("keydown", (e) => {
    if (e.key === "Escape") close();
  });
});
</script>

<style scoped>
.color-picker-overlay {
  position: fixed;
  inset: 0;
  cursor: crosshair;
  z-index: 100000;
}
.color-preview {
  position: fixed;
  display: flex;
  align-items: center;
  gap: 8px;
  background: rgba(17, 24, 39, 0.95);
  border: 1px solid #374151;
  border-radius: 6px;
  padding: 6px 10px;
  pointer-events: none;
  z-index: 100001;
  white-space: nowrap;
}
.color-swatch {
  width: 24px; height: 24px;
  border-radius: 4px;
  border: 1px solid #4b5563;
}
.color-info { font-size: 12px; color: #e5e7eb; }
.color-hex { font-weight: 600; font-family: monospace; }
.color-rgb { color: #9ca3af; font-size: 10px; }
.color-magnifier {
  position: fixed;
  background: #111827;
  border: 1px solid #374151;
  border-radius: 4px;
  pointer-events: none;
  overflow: hidden;
  z-index: 100001;
}
</style>
