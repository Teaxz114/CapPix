<template>
  <div
    class="pin-container"
    data-tauri-drag-region
    @wheel.prevent="onWheel"
    :style="{ opacity: opacity }"
  >
    <!-- Close button -->
    <div class="pin-close" @click.stop="close">×</div>

    <!-- Resize handle -->
    <div class="pin-resize" @mousedown.stop="startResize"></div>

    <!-- Image -->
    <img
      v-if="imageData"
      :src="`data:image/png;base64,${imageData}`"
      class="pin-image"
      :style="{ transform: `scale(${scale})`, transformOrigin: 'top left' }"
      draggable="false"
      @load="onImageLoad"
    />

    <!-- Toolbar (shown on hover) -->
    <div class="pin-toolbar">
      <button @click.stop="zoomIn" title="放大">+</button>
      <button @click.stop="zoomOut" title="缩小">−</button>
      <button @click.stop="resetZoom" title="重置">1:1</button>
      <span class="toolbar-sep"></span>
      <button @click.stop="decreaseOpacity" title="降低透明度">◐</button>
      <button @click.stop="increaseOpacity" title="增加透明度">◑</button>
      <button @click.stop="toggleClickthrough" :class="{ active: clickthrough }" title="鼠标穿透">✦</button>
      <span class="toolbar-sep"></span>
      <button @click.stop="copyImage" title="复制">复制</button>
      <button @click.stop="saveImage" title="保存">保存</button>
      <button @click.stop="close" title="关闭">关闭</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

const imageData = ref("");
const scale = ref(1);
const pinId = ref("");
const opacity = ref(1);
const clickthrough = ref(false);
let unlisten: (() => void) | null = null;

// Get pin ID from URL hash params: /index.html#/pin?id=xxx
const hash = window.location.hash; // e.g. "#/pin?id=pin-xxx"
const queryString = hash.includes("?") ? hash.split("?")[1] : "";
const params = new URLSearchParams(queryString);
pinId.value = params.get("id") || "";

