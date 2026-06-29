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

    <!-- Record Dialog -->
    <div v-if="showRecordDialog" class="dialog-overlay" @click.self="showRecordDialog = false">
      <div class="dialog-box">
        <h3>{{ recordMode === 'gif' ? 'GIF 录制' : '录屏设置' }}</h3>
        <div class="dialog-field">
          <label><input type="checkbox" v-model="recordRegion" /> 选择区域（默认全屏）</label>
        </div>
        <div v-if="recordMode === 'video'" class="dialog-field">
          <label><input type="checkbox" v-model="recordAudio" /> 录制系统声音</label>
        </div>
        <div v-if="recordMode === 'gif'" class="dialog-field">
          <label>录制时长: {{ gifDuration }}秒</label>
          <input type="range" v-model.number="gifDuration" min="1" max="30" step="1" />
        </div>
        <div class="dialog-actions">
          <button @click="showRecordDialog = false" class="btn-cancel">取消</button>
          <button @click="recordMode === 'gif' ? startGifRecording() : startRecording()" class="btn-primary">
            开始录制
          </button>
        </div>
      </div>
    </div>
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
  { id: "gif_record", name: "GIF 录制", shortcut: "" },
];

const showRecordDialog = ref(false);
const recordMode = ref<"video" | "gif">("video");
const recordRegion = ref(false);
const recordAudio = ref(false);
const gifDuration = ref(5);

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
    recordMode.value = "video";
    showRecordDialog.value = true;
  } else if (id === "gif_record") {
    recordMode.value = "gif";
    showRecordDialog.value = true;
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
  showRecordDialog.value = false;
  try {
    const region = recordRegion.value ? await selectRegion() : null;
    const path = await invoke<string>("start_recording", {
      outputPath: null,
      region,
      withAudio: recordAudio.value,
    });
    console.log("Recording started:", path);
  } catch (e) {
    const msg = String(e);
    if (msg.includes("FFmpeg") || msg.includes("ffmpeg") || msg.includes("Failed to start")) {
      alert("录屏需要 FFmpeg，请先安装并添加到 PATH。\n\n下载地址: https://ffmpeg.org/download.html\n\nWindows 推荐: https://www.gyan.dev/ffmpeg/builds/");
    } else if (!msg.includes("cancel")) {
      alert("录屏启动失败: " + msg);
    }
  }
}

async function startGifRecording() {
  showRecordDialog.value = false;
  try {
    const region = recordRegion.value ? await selectRegion() : null;
    const path = await invoke<string>("record_to_gif", {
      outputPath: null,
      region,
      durationSecs: gifDuration.value,
    });
    alert(`GIF 已保存: ${path}`);
  } catch (e) {
    const msg = String(e);
    if (msg.includes("FFmpeg") || msg.includes("ffmpeg")) {
      alert("GIF 录制需要 FFmpeg，请先安装并添加到 PATH。");
    } else if (!msg.includes("cancel")) {
      alert("GIF 录制失败: " + msg);
    }
  }
}

async function selectRegion(): Promise<[number, number, number, number] | null> {
  // Use screenshot overlay to let user select a region
  // Returns [x, y, w, h] or null if cancelled
  try {
    await invoke("trigger_capture", { mode: "capture_region" });
    const { listen } = await import("@tauri-apps/api/event");
    return new Promise((resolve) => {
      const timeout = setTimeout(() => resolve(null), 60000);
      listen<{ x: number; y: number; w: number; h: number }>("region-selected", (event) => {
        clearTimeout(timeout);
        resolve([event.payload.x, event.payload.y, event.payload.w, event.payload.h]);
      });
    });
  } catch {
    return null;
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

<style scoped>
.dialog-overlay {
  position: fixed; inset: 0; background: rgba(0,0,0,0.6);
  display: flex; align-items: center; justify-content: center; z-index: 9999;
}
.dialog-box {
  background: #1f2937; border: 1px solid #374151; border-radius: 12px;
  padding: 24px; min-width: 340px; max-width: 420px;
}
.dialog-box h3 { color: #e5e7eb; font-size: 16px; margin: 0 0 16px; }
.dialog-field { margin-bottom: 12px; color: #d1d5db; font-size: 14px; }
.dialog-field label { display: flex; align-items: center; gap: 8px; cursor: pointer; }
.dialog-field input[type="range"] { width: 100%; margin-top: 4px; }
.dialog-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
.btn-cancel { background: #374151; color: #d1d5db; border: none; padding: 8px 16px; border-radius: 6px; cursor: pointer; }
.btn-primary { background: #2563eb; color: white; border: none; padding: 8px 16px; border-radius: 6px; cursor: pointer; }
.btn-primary:hover { background: #1d4ed8; }
</style>
