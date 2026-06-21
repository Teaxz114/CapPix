<template>
  <div
    class="screenshot-overlay"
    ref="overlayRef"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
    @dblclick="onDoubleClick"
  >
    <!-- Screenshot background image -->
    <img
      v-if="screenshotData"
      :src="`data:image/png;base64,${screenshotData}`"
      class="screenshot-bg"
      draggable="false"
    />

    <!-- Dark overlay with cutout for selection -->
    <svg class="overlay-svg" v-if="screenshotData">
      <defs>
        <mask id="selection-mask">
          <rect x="0" y="0" width="100%" height="100%" fill="white" />
          <rect
            v-if="isSelecting || hasSelection"
            :x="selectionX"
            :y="selectionY"
            :width="selectionW"
            :height="selectionH"
            fill="black"
          />
        </mask>
      </defs>
      <rect
        x="0" y="0" width="100%" height="100%"
        fill="rgba(0,0,0,0.4)"
        mask="url(#selection-mask)"
      />
    </svg>

    <!-- Selection border -->
    <div
      v-if="isSelecting || hasSelection"
      class="selection-border"
      :style="{
        left: selectionX + 'px',
        top: selectionY + 'px',
        width: selectionW + 'px',
        height: selectionH + 'px',
      }"
    >
      <!-- Corner handles -->
      <span class="handle handle-tl"></span>
      <span class="handle handle-tr"></span>
      <span class="handle handle-bl"></span>
      <span class="handle handle-br"></span>
    </div>

    <!-- Dimension info -->
    <div
      v-if="isSelecting || hasSelection"
      class="selection-info"
      :style="infoPosition"
    >
      {{ selectionW }} × {{ selectionH }}
    </div>

    <!-- Magnifier -->
    <div
      v-if="showMagnifier && screenshotData && !isSelecting"
      class="magnifier"
      :style="{
        left: magnifierX + 'px',
        top: magnifierY + 'px',
      }"
    >
      <canvas ref="magnifierCanvas" width="120" height="120"></canvas>
      <div class="magnifier-crosshair"></div>
      <div class="magnifier-color" v-if="pixelColor">
        <span class="color-swatch" :style="{ background: pixelColor }"></span>
        {{ pixelColor }}
      </div>
    </div>

    <!-- Hint text -->
    <div v-if="!isSelecting && !hasSelection" class="hint-text">
      拖拽选择区域 · 双击全屏截图 · ESC 取消
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, nextTick, watch } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useRouter } from "vue-router";

const router = useRouter();
const overlayRef = ref<HTMLDivElement | null>(null);
const magnifierCanvas = ref<HTMLCanvasElement | null>(null);

// Screenshot data
const screenshotData = ref("");
const screenshotImage = ref<HTMLImageElement | null>(null);

// Selection state
const isSelecting = ref(false);
const hasSelection = ref(false);
const startX = ref(0);
const startY = ref(0);
const endX = ref(0);
const endY = ref(0);

// Magnifier state
const showMagnifier = ref(true);
const cursorX = ref(0);
const cursorY = ref(0);
const pixelColor = ref("");

// Computed selection bounds
const selectionX = computed(() => Math.min(startX.value, endX.value));
const selectionY = computed(() => Math.min(startY.value, endY.value));
const selectionW = computed(() => Math.abs(endX.value - startX.value));
const selectionH = computed(() => Math.abs(endY.value - startY.value));

// Info position (near selection, offset to avoid overlap)
const infoPosition = computed(() => {
  const x = selectionX.value;
  const y = selectionY.value;
  const w = selectionW.value;
  const h = selectionH.value;
  // Place info below the selection, centered
  let infoX = x + w / 2 - 40;
  let infoY = y + h + 8;
  // If too close to bottom, place above
  if (infoY + 30 > window.innerHeight) {
    infoY = y - 28;
  }
  // Clamp horizontal
  infoX = Math.max(4, Math.min(infoX, window.innerWidth - 100));
  return {
    left: infoX + "px",
    top: infoY + "px",
  };
});

// Magnifier position
const magnifierX = computed(() => cursorX.value + 20);
const magnifierY = computed(() => cursorY.value - 140);

// Load screenshot image for magnifier pixel reading
watch(screenshotData, (data) => {
  if (data) {
    const img = new Image();
    img.onload = () => {
      screenshotImage.value = img;
    };
    img.src = `data:image/png;base64,${data}`;
  }
});

// Tauri event listener
let unlisten: (() => void) | null = null;

onMounted(async () => {
  try {
    unlisten = await listen<string>("screenshot-ready", (event) => {
      screenshotData.value = event.payload;
    });
  } catch (e) {
    console.error("Failed to listen for screenshot-ready:", e);
  }

  // ESC key handler
  document.addEventListener("keydown", onKeyDown);
});

onUnmounted(() => {
  if (unlisten) unlisten();
  document.removeEventListener("keydown", onKeyDown);
});

function onKeyDown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    getCurrentWindow().close();
  }
}

function onMouseDown(e: MouseEvent) {
  if (e.button !== 0) return;
  isSelecting.value = true;
  hasSelection.value = false;
  startX.value = e.clientX;
  startY.value = e.clientY;
  endX.value = e.clientX;
  endY.value = e.clientY;
  showMagnifier.value = false;
}

function onMouseMove(e: MouseEvent) {
  cursorX.value = e.clientX;
  cursorY.value = e.clientY;

  if (isSelecting.value) {
    endX.value = e.clientX;
    endY.value = e.clientY;
  } else {
    // Update magnifier
    showMagnifier.value = true;
    updateMagnifier(e.clientX, e.clientY);
  }
}

