<template>
  <div
    class="screenshot-overlay"
    ref="overlayRef"
    tabindex="0"
    @mousedown="onMouseDown"
    @mousemove="onMouseMove"
    @mouseup="onMouseUp"
    @contextmenu.prevent="onContextMenu"
    @dblclick="onDoubleClick"
    @keydown="onKeyDown"
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
        fill="rgba(0,0,0,0.5)"
        mask="url(#selection-mask)"
      />
    </svg>

    <!-- Window detection highlight -->
    <div
      v-if="windowHighlight && !isSelecting && !hasSelection"
      class="window-highlight"
      :style="{
        left: windowHighlight.x + 'px',
        top: windowHighlight.y + 'px',
        width: windowHighlight.width + 'px',
        height: windowHighlight.height + 'px',
      }"
    >
      <div class="window-highlight-border"></div>
      <span class="window-highlight-title">{{ windowHighlight.title }}</span>
    </div>

    <!-- Context menu -->
    <div v-if="contextMenu" class="context-menu" :style="{ left: contextMenu.x + 'px', top: contextMenu.y + 'px' }">
      <button @click="captureRegionFromMenu">截取选区</button>
      <button @click="captureFullscreen">截取全屏</button>
      <button @click="pickColor">取色</button>
      <button @click="ocrRegion">OCR 识别</button>
      <hr />
      <button @click="cancelCapture">取消 (ESC)</button>
    </div>

    <!-- Selection border with handles -->
    <div
      v-if="isSelecting || hasSelection"
      class="selection-border"
      :style="{
        left: selectionX + 'px',
        top: selectionY + 'px',
        width: selectionW + 'px',
        height: selectionH + 'px',
      }"
      @mousedown.stop="onSelectionMouseDown($event)"
    >
      <!-- 4 corner handles -->
      <span class="handle handle-tl" data-handle="tl"></span>
      <span class="handle handle-tr" data-handle="tr"></span>
      <span class="handle handle-bl" data-handle="bl"></span>
      <span class="handle handle-br" data-handle="br"></span>
      <!-- 4 edge midpoint handles -->
      <span class="handle handle-tm" data-handle="tm"></span>
      <span class="handle handle-bm" data-handle="bm"></span>
      <span class="handle handle-ml" data-handle="ml"></span>
      <span class="handle handle-mr" data-handle="mr"></span>
    </div>

    <!-- Dimension info -->
    <div
      v-if="isSelecting || hasSelection"
      class="selection-info"
      :style="infoPosition"
    >
      {{ selectionW }} × {{ selectionH }}
    </div>

    <!-- ★ Action toolbar — PixPin style: appears below selection after drag -->
    <div
      v-if="hasSelection && !isSelecting"
      class="action-toolbar"
      :style="toolbarPosition"
      @mousedown.stop
    >
      <button class="toolbar-btn" @click="actionAnnotate" title="标注">
        <span class="toolbar-icon">✏️</span>
        <span class="toolbar-label">标注</span>
      </button>
      <button class="toolbar-btn" @click="actionCopy" title="复制">
        <span class="toolbar-icon">📋</span>
        <span class="toolbar-label">复制</span>
      </button>
      <button class="toolbar-btn" @click="actionPin" title="贴图">
        <span class="toolbar-icon">📌</span>
        <span class="toolbar-label">贴图</span>
      </button>
      <button class="toolbar-btn" @click="actionOcr" title="OCR">
        <span class="toolbar-icon">🔍</span>
        <span class="toolbar-label">OCR</span>
      </button>
      <button class="toolbar-btn" @click="actionSave" title="保存">
        <span class="toolbar-icon">💾</span>
        <span class="toolbar-label">保存</span>
      </button>
      <div class="toolbar-separator"></div>
      <button class="toolbar-btn toolbar-btn-cancel" @click="cancelCapture" title="取消">
        <span class="toolbar-icon">✕</span>
      </button>
    </div>

    <!-- OCR Panel -->
    <OcrPanel ref="ocrPanelRef" />

    <!-- Magnifier -->
    <div
      v-if="showMagnifier && screenshotData && !isSelecting && !hasSelection"
      class="magnifier"
      :class="{ 'color-picker-mode': colorPickerMode }"
      :style="{
        left: magnifierX + 'px',
        top: magnifierY + 'px',
      }"
    >
      <canvas ref="magnifierCanvas" width="120" height="120"></canvas>
      <div class="magnifier-crosshair"></div>
      <div class="magnifier-color" v-if="pixelColor">
        <span class="color-swatch" :style="{ background: pixelColor }"></span>
        {{ colorPickerMode ? formattedColor() : pixelColor }}
        <span v-if="colorPickerMode" class="color-format-hint">{{ colorFormat.toUpperCase() }} · 右键切换 · 点击复制</span>
      </div>
    </div>

    <!-- Hint text -->
    <div v-if="!isSelecting && !hasSelection" class="hint-text">
      <template v-if="colorPickerMode">
        🎨 取色模式 · 点击复制颜色 · 右键切换格式(HEX/RGB/HSL) · ESC 退出
      </template>
      <template v-else>
        拖拽选择区域 · 点击窗口智能识别 · 双击全屏截图 · ESC 取消
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watch, nextTick } from "vue";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import OcrPanel from "../components/OcrPanel.vue";

