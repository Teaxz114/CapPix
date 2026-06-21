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
import { ref, onMounted } from "vue";
import { useRouter } from "vue-router";
import { invoke } from "@tauri-apps/api/core";
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
});

function handleAction(id: string) {
  if (id === "capture_region" || id === "capture_fullscreen" || id === "capture_window") {
    // Emit hotkey event to trigger capture
    invoke("emit_hotkey", { action: id }).catch(() => {});
  } else if (id === "screen_record") {
    startRecording();
  } else if (id === "color_picker") {
    startColorPicker();
  } else if (id === "pin_clipboard") {
    pinFromClipboard();
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
    console.error("Failed to start recording:", e);
  }
}

async function startColorPicker() {
  try {
    // Get color at current cursor position
    const result = await invoke<{ hex: string; rgb: string; hsl: string; r: number; g: number; b: number }>("pick_color_at_point", {
      x: 0,
      y: 0,
    });
    console.log("Color:", result);
  } catch (e) {
    console.error("Color picker failed:", e);
  }
}

async function pinFromClipboard() {
  try {
    // Read clipboard image and pin it
    // For now, just show a message
    console.log("Pin from clipboard");
  } catch (e) {
    console.error("Pin failed:", e);
  }
}

function goHistory() {
  router.push("/history");
}

function goSettings() {
  router.push("/settings");
}
</script>
