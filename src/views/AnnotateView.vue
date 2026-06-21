<template>
  <div class="annotate-view">
    <Toolbar
      :current-tool="currentTool"
      :current-color="currentColor"
      :current-stroke-width="currentStrokeWidth"
      :can-undo="historyIndex > 0"
      :can-redo="historyIndex < historyStack.length - 1"
      @tool-change="onToolChange"
      @style-change="onStyleChange"
      @undo="undo"
      @redo="redo"
      @save="saveToFile"
      @copy="copyToClipboard"
      @pin="pinToDesktop"
    />
    <Canvas
      ref="canvasRef"
      :image-base64="imageBase64"
    />
    <div v-if="statusMessage" class="status-bar">
      {{ statusMessage }}
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { Canvas as FabricCanvas, Rect, Ellipse, Line, IText, Path, PencilBrush } from "fabric";
import Toolbar from "../components/Toolbar.vue";
import CanvasComponent from "../components/Canvas.vue";

const canvasRef = ref<InstanceType<typeof CanvasComponent> | null>(null);
const imageBase64 = ref("");
const currentTool = ref("rect");
const currentColor = ref("#ff4444");
const currentStrokeWidth = ref(3);
const statusMessage = ref("");
const historyStack = ref<string[]>([]);
const historyIndex = ref(-1);

let fabricCanvas: FabricCanvas | null = null;
let isDrawing = false;
let startX = 0;
let startY = 0;
let activeObject: Rect | Ellipse | Line | IText | Path | null = null;
let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  // Listen for screenshot data
  unlisten = await listen<string>("annotate-image", (event) => {
    imageBase64.value = event.payload;
    setTimeout(initCanvasTools, 300);
  });

  // Listen for direct image prop via URL params
  const params = new URLSearchParams(window.location.search);
  const imgData = params.get("img");
  if (imgData) {
    imageBase64.value = imgData;
  }

  setTimeout(initCanvasTools, 500);
});

onUnmounted(() => {
  if (unlisten) unlisten();
  cleanupCanvasListeners();
});

function initCanvasTools() {
  const canvasInstance = canvasRef.value?.getCanvas?.();
  if (!canvasInstance) return;

  fabricCanvas = canvasInstance as FabricCanvas;

  // Save initial state
  saveHistory();

  // Set up canvas event listeners
  fabricCanvas.on("mouse:down", onMouseDown);
  fabricCanvas.on("mouse:move", onMouseMove);
  fabricCanvas.on("mouse:up", onMouseUp);

  // Track object modifications for history
  fabricCanvas.on("object:modified", () => saveHistory());
  fabricCanvas.on("path:created", () => saveHistory());

  // Keyboard shortcuts
  document.addEventListener("keydown", onKeyDown);
}

function cleanupCanvasListeners() {
  if (fabricCanvas) {
    fabricCanvas.off("mouse:down", onMouseDown);
    fabricCanvas.off("mouse:move", onMouseMove);
    fabricCanvas.off("mouse:up", onMouseUp);
    fabricCanvas.off("object:modified");
    fabricCanvas.off("path:created");
  }
  document.removeEventListener("keydown", onKeyDown);
}

function onToolChange(tool: string) {
  currentTool.value = tool;
  if (!fabricCanvas) return;

  // Reset drawing mode
  fabricCanvas.isDrawingMode = false;
  fabricCanvas.selection = false;
  fabricCanvas.defaultCursor = "crosshair";

  // Deselect all objects
  fabricCanvas.discardActiveObject();
  fabricCanvas.renderAll();

  if (tool === "pencil") {
    fabricCanvas.isDrawingMode = true;
    fabricCanvas.freeDrawingBrush = new PencilBrush(fabricCanvas);
    fabricCanvas.freeDrawingBrush.color = currentColor.value;
    fabricCanvas.freeDrawingBrush.width = currentStrokeWidth.value;
  } else if (tool === "eraser") {
    fabricCanvas.selection = true;
    fabricCanvas.defaultCursor = "pointer";
  } else if (tool === "text") {
    fabricCanvas.defaultCursor = "text";
  }
}