const overlayRef = ref<HTMLDivElement | null>(null);
const magnifierCanvas = ref<HTMLCanvasElement | null>(null);
const ocrPanelRef = ref<InstanceType<typeof OcrPanel> | null>(null);

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

// Captured region data (filled when user completes a selection)
let capturedBase64 = "";

// Magnifier state
const showMagnifier = ref(true);
const cursorX = ref(0);
const cursorY = ref(0);
const pixelColor = ref("");
const colorPickerMode = ref(false);
const colorFormat = ref<"hex" | "rgb" | "hsl">("hex");

// Selection resize/move state
type HandleId = "tl" | "tr" | "bl" | "br" | "tm" | "bm" | "ml" | "mr" | "move" | null;
const activeHandle = ref<HandleId>(null);
const dragStartX = ref(0);
const dragStartY = ref(0);
const dragStartSelX = ref(0);
const dragStartSelY = ref(0);
const dragStartEndX = ref(0);
const dragStartEndY = ref(0);

// Window detection state
interface WindowRegion {
  x: number;
  y: number;
  width: number;
  height: number;
  title: string;
  hwnd: number;
}

const windowHighlight = ref<WindowRegion | null>(null);
let windowDetectTimer: ReturnType<typeof setTimeout> | null = null;
const WINDOW_DETECT_DEBOUNCE = 150;
const contextMenu = ref<{ x: number; y: number } | null>(null);

// Virtual screen offset
let virtualScreenOffsetX = 0;
let virtualScreenOffsetY = 0;

// Computed selection bounds
const selectionX = computed(() => Math.min(startX.value, endX.value));
const selectionY = computed(() => Math.min(startY.value, endY.value));
const selectionW = computed(() => Math.abs(endX.value - startX.value));
const selectionH = computed(() => Math.abs(endY.value - startY.value));

// Info position (near selection)
const infoPosition = computed(() => {
  const x = selectionX.value;
  const y = selectionY.value;
  const w = selectionW.value;
  const h = selectionH.value;
  let infoX = x + w / 2 - 40;
  let infoY = y + h + 8;
  if (infoY + 30 > window.innerHeight) {
    infoY = y - 28;
  }
  infoX = Math.max(4, Math.min(infoX, window.innerWidth - 100));
  return { left: infoX + "px", top: infoY + "px" };
});

