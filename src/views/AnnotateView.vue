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
      @ocr="performOcr"
    />
    <Canvas
      ref="canvasRef"
      :image-base64="imageBase64"
    />
    <OcrPanel
      :visible="showOcr"
      :result="ocrResult"
      :loading="ocrLoading"
      :error="ocrError"
      @close="showOcr = false"
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
import { Canvas as FabricCanvas, Rect, Ellipse, Line, IText, Path, PencilBrush, Group, Image as FabricImage } from "fabric";
import Toolbar from "../components/Toolbar.vue";
import OcrPanel from "../components/OcrPanel.vue";
import CanvasComponent from "../components/Canvas.vue";
import { useConfigStore } from "../stores/config";

interface OcrBlock {
  text: string;
  confidence: number;
  bbox: number[][];
}

interface OcrResult {
  text: string;
  blocks: OcrBlock[];
  elapsed?: number;
  error?: string;
}

const canvasRef = ref<InstanceType<typeof CanvasComponent> | null>(null);
const imageBase64 = ref("");
const currentTool = ref("rect");
const configStore = useConfigStore();
const currentColor = ref(configStore.config.defaultColor);
const currentStrokeWidth = ref(configStore.config.defaultStrokeWidth);
const statusMessage = ref("");
const historyStack = ref<string[]>([]);
const historyIndex = ref(-1);
const showOcr = ref(false);
const ocrResult = ref<OcrResult | null>(null);
const ocrLoading = ref(false);
const ocrError = ref<string | null>(null);

let fabricCanvas: FabricCanvas | null = null;
let isDrawing = false;
let startX = 0;
let startY = 0;
let activeObject: Rect | Ellipse | Line | IText | Path | null = null;
let unlisten: UnlistenFn | null = null;