function onStyleChange(style: { color?: string; strokeWidth?: number }) {
  if (style.color) currentColor.value = style.color;
  if (style.strokeWidth !== undefined) currentStrokeWidth.value = style.strokeWidth;

  if (fabricCanvas?.isDrawingMode && fabricCanvas.freeDrawingBrush) {
    fabricCanvas.freeDrawingBrush.color = currentColor.value;
    fabricCanvas.freeDrawingBrush.width = currentStrokeWidth.value;
  }
}

function onMouseDown(opt: any) {
  if (!fabricCanvas) return;
  const pointer = fabricCanvas.getScenePoint(opt.e);
  isDrawing = true;
  startX = pointer.x;
  startY = pointer.y;

  if (currentTool.value === "eraser") {
    const target = fabricCanvas.findTarget(opt.e);
    if (target) {
      fabricCanvas.remove(target);
      fabricCanvas.renderAll();
      saveHistory();
    }
    return;
  }

  if (currentTool.value === "text") {
    const text = new ITEXT("文字", {
      left: pointer.x,
      top: pointer.y,
      fontSize: 20,
      fill: currentColor.value,
      fontFamily: "Microsoft YaHei, sans-serif",
    });
    fabricCanvas.add(text);
    fabricCanvas.setActiveObject(text);
    text.enterEditing();
    saveHistory();
    isDrawing = false;
    return;
  }

  if (currentTool.value === "mosaic") {
    // Mosaic: draw a semi-transparent rect as placeholder
    const rect = new Rect({
      left: pointer.x,
      top: pointer.y,
      width: 0,
      height: 0,
      fill: "rgba(128,128,128,0.8)",
      stroke: "transparent",
      selectable: true,
    });
    activeObject = rect;
    fabricCanvas.add(rect);
    return;
  }

  // Shape tools
  if (currentTool.value === "rect") {
    const rect = new Rect({
      left: pointer.x,
      top: pointer.y,
      width: 0,
      height: 0,
      fill: "transparent",
      stroke: currentColor.value,
      strokeWidth: currentStrokeWidth.value,
      selectable: true,
    });
    activeObject = rect;
    fabricCanvas.add(rect);
  } else if (currentTool.value === "ellipse") {
    const ellipse = new Ellipse({
      left: pointer.x,
      top: pointer.y,
      rx: 0,
      ry: 0,
      fill: "transparent",
      stroke: currentColor.value,
      strokeWidth: currentStrokeWidth.value,
      selectable: true,
    });
    activeObject = ellipse;
    fabricCanvas.add(ellipse);
  } else if (currentTool.value === "line") {
    const line = new Line([pointer.x, pointer.y, pointer.x, pointer.y], {
      stroke: currentColor.value,
      strokeWidth: currentStrokeWidth.value,
      selectable: true,
    });
    activeObject = line;
    fabricCanvas.add(line);
  } else if (currentTool.value === "arrow") {
    const arrow = new Line([pointer.x, pointer.y, pointer.x, pointer.y], {
      stroke: currentColor.value,
      strokeWidth: currentStrokeWidth.value,
      selectable: true,
    });
    activeObject = arrow;
    fabricCanvas.add(arrow);
  }
}

function onMouseMove(opt: any) {
  if (!fabricCanvas || !isDrawing || !activeObject) return;
  const pointer = fabricCanvas.getScenePoint(opt.e);

  if (currentTool.value === "rect" || currentTool.value === "mosaic") {
    const left = Math.min(startX, pointer.x);
    const top = Math.min(startY, pointer.y);
    const width = Math.abs(pointer.x - startX);
    const height = Math.abs(pointer.y - startY);
    activeObject.set({ left, top, width, height });
  } else if (currentTool.value === "ellipse") {
    const left = Math.min(startX, pointer.x);
    const top = Math.min(startY, pointer.y);
    const rx = Math.abs(pointer.x - startX) / 2;
    const ry = Math.abs(pointer.y - startY) / 2;
    activeObject.set({ left, top, rx, ry });
  } else if (currentTool.value === "line" || currentTool.value === "arrow") {
    activeObject.set({ x2: pointer.x, y2: pointer.y });
  }

  fabricCanvas.renderAll();
}