// Toolbar position (below selection, centered)
const toolbarPosition = computed(() => {
  const x = selectionX.value;
  const y = selectionY.value;
  const w = selectionW.value;
  const h = selectionH.value;
  const toolbarWidth = 320;
  let tbX = x + w / 2 - toolbarWidth / 2;
  let tbY = y + h + 8;
  // If too close to bottom, place above
  if (tbY + 50 > window.innerHeight) {
    tbY = y - 50;
  }
  // Clamp horizontal
  tbX = Math.max(4, Math.min(tbX, window.innerWidth - toolbarWidth - 4));
  return { left: tbX + "px", top: tbY + "px" };
});

// Magnifier position
const magnifierX = computed(() => cursorX.value + 20);
const magnifierY = computed(() => cursorY.value - 140);

// Load screenshot image for magnifier
watch(screenshotData, (data) => {
  if (data) {
    const img = new Image();
    img.onload = () => {
      screenshotImage.value = img;
    };
    img.src = `data:image/png;base64,${data}`;
  }
});

async function fetchVirtualScreenOffset() {
  try {
    const screens = await invoke<Array<{ id: number; x: number; y: number; width: number; height: number; is_primary: boolean }>>("get_screens");
    if (screens.length > 0) {
      virtualScreenOffsetX = Math.min(...screens.map(s => s.x));
      virtualScreenOffsetY = Math.min(...screens.map(s => s.y));
    }
  } catch (e) {
    console.error("Failed to get screen info:", e);
  }
}

function detectWindowAtCursor(clientX: number, clientY: number) {
  if (windowDetectTimer) clearTimeout(windowDetectTimer);
  windowDetectTimer = setTimeout(async () => {
    try {
      const screenX = clientX + virtualScreenOffsetX;
      const screenY = clientY + virtualScreenOffsetY;
      const result = await invoke<WindowRegion | null>("get_window_at_point", { x: screenX, y: screenY });
      if (result) {
        windowHighlight.value = {
          x: result.x - virtualScreenOffsetX,
          y: result.y - virtualScreenOffsetY,
          width: result.width,
          height: result.height,
          title: result.title,
          hwnd: result.hwnd,
        };
      } else {
        windowHighlight.value = null;
      }
    } catch (e) {
      windowHighlight.value = null;
    }
  }, WINDOW_DETECT_DEBOUNCE);
}

let unlisten: (() => void) | null = null;

onMounted(async () => {
  // Get pending screenshot data (reliable pull, no timing issues)
  try {
    const pending = await invoke<string | null>("get_pending_screenshot");
    if (pending) screenshotData.value = pending;
  } catch (e) {
    console.error("Failed to get pending screenshot:", e);
  }

  // Fallback: listen for screenshot-ready event
  try {
    unlisten = await listen<string>("screenshot-ready", (event) => {
      screenshotData.value = event.payload;
    });
  } catch (e) {
    console.error("Failed to listen for screenshot-ready:", e);
  }

  await fetchVirtualScreenOffset();

  // Use document for keyboard events (no need for both document + window)
  document.addEventListener("keydown", onKeyDown);

  // Ensure focus — try multiple times with delay
  try { await getCurrentWindow().setFocus(); } catch (_) {}
  nextTick(() => {
    overlayRef.value?.focus();
  });
  setTimeout(async () => {
    try { await getCurrentWindow().setFocus(); } catch (_) {}
    overlayRef.value?.focus();
  }, 100);
  setTimeout(async () => {
    try { await getCurrentWindow().setFocus(); } catch (_) {}
    overlayRef.value?.focus();
  }, 300);
});

onUnmounted(() => {
  if (unlisten) unlisten();
  if (windowDetectTimer) clearTimeout(windowDetectTimer);
  document.removeEventListener("keydown", onKeyDown);
});