onMounted(async () => {
  // Try to get image data from PendingAnnotateImage (set by Rust before window creation)
  try {
    const data = await invoke<string | null>("get_pending_annotate_image");
    if (data) {
      imageBase64.value = data;
    }
  } catch (_) {}

  // Also listen for annotate-image event as fallback (e.g. from history view)
  unlisten = await listen<string>("annotate-image", (event) => {
    imageBase64.value = event.payload;
    setTimeout(initCanvasTools, 300);
  });

  // Listen for direct image prop via URL hash params
  const hash = window.location.hash;
  const queryString = hash.includes("?") ? hash.split("?")[1] : "";
  const params = new URLSearchParams(queryString);
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
  } else if (tool === "highlighter") {
    fabricCanvas.isDrawingMode = true;
    fabricCanvas.freeDrawingBrush = new PencilBrush(fabricCanvas);
    // Highlighter: semi-transparent, wider stroke
    const color = currentColor.value;
    const r = parseInt(color.slice(1, 3), 16);
    const g = parseInt(color.slice(3, 5), 16);
    const b = parseInt(color.slice(5, 7), 16);
    fabricCanvas.freeDrawingBrush.color = `rgba(${r},${g},${b},0.35)`;
    fabricCanvas.freeDrawingBrush.width = Math.max(currentStrokeWidth.value * 4, 16);
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
    const text = new IText("文字", {
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

  if (currentTool.value === "number") {
    // Auto-increment number annotation
    const existingNumbers = fabricCanvas.getObjects().filter((obj: any) => obj._cappixNumber);
    const nextNum = existingNumbers.length + 1;
    const circle = new Ellipse({
      left: pointer.x - 16,
      top: pointer.y - 16,
      rx: 16,
      ry: 16,
      fill: currentColor.value,
      stroke: "transparent",
      selectable: true,
    });
    (circle as any)._cappixNumber = nextNum;
    fabricCanvas.add(circle);
    const numText = new IText(String(nextNum), {
      left: pointer.x - 5,
      top: pointer.y - 10,
      fontSize: 16,
      fill: "#ffffff",
      fontFamily: "Arial, sans-serif",
      fontWeight: "bold",
      textAlign: "center",
      selectable: false,
      evented: false,
    });
    (numText as any)._cappixNumberLabel = true;
    fabricCanvas.add(numText);
    fabricCanvas.renderAll();
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

  if (currentTool.value === "blur") {
    // Blur: draw a semi-transparent rect as placeholder
    const rect = new Rect({
      left: pointer.x,
      top: pointer.y,
      width: 0,
      height: 0,
      fill: "rgba(128,128,128,0.5)",
      stroke: "rgba(100,149,237,0.8)",
      strokeWidth: 1,
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
    // Arrow: use a Group with Line + triangle arrowhead, created on mouseUp
    const arrow = new Line([pointer.x, pointer.y, pointer.x, pointer.y], {
      stroke: currentColor.value,
      strokeWidth: currentStrokeWidth.value,
      selectable: false,
      evented: false,
    });
    (arrow as any)._cappixArrow = true;
    activeObject = arrow;
    fabricCanvas.add(arrow);
  }
}

function onMouseMove(opt: any) {
  if (!fabricCanvas || !isDrawing || !activeObject) return;
  const pointer = fabricCanvas.getScenePoint(opt.e);

  if (currentTool.value === "rect" || currentTool.value === "mosaic" || currentTool.value === "blur") {
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

  // Apply Gaussian blur if needed
  if (currentTool.value === "blur" && activeObject) {
    applyBlur(activeObject as Rect);
  }

  // Convert arrow Line to Line+Triangle Group
  if (currentTool.value === "arrow" && activeObject && (activeObject as any)._cappixArrow) {
    const line = activeObject as Line;
    const x1 = (line.x1 ?? 0), y1 = (line.y1 ?? 0);
    const x2 = (line.x2 ?? 0), y2 = (line.y2 ?? 0);
    const dx = x2 - x1, dy = y2 - y1;
    const len = Math.sqrt(dx * dx + dy * dy);
    fabricCanvas.remove(line);
    if (len > 10) {
      // Arrow: draw arrowhead triangle at end point, and replace Line with Group
      const headLen = Math.min(20, len * 0.3);
      const angle = Math.atan2(dy, dx);
      const tipX = x2, tipY = y2;
      const leftX = tipX - headLen * Math.cos(angle - Math.PI / 6);
      const leftY = tipY - headLen * Math.sin(angle - Math.PI / 6);
      const rightX = tipX - headLen * Math.cos(angle + Math.PI / 6);
      const rightY = tipY - headLen * Math.sin(angle + Math.PI / 6);
      const pathStr = `M ${tipX} ${tipY} L ${leftX} ${leftY} L ${rightX} ${rightY} Z`;
      const arrowHead = new Path(pathStr, {
        fill: currentColor.value,
        stroke: currentColor.value,
        strokeWidth: 1,
        selectable: false,
        evented: false,
      });
      const arrowLine = new Line([x1, y1, x2, y2], {
        stroke: currentColor.value,
        strokeWidth: currentStrokeWidth.value,
        selectable: false,
        evented: false,
      });
      const group = new Group([arrowLine, arrowHead], {
        selectable: true,
        evented: true,
      });
      fabricCanvas.add(group);
    }
  }

  activeObject = null;
  fabricCanvas.renderAll();
  saveHistory();
}

function applyMosaic(rect: Rect) {
  if (!fabricCanvas) return;
  const blockSize = configStore.config.mosaicBlockSize;
  const left = rect.left || 0;
  const top = rect.top || 0;
  const width = rect.width || 0;
  const height = rect.height || 0;

  if (width < 2 || height < 2) {
    fabricCanvas.remove(rect);
    return;
  }

  // Remove the placeholder rect
  fabricCanvas.remove(rect);

  // Read background pixels by rendering the canvas (including background image) to a temp canvas
  const tempCanvas = fabricCanvas.toCanvasElement();
  const tempCtx = tempCanvas.getContext("2d");
  if (!tempCtx) return;

  // Read pixel data from the mosaic region
  try {
    const imgData = tempCtx.getImageData(left, top, width, height);
    const pixels = imgData.data;

    // Pixelate: for each block, average the colors
    for (let by = 0; by < height; by += blockSize) {
      for (let bx = 0; bx < width; bx += blockSize) {
        let r = 0, g = 0, b = 0, count = 0;
        const bw = Math.min(blockSize, width - bx);
        const bh = Math.min(blockSize, height - by);

        for (let y = by; y < by + bh; y++) {
          for (let x = bx; x < bx + bw; x++) {
            const idx = (y * width + x) * 4;
            r += pixels[idx];
            g += pixels[idx + 1];
            b += pixels[idx + 2];
            count++;
          }
        }

        r = Math.round(r / count);
        g = Math.round(g / count);
        b = Math.round(b / count);

        // Fill block with averaged color
        const mosaicRect = new Rect({
          left: left + bx,
          top: top + by,
          width: bw,
          height: bh,
          fill: `rgb(${r},${g},${b})`,
          stroke: "transparent",
          selectable: false,
          evented: false,
        });
        (mosaicRect as any)._cappixMosaic = true;
        fabricCanvas.add(mosaicRect);
      }
    }
  } catch (e) {
    // Cross-origin or other error: fall back to solid gray
    const fallbackRect = new Rect({
      left,
      top,
      width,
      height,
      fill: "rgba(128,128,128,0.9)",
      stroke: "transparent",
      selectable: true,
    });
    (fallbackRect as any)._cappixMosaic = true;
    fabricCanvas.add(fallbackRect);
  }

  fabricCanvas.renderAll();
}

function applyBlur(rect: Rect) {
  if (!fabricCanvas) return;
  const blurRadius = configStore.config.blurRadius || 12;
  const left = rect.left || 0;
  const top = rect.top || 0;
  const width = rect.width || 0;
  const height = rect.height || 0;

  if (width < 2 || height < 2) {
    fabricCanvas.remove(rect);
    return;
  }

  // Remove the placeholder rect
  fabricCanvas.remove(rect);

  // Render background to temp canvas
  const tempCanvas = fabricCanvas.toCanvasElement();
  const tempCtx = tempCanvas.getContext("2d");
  if (!tempCtx) return;

  try {
    // Extract the region, apply CSS blur, then paste back as image
    const blurCanvas = document.createElement("canvas");
    blurCanvas.width = width;
    blurCanvas.height = height;
    const blurCtx = blurCanvas.getContext("2d");
    if (!blurCtx) return;

    // Draw the region from temp canvas
    blurCtx.drawImage(tempCanvas, left, top, width, height, 0, 0, width, height);

    // Apply blur via CSS filter
    blurCtx.filter = `blur(${blurRadius}px)`;
    blurCtx.drawImage(blurCanvas, 0, 0);
    blurCtx.filter = "none";

    // Create Fabric image from blurred canvas
    const dataUrl = blurCanvas.toDataURL("image/png");
    FabricImage.fromURL(dataUrl).then((img) => {
      img.set({
        left,
        top,
        selectable: true,
        evented: true,
      });
      (img as any)._cappixBlur = true;
      fabricCanvas!.add(img);
      fabricCanvas!.renderAll();
    });
  } catch (e) {
    // Cross-origin or other error: fall back to semi-transparent overlay
    const fallbackRect = new Rect({
      left,
      top,
      width,
      height,
      fill: "rgba(200,200,200,0.7)",
      stroke: "transparent",
      selectable: true,
    });
    (fallbackRect as any)._cappixBlur = true;
    fabricCanvas.add(fallbackRect);
  }
}

function onKeyDown(e: KeyboardEvent) {
  if (e.ctrlKey && e.key === "z") {
    e.preventDefault();
    undo();
  } else if (e.ctrlKey && e.key === "y") {
    e.preventDefault();
    redo();
  } else if (e.ctrlKey && e.key === "s") {
    e.preventDefault();
    saveToFile();
  } else if (e.ctrlKey && e.key === "c") {
    e.preventDefault();
    copyToClipboard();
  } else if (e.key === "Escape") {
    // Don't close the main window — restore it and navigate back to home
    restoreMainWindow();
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

// Export canvas including background image
function exportCanvasBase64(): string {
  if (!fabricCanvas) return "";
  // toCanvasElement renders everything including background
  const tempCanvas = fabricCanvas.toCanvasElement();
  const dataUrl = tempCanvas.toDataURL("image/png");
  return dataUrl.replace(/^data:image\/png;base64,/, "");
}

// Save/Copy/Pin actions
async function saveToFile() {
  if (!fabricCanvas) return;
  setStatus("正在保存...");
  try {
    // Lower window so save dialog is visible on top
    const win = getCurrentWindow();
    await win.setAlwaysOnTop(false);
    await win.setDecorations(true);
    const base64 = exportCanvasBase64();
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
    const base64 = exportCanvasBase64();
    await invoke("copy_image_to_clipboard", { imageBase64: base64 });
    setStatus("已复制到剪贴板");
  } catch (e) {
    setStatus("复制失败: " + e);
  }
}

async function pinToDesktop() {
  if (!fabricCanvas) return;
  setStatus("正在贴图...");
  try {
    const base64 = exportCanvasBase64();
    await invoke("create_pin_window", { imageBase64: base64 });
    setStatus("已贴图到桌面");
  } catch (e) {
    setStatus("贴图失败: " + e);
  }
}

async function performOcr() {
  if (!fabricCanvas) return;
  showOcr.value = true;
  ocrLoading.value = true;
  ocrError.value = null;
  try {
    const base64 = exportCanvasBase64();
    const result = await invoke<OcrResult>("ocr_image", { imageBase64: base64 });
    if (result.error) {
      ocrError.value = result.error;
    } else {
      ocrResult.value = result;
    }
  } catch (e) {
    ocrError.value = String(e);
  } finally {
    ocrLoading.value = false;
  }
}

function setStatus(msg: string) {
  statusMessage.value = msg;
  setTimeout(() => {
    statusMessage.value = "";
  }, 2000);
}

async function restoreMainWindow() {
  try {
    const win = getCurrentWindow();
    await win.setDecorations(true);
    await win.setAlwaysOnTop(false);
    await win.setResizable(true);
    const { LogicalSize } = await import("@tauri-apps/api/dpi");
    await win.setSize(new LogicalSize(800, 600));
    await win.center();
    window.location.hash = '/';
  } catch (e) {
    console.error("Failed to restore window:", e);
  }
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
  position: relative;
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