onMounted(async () => {
  // Try to get image data from PendingScreenshot (stored by Rust before window creation)
  try {
    const pending = await invoke<string | null>("get_pending_screenshot");
    if (pending) {
      imageData.value = pending;
    }
  } catch (_) {}

  // Also listen for the pin-image event carrying our image data
  unlisten = await listen<{ id: string; image_base64: string }>("pin-image", (event) => {
    if (event.payload.id === pinId.value) {
      imageData.value = event.payload.image_base64;
    }
  });

  // ESC to close
  function onEscKey(e: KeyboardEvent) {
    if (e.key === "Escape") close();
  }
  window.addEventListener("keydown", onEscKey);

  // Store cleanup reference
  const originalUnlisten = unlisten;
  unlisten = async () => {
    if (originalUnlisten) await originalUnlisten();
    window.removeEventListener("keydown", onEscKey);
  };
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

function onImageLoad(e: Event) {
  const img = e.target as HTMLImageElement;
  const maxW = 800;
  const maxH = 600;
  if (img.naturalWidth > maxW || img.naturalHeight > maxH) {
    const ratioW = maxW / img.naturalWidth;
    const ratioH = maxH / img.naturalHeight;
    scale.value = Math.min(ratioW, ratioH);
  }

  const w = img.naturalWidth * scale.value;
  const h = img.naturalHeight * scale.value;
  invoke("resize_pin_window", { id: pinId.value, width: w, height: h + 4 });
}

// Zoom controls
function onWheel(e: WheelEvent) {
  if (e.deltaY < 0) {
    zoomIn();
  } else {
    zoomOut();
  }
}

function zoomIn() {
  scale.value = Math.min(3, scale.value + 0.1);
  updateWindowSize();
}

function zoomOut() {
  scale.value = Math.max(0.25, scale.value - 0.1);
  updateWindowSize();
}

function resetZoom() {
  scale.value = 1;
  updateWindowSize();
}

async function updateWindowSize() {
  const img = document.querySelector(".pin-image") as HTMLImageElement;
  if (!img) return;
  const w = img.naturalWidth * scale.value;
  const h = img.naturalHeight * scale.value;
  try {
    await invoke("resize_pin_window", { id: pinId.value, width: w, height: h + 4 });
  } catch (_) {}
}

// Opacity controls
async function increaseOpacity() {
  opacity.value = Math.min(1, opacity.value + 0.1);
  await applyOpacity();
}

async function decreaseOpacity() {
  opacity.value = Math.max(0.1, opacity.value - 0.1);
  await applyOpacity();
}

async function applyOpacity() {
  try {
    await invoke("set_pin_opacity", { id: pinId.value, opacity: opacity.value });
  } catch (e) {
    console.error("Set opacity failed:", e);
  }
}

// Clickthrough toggle
async function toggleClickthrough() {
  clickthrough.value = !clickthrough.value;
  try {
    await invoke("set_pin_clickthrough", { id: pinId.value, clickthrough: clickthrough.value });
  } catch (e) {
    console.error("Set clickthrough failed:", e);
    clickthrough.value = !clickthrough.value; // revert on error
  }
}

// Copy image to clipboard
async function copyImage() {
  try {
    await invoke("copy_image_to_clipboard", { imageBase64: imageData.value });
  } catch (e) {
    console.error("Copy failed:", e);
  }
}

// Save image to file
async function saveImage() {
  if (!imageData.value) return;
  try {
    await invoke("save_image_to_file", { imageBase64: imageData.value });
  } catch (e) {
    console.error("Save failed:", e);
  }
}

// Close this pin window
async function close() {
  try {
    // Delete from database so it won't be restored on next launch
    await invoke("pin_delete", { id: pinId.value });
    // Try to close the pin-specific window (if it's a separate webview)
    await invoke("close_pin_window", { id: pinId.value });
  } catch (e) {
    // If close_pin_window failed, we might be in main-window fallback mode
    // Don't close the main window — just navigate back to home
    const win = getCurrentWindow();
    const label = win.label;
    if (label === "main") {
      // We're in the main window — restore and go home
      try {
        await win.setDecorations(true);
        await win.setAlwaysOnTop(false);
        await win.setResizable(true);
        const { LogicalSize } = await import("@tauri-apps/api/dpi");
        await win.setSize(new LogicalSize(800, 600));
        await win.center();
        window.location.hash = '/';
      } catch (_) {}
    } else {
      // We're in a separate pin window — safe to close
      await win.close();
    }
  }
}

// Resize handle
function startResize(e: MouseEvent) {
  e.preventDefault();
  const startX = e.clientX;
  const startY = e.clientY;
  const currentWindow = getCurrentWindow();

  const onMouseMove = async (ev: MouseEvent) => {
    const dx = ev.clientX - startX;
    const dy = ev.clientY - startY;
    try {
      const size = await currentWindow.innerSize();
      const newW = Math.max(100, size.width + dx);
      const newH = Math.max(80, size.height + dy);
      await invoke("resize_pin_window", { id: pinId.value, width: newW, height: newH });
    } catch (_) {}
  };

  const onMouseUp = () => {
    document.removeEventListener("mousemove", onMouseMove);
    document.removeEventListener("mouseup", onMouseUp);
  };

  document.addEventListener("mousemove", onMouseMove);
  document.addEventListener("mouseup", onMouseUp);
}
</script>

<style scoped>
.pin-container {
  position: relative;
  width: 100vw;
  height: 100vh;
  overflow: hidden;
  background: #1a1a2e;
  border: 1px solid rgba(255, 255, 255, 0.1);
  border-radius: 4px;
  cursor: default;
  user-select: none;
  transition: opacity 0.15s;
}

.pin-image {
  display: block;
  max-width: none;
  pointer-events: none;
}

.pin-close {
  position: absolute;
  top: 2px;
  right: 2px;
  width: 22px;
  height: 22px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: rgba(239, 68, 68, 0.85);
  color: white;
  font-size: 14px;
  font-weight: bold;
  border-radius: 50%;
  cursor: pointer;
  z-index: 20;
  opacity: 0;
  transition: opacity 0.2s;
  line-height: 1;
}

.pin-container:hover .pin-close {
  opacity: 1;
}

.pin-close:hover {
  background: rgba(239, 68, 68, 1);
}

.pin-resize {
  position: absolute;
  bottom: 0;
  right: 0;
  width: 16px;
  height: 16px;
  cursor: nwse-resize;
  z-index: 20;
  opacity: 0;
  transition: opacity 0.2s;
}

.pin-container:hover .pin-resize {
  opacity: 1;
}

.pin-resize::after {
  content: "";
  position: absolute;
  bottom: 3px;
  right: 3px;
  width: 8px;
  height: 8px;
  border-right: 2px solid rgba(255, 255, 255, 0.6);
  border-bottom: 2px solid rgba(255, 255, 255, 0.6);
}

.pin-toolbar {
  position: absolute;
  bottom: 0;
  left: 50%;
  transform: translateX(-50%);
  display: flex;
  align-items: center;
  gap: 2px;
  padding: 3px 6px;
  background: rgba(31, 41, 55, 0.92);
  border-radius: 6px 6px 0 0;
  z-index: 20;
  opacity: 0;
  transition: opacity 0.2s;
  pointer-events: none;
}

.pin-container:hover .pin-toolbar {
  opacity: 1;
  pointer-events: auto;
}

.toolbar-sep {
  width: 1px;
  height: 14px;
  background: rgba(255, 255, 255, 0.2);
  margin: 0 2px;
}

.pin-toolbar button {
  background: transparent;
  border: none;
  color: #d1d5db;
  font-size: 12px;
  padding: 3px 8px;
  border-radius: 3px;
  cursor: pointer;
  transition: all 0.15s;
  white-space: nowrap;
}

.pin-toolbar button:hover {
  background: #374151;
  color: #ffffff;
}

.pin-toolbar button.active {
  background: #3b82f6;
  color: #ffffff;
}
</style>