function onKeyDown(e: KeyboardEvent) {
  if (e.key === "Escape") {
    // Exit color picker mode first
    if (colorPickerMode.value) {
      colorPickerMode.value = false;
      colorFormat.value = "hex";
      return;
    }
    if (hasSelection.value) {
      // Cancel selection, go back to overlay mode
      hasSelection.value = false;
      capturedBase64 = "";
    } else {
      // Restore main window to normal mode and navigate back to home
      cancelCapture();
    }
  }
  // Enter key = confirm selection (go to annotate)
  if (e.key === "Enter" && hasSelection.value && capturedBase64) {
    navigateToAnnotate(capturedBase64);
  }
}

function onMouseDown(e: MouseEvent) {
  if (e.button !== 0) return;
  contextMenu.value = null;

  // Color picker mode: left click confirms color pick
  if (colorPickerMode.value) {
    e.preventDefault();
    confirmColorPick();
    return;
  }

  // If we have a selection with toolbar showing, start move/resize
  if (hasSelection.value) {
    const sx = selectionX.value;
    const sy = selectionY.value;
    const sw = selectionW.value;
    const sh = selectionH.value;
    const insideSelection =
      e.clientX >= sx && e.clientX <= sx + sw &&
      e.clientY >= sy && e.clientY <= sy + sh;
    if (insideSelection) {
      // Start moving the selection
      startSelectionDrag(e, "move");
      return;
    }
    // Click outside selection — start new selection
  }

  isSelecting.value = true;
  hasSelection.value = false;
  capturedBase64 = "";
  startX.value = e.clientX;
  startY.value = e.clientY;
  endX.value = e.clientX;
  endY.value = e.clientY;
  showMagnifier.value = false;

  windowHighlight.value = null;
  if (windowDetectTimer) {
    clearTimeout(windowDetectTimer);
    windowDetectTimer = null;
  }
}

function onMouseMove(e: MouseEvent) {
  cursorX.value = e.clientX;
  cursorY.value = e.clientY;

  // Handle selection drag (move or resize)
  if (activeHandle.value) {
    handleSelectionDrag(e);
    return;
  }

  if (isSelecting.value) {
    endX.value = e.clientX;
    endY.value = e.clientY;
  } else if (!hasSelection.value) {
    showMagnifier.value = true;
    updateMagnifier(e.clientX, e.clientY);
    detectWindowAtCursor(e.clientX, e.clientY);
  }
}

function onMouseUp(e: MouseEvent) {
  // End selection drag (move or resize)
  if (activeHandle.value) {
    endSelectionDrag();
    return;
  }

  if (!isSelecting.value) return;
  isSelecting.value = false;
  endX.value = e.clientX;
  endY.value = e.clientY;

  const w = selectionW.value;
  const h = selectionH.value;

  // Small click = window capture
  if (w < 5 || h < 5) {
    hasSelection.value = false;
    if (windowHighlight.value) {
      const win = windowHighlight.value;
      // Capture window region and go to annotate directly
      doCaptureRegion(
        win.x + virtualScreenOffsetX,
        win.y + virtualScreenOffsetY,
        win.width,
        win.height,
        true // auto-navigate
      );
      windowHighlight.value = null;
    }
    return;
  }

  // Valid selection — capture the region but DON'T auto-navigate
  // Show action toolbar instead (PixPin style)
  hasSelection.value = true;
  doCaptureRegion(
    selectionX.value + virtualScreenOffsetX,
    selectionY.value + virtualScreenOffsetY,
    w,
    h,
    false // don't auto-navigate, wait for toolbar action
  );
}