function onMouseUp(e: MouseEvent) {
  if (!isSelecting.value) return;
  isSelecting.value = false;
  endX.value = e.clientX;
  endY.value = e.clientY;

  const w = selectionW.value;
  const h = selectionH.value;

  if (w < 5 || h < 5) {
    hasSelection.value = false;
    return;
  }

  hasSelection.value = true;

  // Capture the selected region
  captureRegion(selectionX.value, selectionY.value, w, h);
}

async function onDoubleClick() {
  // Full screen capture
  try {
    const result = await invoke<{ image_base64: string; width: number; height: number }>("capture_fullscreen");
    navigateToAnnotate(result.image_base64);
  } catch (err) {
    console.error("Failed to capture fullscreen:", err);
  }
}

async function captureRegion(x: number, y: number, w: number, h: number) {
  try {
    const result = await invoke<{ image_base64: string; width: number; height: number }>("capture_region", {
      x,
      y,
      width: w,
      height: h,
    });
    navigateToAnnotate(result.image_base64);
  } catch (err) {
    console.error("Failed to capture region:", err);
  }
}

function navigateToAnnotate(imageBase64: string) {
  // Store the image data and navigate to annotate view
  // Use sessionStorage to pass data between views
  sessionStorage.setItem("cappix-annotate-image", imageBase64);
  router.push("/annotate");
}

function updateMagnifier(x: number, y: number) {
  if (!magnifierCanvas.value || !screenshotImage.value) return;

  const canvas = magnifierCanvas.value;
  const ctx = canvas.getContext("2d");
  if (!ctx) return;

  const img = screenshotImage.value;
  const scaleX = img.naturalWidth / window.innerWidth;
  const scaleY = img.naturalHeight / window.innerHeight;

  const srcX = x * scaleX;
  const srcY = y * scaleY;
  const zoom = 8;
  const srcSize = canvas.width / zoom;

  ctx.imageSmoothingEnabled = false;
  ctx.clearRect(0, 0, canvas.width, canvas.height);

  // Draw zoomed portion
  ctx.drawImage(
    img,
    srcX - srcSize / 2,
    srcY - srcSize / 2,
    srcSize,
    srcSize,
    0,
    0,
    canvas.width,
    canvas.height
  );

  // Draw grid lines
  ctx.strokeStyle = "rgba(255,255,255,0.3)";
  ctx.lineWidth = 0.5;
  const cellSize = canvas.width / srcSize;
  for (let i = 0; i <= srcSize; i++) {
    ctx.beginPath();
    ctx.moveTo(i * cellSize, 0);
    ctx.lineTo(i * cellSize, canvas.height);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(0, i * cellSize);
    ctx.lineTo(canvas.width, i * cellSize);
    ctx.stroke();
  }

  // Get pixel color
  try {
    const pixel = ctx.getImageData(canvas.width / 2, canvas.height / 2, 1, 1).data;
    pixelColor.value = `#${pixel[0].toString(16).padStart(2, "0")}${pixel[1].toString(16).padStart(2, "0")}${pixel[2].toString(16).padStart(2, "0")}`;
  } catch {
    pixelColor.value = "";
  }
}
</script>

<style scoped>
.screenshot-overlay {
  position: fixed;
  top: 0;
  left: 0;
  width: 100vw;
  height: 100vh;
  cursor: crosshair;
  background: transparent;
  z-index: 9999;
}

.screenshot-bg {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
  user-select: none;
}

.overlay-svg {
  position: fixed;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  pointer-events: none;
}

.selection-border {
  position: fixed;
  border: 2px solid #3b82f6;
  pointer-events: none;
  z-index: 10;
}

.handle {
  position: absolute;
  width: 8px;
  height: 8px;
  background: #3b82f6;
  border: 1px solid white;
  border-radius: 1px;
}

.handle-tl { top: -4px; left: -4px; }
.handle-tr { top: -4px; right: -4px; }
.handle-bl { bottom: -4px; left: -4px; }
.handle-br { bottom: -4px; right: -4px; }

.selection-info {
  position: fixed;
  background: #1f2937;
  color: #e5e7eb;
  padding: 4px 10px;
  border-radius: 4px;
  font-size: 12px;
  font-family: "Cascadia Code", "Fira Code", "Consolas", monospace;
  pointer-events: none;
  z-index: 9999;
  white-space: nowrap;
  box-shadow: 0 2px 8px rgba(0,0,0,0.3);
}

.magnifier {
  position: fixed;
  pointer-events: none;
  z-index: 10000;
  display: flex;
  flex-direction: column;
  align-items: center;
}

.magnifier canvas {
  border: 2px solid #3b82f6;
  border-radius: 4px;
  background: #000;
}

.magnifier-crosshair {
  position: absolute;
  top: 58px;
  left: 58px;
  width: 6px;
  height: 6px;
  border: 1px solid #ff4444;
  pointer-events: none;
}

.magnifier-color {
  margin-top: 4px;
  background: #1f2937;
  color: #e5e7eb;
  padding: 2px 6px;
  border-radius: 3px;
  font-size: 10px;
  font-family: monospace;
  display: flex;
  align-items: center;
  gap: 4px;
}

.color-swatch {
  width: 10px;
  height: 10px;
  border: 1px solid #666;
  border-radius: 2px;
  display: inline-block;
}

.hint-text {
  position: fixed;
  bottom: 40px;
  left: 50%;
  transform: translateX(-50%);
  background: rgba(31, 41, 55, 0.9);
  color: #9ca3af;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 13px;
  pointer-events: none;
  z-index: 10000;
  white-space: nowrap;
}
</style>
