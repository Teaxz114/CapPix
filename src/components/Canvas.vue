<template>
  <div ref="canvasContainer" class="canvas-container">
    <canvas ref="canvasEl"></canvas>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted, nextTick, watch } from "vue";
import { Canvas, Rect, Ellipse, Line, IText, Path, PencilBrush, FabricImage } from "fabric";

const props = defineProps<{
  imageBase64: string;
}>();

const canvasEl = ref<HTMLCanvasElement | null>(null);
const canvasContainer = ref<HTMLDivElement | null>(null);
let fabricCanvas: Canvas | null = null;

// Expose the canvas instance for parent components
defineExpose({
  getCanvas: () => fabricCanvas,
});

onMounted(async () => {
  await nextTick();
  initCanvas();
});

onUnmounted(() => {
  if (fabricCanvas) {
    fabricCanvas.dispose();
    fabricCanvas = null;
  }
});

function initCanvas() {
  if (!canvasEl.value || !canvasContainer.value) return;

  const container = canvasContainer.value;
  const width = container.clientWidth;
  const height = container.clientHeight;

  fabricCanvas = new Canvas(canvasEl.value, {
    width,
    height,
    backgroundColor: "#1a1a2e",
    selection: true,
    preserveObjectStacking: true,
  });

  // Load the screenshot as background
  if (props.imageBase64) {
    loadImageAsBackground(props.imageBase64);
  }
}

function loadImageAsBackground(base64: string) {
  if (!fabricCanvas) return;

  const img = new Image();
  img.onload = () => {
    if (!fabricCanvas) return;

    const container = canvasContainer.value;
    if (!container) return;

    // Scale image to fit container while maintaining aspect ratio
    const canvasWidth = container.clientWidth;
    const canvasHeight = container.clientHeight;
    const scaleX = canvasWidth / img.width;
    const scaleY = canvasHeight / img.height;
    const scale = Math.min(scaleX, scaleY, 1); // Don't upscale

    const displayWidth = img.width * scale;
    const displayHeight = img.height * scale;

    fabricCanvas.setDimensions({
      width: canvasWidth,
      height: canvasHeight,
    });

    // Center the image
    const offsetX = (canvasWidth - displayWidth) / 2;
    const offsetY = (canvasHeight - displayHeight) / 2;

    // Fabric.js 6: use backgroundImage property instead of setBackgroundImage method
    const bgImgEl = new Image();
    bgImgEl.onload = () => {
      if (!fabricCanvas) return;
      const bgFabricImg = new FabricImage(bgImgEl, {
        scaleX: scale,
        scaleY: scale,
        left: offsetX,
        top: offsetY,
        originX: "left",
        originY: "top",
      });
      fabricCanvas.backgroundImage = bgFabricImg;
      fabricCanvas.renderAll();
    };
    bgImgEl.src = `data:image/png;base64,${base64}`;
  };
  img.src = `data:image/png;base64,${base64}`;
}

// Watch for image changes
watch(
  () => props.imageBase64,
  (newVal) => {
    if (newVal && fabricCanvas) {
      loadImageAsBackground(newVal);
    }
  }
);
</script>

<style scoped>
.canvas-container {
  width: 100%;
  height: 100%;
  overflow: hidden;
  background: #111827;
}

.canvas-container canvas {
  display: block;
}
</style>