async function doCaptureRegion(x: number, y: number, w: number, h: number, autoNavigate: boolean) {
  // IMPORTANT: Don't re-capture the screen! The overlay is on screen,
  // so capture_region would include the overlay UI. Instead, crop from
  // the original full-screen screenshot data we already have.
  if (!screenshotData.value) {
    console.error("No screenshot data available for cropping");
    return;
  }
  try {
    // Convert overlay-local logical pixels to screenshot image pixels
    // The screenshot image may be larger than the overlay window due to DPI scaling
    const scaleX = screenshotImage.value ? screenshotImage.value.naturalWidth / window.innerWidth : 1;
    const scaleY = screenshotImage.value ? screenshotImage.value.naturalHeight / window.innerHeight : 1;

    const croppedBase64 = await invoke<string>("crop_image", {
      imageBase64: screenshotData.value,
      x: Math.round(x * scaleX),
      y: Math.round(y * scaleY),
      width: Math.round(w * scaleX),
      height: Math.round(h * scaleY),
    });
    capturedBase64 = croppedBase64;
    if (autoNavigate) {
      await navigateToAnnotate(croppedBase64);
    }
  } catch (err) {
    console.error("Failed to crop region:", err);
  }
}

async function onDoubleClick() {
  if (screenshotData.value) {
    await navigateToAnnotate(screenshotData.value);
  } else {
    try {
      const result = await invoke<{ image_base64: string; width: number; height: number }>("capture_fullscreen");
      await navigateToAnnotate(result.image_base64);
    } catch (err) {
      console.error("Failed to capture fullscreen:", err);
    }
  }
}

// ===== Action toolbar handlers =====

async function actionAnnotate() {
  if (capturedBase64) {
    await navigateToAnnotate(capturedBase64);
  }
}

async function actionPin() {
  if (!capturedBase64) return;
  try {
    await invoke("create_pin_window", { imageBase64: capturedBase64 });
  } catch (e) {
    console.error("Pin failed:", e);
    return;
  }
  await restoreMainWindow();
}

async function actionOcr() {
  if (!capturedBase64) return;
  // Show OCR panel with results
  if (ocrPanelRef.value) {
    await ocrPanelRef.value.recognize(capturedBase64);
  }
}

async function actionSave() {
  if (!capturedBase64) return;
  try {
    // Exit fullscreen overlay mode so save dialog is visible
    const win = getCurrentWindow();
    await win.setAlwaysOnTop(false);
    await win.setDecorations(true);
    await invoke("save_image_to_file", { imageBase64: capturedBase64 });
    saveToHistory();
  } catch (e) {
    // User cancelled save dialog or error — don't close overlay
    console.error("Save failed:", e);
    return;
  }
  await restoreMainWindow();
}

async function actionCopy() {
  if (!capturedBase64) return;
  try {
    await invoke("copy_image_to_clipboard", { imageBase64: capturedBase64 });
    saveToHistory();
  } catch (e) {
    console.error("Copy failed:", e);
    return;
  }
  await restoreMainWindow();
}

function saveToHistory() {
  if (!capturedBase64) return;
  invoke("history_save", {
    imageBase64: capturedBase64,
    width: Math.round(selectionW.value),
    height: Math.round(selectionH.value),
    source: "region",
    ocrText: null,
  }).catch((e) => console.error("History save failed:", e));
}

async function navigateToAnnotate(imageBase64: string) {
  try {
    await invoke("open_annotate_window", { imageBase64 });
    // Only navigate if invoke succeeded (data is stored in PendingAnnotateImage)
    window.location.hash = '/annotate';
  } catch (e) {
    console.error("Failed to open annotate window:", e);
    // Don't navigate — no image data available
  }
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
  ctx.drawImage(img, srcX - srcSize / 2, srcY - srcSize / 2, srcSize, srcSize, 0, 0, canvas.width, canvas.height);

  ctx.strokeStyle = "rgba(255,255,255,0.3)";
  ctx.lineWidth = 0.5;
  const cellSize = canvas.width / srcSize;
  for (let i = 0; i <= srcSize; i++) {
    ctx.beginPath(); ctx.moveTo(i * cellSize, 0); ctx.lineTo(i * cellSize, canvas.height); ctx.stroke();
    ctx.beginPath(); ctx.moveTo(0, i * cellSize); ctx.lineTo(canvas.width, i * cellSize); ctx.stroke();
  }

  try {
    const pixel = ctx.getImageData(canvas.width / 2, canvas.height / 2, 1, 1).data;
    pixelColor.value = `#${pixel[0].toString(16).padStart(2, "0")}${pixel[1].toString(16).padStart(2, "0")}${pixel[2].toString(16).padStart(2, "0")}`;
  } catch { pixelColor.value = ""; }
}

