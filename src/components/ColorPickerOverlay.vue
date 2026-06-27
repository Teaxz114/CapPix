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
    <!-- Color history -->
    <div v-if="colorHistory.length > 0" class="color-history" :style="{ left: cursorX + 16 + 'px', top: cursorY + 10 + 'px' }">
      <div
        v-for="(c, i) in colorHistory" :key="i"
        class="history-swatch"
        :style="{ background: c }"
        :title="c"
        @click.stop="copyHistoryColor(c)"
      ></div>
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
const colorHistory = ref<string[]>([]);

const MAX_HISTORY = 20;
const HISTORY_KEY = "cappix-color-history";

const emit = defineEmits<{
  (e: "pick", color: ColorInfo): void;
  (e: "close"): void;
}>();

function loadHistory() {
  try {
    const stored = localStorage.getItem(HISTORY_KEY);
    if (stored) colorHistory.value = JSON.parse(stored);
  } catch {}
}

function saveHistoryColor(hex: string) {
  // Don't add duplicates
  const idx = colorHistory.value.indexOf(hex);
  if (idx !== -1) colorHistory.value.splice(idx, 1);
  colorHistory.value.unshift(hex);
  if (colorHistory.value.length > MAX_HISTORY) colorHistory.value.pop();
  localStorage.setItem(HISTORY_KEY, JSON.stringify(colorHistory.value));
}

function onMouseMove(e: MouseEvent) {
  cursorX.value = e.clientX;
  cursorY.value = e.clientY;
  invoke<ColorInfo>("pick_color_at_point", { x: e.screenX, y: e.screenY })
    .then(c => { color.value = c; })
    .catch(() => {});

  // Draw magnifier — pick 20x20 pixel region around cursor
  drawMagnifier(e.screenX, e.screenY);
}

function drawMagnifier(screenX: number, screenY: number) {
  const canvas = magCanvas.value;
  if (!canvas) return;

  invoke<number[]>("pick_color_region", { x: screenX, y: screenY, size: 20 })
    .then(rgba => {
      const ctx = canvas.getContext("2d");
      if (!ctx) return;

      const regionSize = 20;
      const canvasSize = 160;
      const scale = canvasSize / regionSize; // 8x magnification

      ctx.imageSmoothingEnabled = false;
      ctx.clearRect(0, 0, canvasSize, canvasSize);

      // Draw each pixel as a scaled block
      for (let y = 0; y < regionSize; y++) {
        for (let x = 0; x < regionSize; x++) {
          const i = (y * regionSize + x) * 4;
          const r = rgba[i];
          const g = rgba[i + 1];
          const b = rgba[i + 2];
          ctx.fillStyle = `rgb(${r},${g},${b})`;
          ctx.fillRect(x * scale, y * scale, scale, scale);
        }
      }

      // Draw crosshair at center
      const cx = (regionSize / 2) * scale;
      const cy = (regionSize / 2) * scale;
      ctx.strokeStyle = "rgba(255,255,255,0.8)";
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.moveTo(cx, 0); ctx.lineTo(cx, canvasSize);
      ctx.moveTo(0, cy); ctx.lineTo(canvasSize, cy);
      ctx.stroke();

      // Draw border around center pixel
      ctx.strokeStyle = "rgba(255,255,0,0.8)";
      ctx.lineWidth = 2;
      ctx.strokeRect(cx - scale / 2, cy - scale / 2, scale, scale);
    })
    .catch(() => {});
}

async function onPick() {
  if (color.value) {
    await copyText(color.value.hex);
    saveHistoryColor(color.value.hex);
    emit("pick", color.value);
  }
  close();
}

async function copyHistoryColor(hex: string) {
  await copyText(hex);
}

async function copyText(text: string) {
  try {
    const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
    await writeText(text);
  } catch (e) {
    console.error("Copy text failed:", e);
  }
}

function close() {
  emit("close");
  // Don't close the window — just emit close event for parent to handle
}

function onEscKey(e: KeyboardEvent) {
  if (e.key === "Escape") close();
}

onMounted(() => {
  loadHistory();
  document.addEventListener("keydown", onEscKey);
});

onUnmounted(() => {
  document.removeEventListener("keydown", onEscKey);
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
.color-history {
  position: fixed;
  display: flex;
  gap: 3px;
  padding: 4px;
  background: rgba(17, 24, 39, 0.95);
  border: 1px solid #374151;
  border-radius: 4px;
  z-index: 100001;
}
.history-swatch {
  width: 16px; height: 16px;
  border-radius: 2px;
  border: 1px solid rgba(255,255,255,0.2);
  cursor: pointer;
  transition: transform 0.1s;
}
.history-swatch:hover { transform: scale(1.3); }
</style>