function onMouseUp() {
  if (!fabricCanvas || !isDrawing) return;
  isDrawing = false;

  // Apply mosaic pixelation if needed
  if (currentTool.value === "mosaic" && activeObject) {
    applyMosaic(activeObject as Rect);
  }

  activeObject = null;
  fabricCanvas.renderAll();
  saveHistory();
}

function applyMosaic(rect: Rect) {
  // Simple mosaic: just keep the gray overlay for now
  // Full pixel-level mosaic would require reading background pixels
  rect.set({
    fill: "rgba(40, 40, 40, 0.9)",
    stroke: "transparent",
  });
}

function onKeyDown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === "z") {
    e.preventDefault();
    undo();
  } else if (e.ctrlKey && e.key === "y") {
    e.preventDefault();
    redo();
  } else if (e.key === "Escape") {
    getCurrentWindow().close();
  } else if (e.key === "Delete" || e.key === "Backspace") {
    if (fabricCanvas) {
      const active = fabricCanvas.getActiveObject();
      if (active && !((active as any).isEditing)) {
        fabricCanvas.remove(active);
        fabricCanvas.renderAll();
        saveHistory();
      }
    }
  }
}

// History management
function saveHistory() {
  if (!fabricCanvas) return;
  const json = JSON.stringify(fabricCanvas.toJSON());
  // Remove any future states after current index
  historyStack.value = historyStack.value.slice(0, historyIndex.value + 1);
  historyStack.value.push(json);
  historyIndex.value = historyStack.value.length - 1;

  // Limit history size
  if (historyStack.value.length > 50) {
    historyStack.value.shift();
    historyIndex.value--;
  }
}

function undo() {
  if (historyIndex.value <= 0 || !fabricCanvas) return;
  historyIndex.value--;
  loadHistory(historyIndex.value);
}

function redo() {
  if (historyIndex.value >= historyStack.value.length - 1 || !fabricCanvas) return;
  historyIndex.value++;
  loadHistory(historyIndex.value);
}

function loadHistory(index: number) {
  if (!fabricCanvas) return;
  const json = historyStack.value[index];
  fabricCanvas.loadFromJSON(json).then(() => {
    fabricCanvas?.renderAll();
  });
}

// Save/Copy/Pin actions
async function saveToFile() {
  if (!fabricCanvas) return;
  setStatus("正在保存...");
  try {
    const dataUrl = fabricCanvas.toDataURL({ format: "png", quality: 1 });
    const base64 = dataUrl.replace(/^data:image\/png;base64,/, "");
    await invoke("save_image_to_file", { imageBase64: base64 });
    setStatus("已保存");
  } catch (e) {
    setStatus("保存失败: " + e);
  }
}

async function copyToClipboard() {
  if (!fabricCanvas) return;
  setStatus("正在复制...");
  try {
    const dataUrl = fabricCanvas.toDataURL({ format: "png", quality: 1 });
    const base64 = dataUrl.replace(/^data:image\/png;base64,/, "");
    await invoke("copy_image_to_clipboard", { imageBase64: base64 });
    setStatus("已复制到剪贴板");
  } catch (e) {
    setStatus("复制失败: " + e);
  }
}

async function pinToDesktop() {
  if (!fabricCanvas) return;
  setStatus("贴图功能开发中...");
  // Future: create a pin window with the annotated image
}

function setStatus(msg: string) {
  statusMessage.value = msg;
  setTimeout(() => {
    statusMessage.value = "";
  }, 2000);
}
</script>

<style scoped>
.annotate-view {
  display: flex;
  flex-direction: column;
  width: 100vw;
  height: 100vh;
  background: #111827;
  overflow: hidden;
}

.status-bar {
  position: fixed;
  bottom: 16px;
  left: 50%;
  transform: translateX(-50%);
  background: #1f2937;
  color: #e5e7eb;
  padding: 6px 16px;
  border-radius: 6px;
  font-size: 12px;
  box-shadow: 0 2px 8px rgba(0, 0, 0, 0.3);
  z-index: 100;
  pointer-events: none;
}
</style>