/** Start dragging a selection handle or moving the selection */
function onSelectionMouseDown(e: MouseEvent) {
  const target = e.target as HTMLElement;
  const handleId = target.dataset.handle as HandleId;
  if (handleId) {
    e.preventDefault();
    startSelectionDrag(e, handleId);
  }
}

function startSelectionDrag(e: MouseEvent, handle: HandleId) {
  activeHandle.value = handle;
  dragStartX.value = e.clientX;
  dragStartY.value = e.clientY;
  // Store original startX/Y and endX/Y (these are the raw refs, not computed)
  dragStartSelX.value = startX.value;
  dragStartSelY.value = startY.value;
  dragStartEndX.value = endX.value;
  dragStartEndY.value = endY.value;
  showMagnifier.value = false;
}

function handleSelectionDrag(e: MouseEvent) {
  const dx = e.clientX - dragStartX.value;
  const dy = e.clientY - dragStartY.value;
  const handle = activeHandle.value;
  if (!handle) return;

  const sx = dragStartSelX.value;
  const sy = dragStartSelY.value;
  const ex = dragStartEndX.value;
  const ey = dragStartEndY.value;

  switch (handle) {
    case "move":
      startX.value = sx + dx;
      startY.value = sy + dy;
      endX.value = ex + dx;
      endY.value = ey + dy;
      break;
    case "tl":
      startX.value = Math.min(sx + dx, ex - 10);
      startY.value = Math.min(sy + dy, ey - 10);
      break;
    case "tr":
      endX.value = Math.max(ex + dx, sx + 10);
      startY.value = Math.min(sy + dy, ey - 10);
      break;
    case "bl":
      startX.value = Math.min(sx + dx, ex - 10);
      endY.value = Math.max(ey + dy, sy + 10);
      break;
    case "br":
      endX.value = Math.max(ex + dx, sx + 10);
      endY.value = Math.max(ey + dy, sy + 10);
      break;
    case "tm":
      startY.value = Math.min(sy + dy, ey - 10);
      break;
    case "bm":
      endY.value = Math.max(ey + dy, sy + 10);
      break;
    case "ml":
      startX.value = Math.min(sx + dx, ex - 10);
      break;
    case "mr":
      endX.value = Math.max(ex + dx, sx + 10);
      break;
  }
}

function endSelectionDrag() {
  activeHandle.value = null;
  // Re-capture the new selection region
  const w = selectionW.value;
  const h = selectionH.value;
  if (w >= 5 && h >= 5) {
    doCaptureRegion(
      selectionX.value + virtualScreenOffsetX,
      selectionY.value + virtualScreenOffsetY,
      w,
      h,
      false
    );
  }
}

function onContextMenu(e: MouseEvent) {
  // In color picker mode, right click cycles color format
  if (colorPickerMode.value) {
    e.preventDefault();
    cycleColorFormat();
    return;
  }
  contextMenu.value = { x: e.clientX, y: e.clientY };
}

function captureRegionFromMenu() {
  contextMenu.value = null;
  if (hasSelection.value) {
    doCaptureRegion(
      selectionX.value + virtualScreenOffsetX,
      selectionY.value + virtualScreenOffsetY,
      selectionW.value,
      selectionH.value,
      true
    );
  } else if (windowHighlight.value) {
    const win = windowHighlight.value;
    doCaptureRegion(
      win.x + virtualScreenOffsetX,
      win.y + virtualScreenOffsetY,
      win.width,
      win.height,
      true
    );
  }
}

