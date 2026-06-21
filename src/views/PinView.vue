<template>
  <div
    class="pin-container"
    data-tauri-drag-region
    @wheel.prevent="onWheel"
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
      <button @click.stop="copyImage" title="复制">复制</button>
      <button @click.stop="close" title="关闭">关闭</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";

const imageData = ref("");
const scale = ref(1);
const pinId = ref("");
let unlisten: UnlistenFn | null = null;

// Get pin ID from URL params
const params = new URLSearchParams(window.location.search);
pinId.value = params.get("id") || "";

onMounted(async () => {
  // Listen for the pin-image event carrying our image data
  unlisten = await listen<{ id: string; image_base64: string }>("pin-image", (event) => {
    if (event.payload.id === pinId.value) {
      imageData.value = event.payload.image_base64;
    }
  });
});

onUnmounted(() => {
  if (unlisten) unlisten();
});

function onImageLoad(e: Event) {
  const img = e.target as HTMLImageElement;
  // If the image is larger than 800x600, scale it down to fit
  const maxW = 800;
  const maxH = 600;
  if (img.naturalWidth > maxW || img.naturalHeight > maxH) {
    const ratioW = maxW / img.naturalWidth;
    const ratioH = maxH / img.naturalHeight;
    scale.value = Math.min(ratioW, ratioH);
  }

  // Resize the window to fit the scaled image
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
  } catch (_) {
    // ignore resize errors
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

// Close this pin window
async function close() {
  try {
    await invoke("close_pin_window", { id: pinId.value });
  } catch (_) {
    // fallback: close directly
    await getCurrentWindow().close();
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
    // We need to read current size, then apply delta
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
</style>
