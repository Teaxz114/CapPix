<template>
  <div class="min-h-screen bg-gray-900 text-gray-100 p-6">
    <header class="mb-8">
      <h1 class="text-2xl font-bold text-white">CapPix</h1>
      <p class="text-gray-400 text-sm mt-1">免费开源截图/贴图/OCR/录屏工具</p>
    </header>

    <section class="mb-8">
      <h2 class="text-lg font-semibold text-gray-200 mb-4 border-b border-gray-700 pb-2">截图</h2>
      <div class="grid grid-cols-3 gap-3">
        <button v-for="item in captureTools" :key="item.id"
          class="bg-gray-800 hover:bg-gray-700 rounded-lg p-4 text-left transition-colors"
          @click="handleAction(item.id)">
          <div class="text-sm font-medium text-gray-100">{{ item.name }}</div>
          <div class="text-xs text-gray-500 mt-1">{{ item.shortcut }}</div>
        </button>
      </div>
    </section>

    <section class="mb-8">
      <h2 class="text-lg font-semibold text-gray-200 mb-4 border-b border-gray-700 pb-2">工具</h2>
      <div class="grid grid-cols-3 gap-3">
        <button v-for="item in utilTools" :key="item.id"
          class="bg-gray-800 hover:bg-gray-700 rounded-lg p-4 text-left transition-colors"
          @click="handleAction(item.id)">
          <div class="text-sm font-medium text-gray-100">{{ item.name }}</div>
          <div class="text-xs text-gray-500 mt-1">{{ item.shortcut || '点击使用' }}</div>
        </button>
      </div>
    </section>

    <section class="mb-8">
      <h2 class="text-lg font-semibold text-gray-200 mb-4 border-b border-gray-700 pb-2">快捷键</h2>
      <div class="space-y-2">
        <div v-for="hk in hotkeys" :key="hk.id"
          class="flex items-center justify-between bg-gray-800 rounded-lg px-4 py-3">
          <span class="text-sm text-gray-300">{{ hk.name }}</span>
          <kbd class="bg-gray-700 text-gray-200 px-2 py-1 rounded text-xs font-mono">{{ hk.shortcut }}</kbd>
        </div>
      </div>
    </section>

    <section class="mb-8">
      <h2 class="text-lg font-semibold text-gray-200 mb-4 border-b border-gray-700 pb-2">更多</h2>
      <div class="grid grid-cols-2 gap-3">
        <button class="bg-gray-800 hover:bg-gray-700 rounded-lg p-4 text-left transition-colors" @click="goHistory">
          <div class="text-sm font-medium text-gray-100">历史记录</div>
          <div class="text-xs text-gray-500 mt-1">查看截图历史</div>
        </button>
        <button class="bg-gray-800 hover:bg-gray-700 rounded-lg p-4 text-left transition-colors" @click="goSettings">
          <div class="text-sm font-medium text-gray-100">设置</div>
          <div class="text-xs text-gray-500 mt-1">快捷键/格式/主题</div>
        </button>
      </div>
    </section>

    <footer class="text-center text-gray-600 text-xs mt-12">
      CapPix v0.1.0 | MIT License
    </footer>
    <RecordingBar />
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import RecordingBar from "../components/RecordingBar.vue";

interface HotkeyInfo {
  id: string;
  name: string;
  shortcut: string;
}

const router = useRouter();
const hotkeys = ref<HotkeyInfo[]>([]);

const captureTools = [
  { id: "capture_region", name: "区域截图", shortcut: "Ctrl+Shift+A" },
  { id: "capture_fullscreen", name: "全屏截图", shortcut: "Ctrl+Shift+S" },
  { id: "capture_window", name: "窗口截图", shortcut: "Ctrl+Shift+Q" },
];

const utilTools = [
  { id: "pin_clipboard", name: "贴图", shortcut: "" },
  { id: "color_picker", name: "取色器", shortcut: "" },
  { id: "screen_record", name: "录屏", shortcut: "" },
];

onMounted(async () => {
  try {
    hotkeys.value = await invoke<HotkeyInfo[]>("get_hotkeys");
  } catch (e) {
    console.error("Failed to load hotkeys:", e);
    hotkeys.value = captureTools.map(t => ({ id: t.id, name: t.name, shortcut: t.shortcut }));
  }

  // Listen for tray-action events from the system tray menu
  try {
    const unlisten = await listen<string>("tray-action", (event) => {
      handleAction(event.payload);
    });
    onUnmounted(() => { unlisten(); });
  } catch (e) {
    console.error("Failed to listen for tray-action:", e);
  }
});

function handleAction(id: string) {
  if (id === "capture_region" || id === "capture_fullscreen" || id === "capture_window") {
    // Trigger capture directly via invoke + open overlay
    triggerCapture(id);
  } else if (id === "screen_record") {
    startRecording();
  } else if (id === "color_picker") {
    startColorPicker();
  } else if (id === "pin_clipboard") {
    pinFromClipboard();
  }
}

async function triggerCapture(id: string) {
  try {
    await invoke("trigger_capture", { mode: id });
  } catch (e) {
    console.error("Capture failed:", e);
  }
}

async function startRecording() {
  try {
    const path = await invoke<string>("start_recording", {
      outputPath: null,
      region: null,
      withAudio: false,
    });
    console.log("Recording started:", path);
  } catch (e) {
    const msg = String(e);
    if (msg.includes("ffmpeg") || msg.includes("Failed to start")) {
      alert("录屏需要 FFmpeg，请先安装并添加到 PATH。\n\n下载地址: https://ffmpeg.org/download.html\n\nWindows 推荐: https://www.gyan.dev/ffmpeg/builds/");
    } else {
      alert("录屏启动失败: " + msg);
    }
  }
}

async function startColorPicker() {
  // Enter screenshot overlay in color-picker mode
  try {
    await invoke("trigger_capture", { mode: "capture_region" });
    // Small delay to let the overlay window open, then switch to color picker mode
    setTimeout(async () => {
      try {
        const { emit } = await import("@tauri-apps/api/event");
        await emit("activate-color-picker");
      } catch (e) {
        console.error("Failed to activate color picker mode:", e);
      }
    }, 500);
  } catch (e) {
    console.error("Color picker failed:", e);
  }
}

async function pinFromClipboard() {
  try {
    // Read image from clipboard via Tauri clipboard plugin
    const { readImage } = await import("@tauri-apps/plugin-clipboard-manager");
    const clipboardImage = await readImage();
    const rgbaData = await clipboardImage.rgba();
    const { width, height } = await clipboardImage.size();

    // Convert RGBA to PNG via canvas, then to base64
    const canvas = document.createElement("canvas");
    canvas.width = width;
    canvas.height = height;
    const ctx = canvas.getContext("2d");
    if (!ctx) throw new Error("Canvas not available");
    const imgData = new ImageData(new Uint8ClampedArray(rgbaData), width, height);
    ctx.putImageData(imgData, 0, 0);
    const dataUrl = canvas.toDataURL("image/png");
    const base64 = dataUrl.replace(/^data:image\/png;base64,/, "");

    await invoke("create_pin_window", { imageBase64: base64 });
  } catch (e) {
    console.error("Pin from clipboard failed:", e);
    alert("无法从剪贴板读取图片。请先复制一张图片再试。");
  }
}

function goHistory() {
  router.push("/history");
}

function goSettings() {
  router.push("/settings");
}
</script>