function captureFullscreen() {
  contextMenu.value = null;
  if (screenshotData.value) navigateToAnnotate(screenshotData.value);
}

function pickColor() {
  contextMenu.value = null;
  colorPickerMode.value = true;
}

/** Convert hex color to RGB string */
function hexToRgb(hex: string): string {
  const r = parseInt(hex.slice(1, 3), 16);
  const g = parseInt(hex.slice(3, 5), 16);
  const b = parseInt(hex.slice(5, 7), 16);
  return `rgb(${r}, ${g}, ${b})`;
}

/** Convert hex color to HSL string */
function hexToHsl(hex: string): string {
  let r = parseInt(hex.slice(1, 3), 16) / 255;
  let g = parseInt(hex.slice(3, 5), 16) / 255;
  let b = parseInt(hex.slice(5, 7), 16) / 255;
  const max = Math.max(r, g, b), min = Math.min(r, g, b);
  let h = 0, s = 0;
  const l = (max + min) / 2;
  if (max !== min) {
    const d = max - min;
    s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    if (max === r) h = ((g - b) / d + (g < b ? 6 : 0)) / 6;
    else if (max === g) h = ((b - r) / d + 2) / 6;
    else h = ((r - g) / d + 4) / 6;
  }
  return `hsl(${Math.round(h * 360)}, ${Math.round(s * 100)}%, ${Math.round(l * 100)}%)`;
}

/** Get formatted color string based on current format */
function formattedColor(): string {
  if (!pixelColor.value) return "";
  switch (colorFormat.value) {
    case "rgb": return hexToRgb(pixelColor.value);
    case "hsl": return hexToHsl(pixelColor.value);
    default: return pixelColor.value;
  }
}

/** Cycle color format on click in picker mode */
function cycleColorFormat() {
  colorFormat.value = colorFormat.value === "hex" ? "rgb" : colorFormat.value === "rgb" ? "hsl" : "hex";
}

/** Copy current color and exit picker mode */
async function confirmColorPick() {
  if (pixelColor.value) {
    const text = formattedColor();
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      // Fallback: Tauri clipboard plugin
      try {
        const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
        await writeText(text);
      } catch { /* ignore */ }
    }
  }
  colorPickerMode.value = false;
  colorFormat.value = "hex";
}

function ocrRegion() {
  contextMenu.value = null;
  if (screenshotData.value && ocrPanelRef.value) {
    ocrPanelRef.value.recognize(screenshotData.value);
  }
}

async function cancelCapture() {
  contextMenu.value = null;
  await restoreMainWindow();
}

async function restoreMainWindow() {
  // Restore main window from fullscreen overlay mode to normal mode
  try {
    const win = getCurrentWindow();
    await win.setDecorations(true);
    await win.setAlwaysOnTop(false);
    await win.setResizable(true);
    await win.setSize(new (await import("@tauri-apps/api/dpi")).LogicalSize(800, 600));
    await win.center();
    // Navigate back to home
    window.location.hash = '/';
  } catch (e) {
    console.error("Failed to restore window:", e);
  }
}
</script>

<style scoped>
.screenshot-overlay {
  position: fixed;
  top: 0; left: 0;
  width: 100vw; height: 100vh;
  cursor: crosshair;
  background: #000;
  z-index: 9999;
  user-select: none;
}

.screenshot-bg {
  position: fixed;
  top: 0; left: 0;
  width: 100%; height: 100%;
  pointer-events: none;
  user-select: none;
}

.overlay-svg {
  position: fixed;
  top: 0; left: 0;
  width: 100%; height: 100%;
  pointer-events: none;
}

.window-highlight {
  position: fixed;
  pointer-events: none;
  z-index: 5;
}

.window-highlight-border {
  position: absolute;
  inset: 0;
  border: 2px solid rgba(59, 130, 246, 0.85);
  background: rgba(59, 130, 246, 0.08);
  border-radius: 2px;
}

.window-highlight-title {
  position: absolute;
  top: -24px; left: 0;
  background: #1f2937;
  color: #e5e7eb;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 11px;
  white-space: nowrap;
  max-width: 200px;
  overflow: hidden;
  text-overflow: ellipsis;
}

.context-menu {
  position: fixed;
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 6px;
  padding: 4px 0;
  z-index: 100;
  min-width: 120px;
  box-shadow: 0 4px 12px rgba(0,0,0,0.5);
}
.context-menu button {
  display: block; width: 100%;
  background: none; border: none;
  color: #e5e7eb; padding: 8px 16px;
  font-size: 13px; cursor: pointer; text-align: left;
}
.context-menu button:hover { background: #374151; }
.context-menu hr { border: none; border-top: 1px solid #374151; margin: 4px 0; }

.selection-border {
  position: fixed;
  border: 2px solid #3b82f6;
  z-index: 10;
  cursor: move;
}

.handle {
  position: absolute;
  width: 8px; height: 8px;
  background: #3b82f6;
  border: 1px solid white;
  border-radius: 1px;
  z-index: 11;
}
.handle-tl { top: -4px; left: -4px; cursor: nwse-resize; }
.handle-tr { top: -4px; right: -4px; cursor: nesw-resize; }
.handle-bl { bottom: -4px; left: -4px; cursor: nesw-resize; }
.handle-br { bottom: -4px; right: -4px; cursor: nwse-resize; }
.handle-tm { top: -4px; left: 50%; transform: translateX(-50%); cursor: ns-resize; }
.handle-bm { bottom: -4px; left: 50%; transform: translateX(-50%); cursor: ns-resize; }
.handle-ml { top: 50%; left: -4px; transform: translateY(-50%); cursor: ew-resize; }
.handle-mr { top: 50%; right: -4px; transform: translateY(-50%); cursor: ew-resize; }

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

/* ===== Action toolbar — PixPin style ===== */
.action-toolbar {
  position: fixed;
  display: flex;
  align-items: center;
  gap: 2px;
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  padding: 4px 6px;
  z-index: 9999;
  box-shadow: 0 4px 16px rgba(0,0,0,0.4);
  animation: toolbar-in 0.15s ease-out;
}

@keyframes toolbar-in {
  from { opacity: 0; transform: translateY(4px); }
  to { opacity: 1; transform: translateY(0); }
}

.toolbar-btn {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 1px;
  background: none;
  border: none;
  color: #d1d5db;
  padding: 4px 8px;
  border-radius: 6px;
  cursor: pointer;
  min-width: 44px;
  transition: background 0.1s;
}
.toolbar-btn:hover { background: #374151; color: #fff; }
.toolbar-btn:active { background: #4b5563; }

.toolbar-icon { font-size: 16px; line-height: 1; }
.toolbar-label { font-size: 10px; line-height: 1; }

.toolbar-btn-cancel { color: #9ca3af; }
.toolbar-btn-cancel:hover { background: #7f1d1d; color: #fca5a5; }

.toolbar-separator {
  width: 1px;
  height: 28px;
  background: #374151;
  margin: 0 2px;
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
  top: 58px; left: 58px;
  width: 6px; height: 6px;
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
  width: 10px; height: 10px;
  border: 1px solid #666;
  border-radius: 2px;
  display: inline-block;
}
.magnifier.color-picker-mode canvas {
  border-color: #f59e0b;
  box-shadow: 0 0 8px rgba(245, 158, 11, 0.5);
}
.magnifier.color-picker-mode .magnifier-crosshair {
  border-color: #f59e0b;
}
.color-picker-mode .magnifier-color {
  background: #78350f;
  flex-direction: column;
  align-items: flex-start;
  padding: 4px 8px;
}
.color-format-hint {
  color: #fbbf24;
  font-size: 8px;
  opacity: 0.8;
}
.hint-text {
  position: fixed;
  bottom: 40px; left: 50%;
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
