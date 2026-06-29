<template>
  <div class="settings-view">
    <h2>设置</h2>
    <div class="settings-sections">
      <!-- Save settings -->
      <section class="settings-section">
        <h3>保存</h3>
        <div class="setting-row">
          <label>默认保存格式</label>
          <select v-model="config.saveFormat">
            <option value="png">PNG</option>
            <option value="jpg">JPG</option>
            <option value="bmp">BMP</option>
          </select>
        </div>
        <div class="setting-row">
          <label>保存质量</label>
          <input type="range" v-model.number="config.saveQuality" min="1" max="100" />
          <span class="setting-value">{{ config.saveQuality }}%</span>
        </div>
        <div class="setting-row">
          <label>截图后自动复制到剪贴板</label>
          <input type="checkbox" v-model="config.autoCopyToClipboard" />
        </div>
        <div class="setting-row vertical">
          <label>保存目录 <span class="hint">（留空则默认为 Pictures/CapPix）</span></label>
          <div class="path-input-row">
            <input
              type="text"
              v-model="config.saveDirectory"
              placeholder="Pictures/CapPix"
              class="path-input"
            />
            <button class="btn-browse" @click="browseSaveDirectory" title="浏览">📁</button>
          </div>
        </div>
        <div class="setting-row vertical">
          <label>文件名模式 <span class="hint">（可用: {date} {time} {seq}）</span></label>
          <input
            type="text"
            v-model="config.filenamePattern"
            placeholder="CapPix_{date}_{time}"
            class="pattern-input"
          />
          <div class="pattern-preview">
            预览: {{ previewFilename }}
          </div>
        </div>
      </section>

      <!-- Annotation settings -->
      <section class="settings-section">
        <h3>标注</h3>
        <div class="setting-row">
          <label>默认颜色</label>
          <input type="color" v-model="config.defaultColor" />
        </div>
        <div class="setting-row">
          <label>默认线宽</label>
          <input type="range" v-model.number="config.defaultStrokeWidth" min="1" max="20" />
          <span class="setting-value">{{ config.defaultStrokeWidth }}px</span>
        </div>
        <div class="setting-row">
          <label>默认字体</label>
          <select v-model="config.defaultFontFamily">
            <option value="Microsoft YaHei">微软雅黑</option>
            <option value="SimSun">宋体</option>
            <option value="Arial">Arial</option>
            <option value="Consolas">Consolas</option>
          </select>
        </div>
        <div class="setting-row">
          <label>字体大小</label>
          <input type="number" v-model.number="config.defaultFontSize" min="10" max="72" />
        </div>
      </section>

      <!-- OCR settings -->
      <section class="settings-section">
        <h3>OCR</h3>
        <div class="setting-row">
          <label>识别语言</label>
          <select v-model="config.ocrLanguage">
            <option value="ch_en">中英混合</option>
            <option value="ch">中文</option>
            <option value="en">英文</option>
          </select>
        </div>
        <div class="setting-row">
          <label>马赛克块大小</label>
          <input type="range" v-model.number="config.mosaicBlockSize" min="4" max="30" />
          <span class="setting-value">{{ config.mosaicBlockSize }}px</span>
        </div>
        <div class="setting-row">
          <label>显示放大镜</label>
          <input type="checkbox" v-model="config.showMagnifier" />
        </div>
      </section>

      <!-- Pin settings -->
      <section class="settings-section">
        <h3>贴图</h3>
        <div class="setting-row">
          <label>贴图透明度</label>
          <input type="range" v-model.number="config.pinOpacity" min="10" max="100" step="5" />
          <span class="setting-value">{{ config.pinOpacity }}%</span>
        </div>
      </section>

      <!-- General settings -->
      <section class="settings-section">
        <h3>通用</h3>
        <div class="setting-row">
          <label>主题</label>
          <select v-model="config.theme">
            <option value="dark">深色</option>
            <option value="light">浅色</option>
          </select>
        </div>
      </section>

      <!-- Hotkey display -->
      <section class="settings-section">
        <h3>快捷键</h3>
        <div class="setting-row" v-for="hk in hotkeys" :key="hk.key">
          <label>{{ hk.label }}</label>
          <input
            class="hotkey-input"
            :value="config[hk.key as keyof typeof config]"
            @keydown="onHotkeyKeydown($event, hk.key)"
            readonly
            placeholder="点击后按键设置"
          />
          <button class="btn-reset-hotkey" @click="resetHotkey(hk.key)" title="恢复默认">↺</button>
        </div>
        <p class="hotkey-hint">点击输入框，按下新快捷键即可修改（立即生效）</p>

        <div class="setting-row" style="margin-top: 12px;">
          <label>游戏模式</label>
          <button
            :class="['btn-toggle', { active: gameMode }]"
            @click="toggleGameMode"
          >
            {{ gameMode ? '已开启' : '已关闭' }}
          </button>
          <span class="setting-hint">开启后禁用所有快捷键，避免与游戏按键冲突</span>
        </div>
      </section>

      <!-- Actions -->
      <section class="settings-section">
        <h3>数据</h3>
        <div class="setting-row">
          <button class="btn-danger" @click="resetConfig">恢复默认设置</button>
        </div>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, ref, onMounted } from "vue";
import { useConfigStore } from "../stores/config";
import { storeToRefs } from "pinia";
import { invoke } from "@tauri-apps/api/core";

const configStore = useConfigStore();
const { config } = storeToRefs(configStore);

const gameMode = ref(false);

onMounted(async () => {
  try {
    gameMode.value = await invoke<boolean>("get_game_mode");
  } catch {}
});

async function toggleGameMode() {
  try {
    const newState = !gameMode.value;
    gameMode.value = await invoke<boolean>("toggle_game_mode", { enabled: newState });
  } catch {}
}

const hotkeys = [
  { key: "hotkeyCaptureRegion", label: "区域截图" },
  { key: "hotkeyCaptureFullscreen", label: "全屏截图" },
  { key: "hotkeyCaptureWindow", label: "窗口截图" },
];

const defaultHotkeys: Record<string, string> = {
  hotkeyCaptureRegion: "Ctrl+Shift+A",
  hotkeyCaptureFullscreen: "Ctrl+Shift+S",
  hotkeyCaptureWindow: "Ctrl+Shift+Q",
};

// Preview the filename pattern with current date/time
const previewFilename = computed(() => {
  const now = new Date();
  const dateStr = `${now.getFullYear()}-${String(now.getMonth() + 1).padStart(2, "0")}-${String(now.getDate()).padStart(2, "0")}`;
  const timeStr = `${String(now.getHours()).padStart(2, "0")}-${String(now.getMinutes()).padStart(2, "0")}-${String(now.getSeconds()).padStart(2, "0")}`;
  const pattern = config.value.filenamePattern || "CapPix_{date}_{time}";
  const ext = config.value.saveFormat || "png";
  const filename = pattern
    .replace("{date}", dateStr)
    .replace("{time}", timeStr)
    .replace("{seq}", "0001");
  return `${filename}.${ext}`;
});

// Browse for save directory using Tauri dialog
async function browseSaveDirectory() {
  try {
    const { open } = await import("@tauri-apps/plugin-dialog");
    const selected = await open({
      directory: true,
      multiple: false,
      title: "选择保存目录",
    });
    if (selected) {
      config.value.saveDirectory = selected;
    }
  } catch (e) {
    console.error("Failed to open directory picker:", e);
  }
}

function onHotkeyKeydown(e: KeyboardEvent, key: string) {
  e.preventDefault();
  e.stopPropagation();

  // Build shortcut string from key event
  const parts: string[] = [];
  if (e.ctrlKey) parts.push("Ctrl");
  if (e.altKey) parts.push("Alt");
  if (e.shiftKey) parts.push("Shift");

  // Ignore standalone modifier presses
  const modKeys = ["Control", "Alt", "Shift", "Meta"];
  if (modKeys.includes(e.key)) return;

  // Map key name
  let keyName = e.key;
  if (keyName === " ") keyName = "Space";
  if (keyName.length === 1) keyName = keyName.toUpperCase();
  parts.push(keyName);

  const shortcut = parts.join("+");

  // Update frontend config
  (config.value as any)[key] = shortcut;

  // Map frontend key to Rust hotkey id
  const keyToId: Record<string, string> = {
    hotkeyCaptureRegion: "capture_region",
    hotkeyCaptureFullscreen: "capture_fullscreen",
    hotkeyCaptureWindow: "capture_window",
  };
  const id = keyToId[key];
  if (id) {
    invoke("set_hotkey", { id, shortcut }).catch((err) => {
      console.error("Failed to register hotkey:", err);
      // Revert frontend config on failure
      (config.value as any)[key] = defaultHotkeys[key];
    });
  }
}

function resetHotkey(key: string) {
  (config.value as any)[key] = defaultHotkeys[key];

  const keyToId: Record<string, string> = {
    hotkeyCaptureRegion: "capture_region",
    hotkeyCaptureFullscreen: "capture_fullscreen",
    hotkeyCaptureWindow: "capture_window",
  };
  const id = keyToId[key];
  if (id) {
    invoke("set_hotkey", { id, shortcut: defaultHotkeys[key] }).catch((err) => {
      console.error("Failed to reset hotkey:", err);
    });
  }
}

function resetConfig() {
  if (confirm("确定恢复默认设置？")) {
    configStore.resetConfig();
  }
}
</script>

<style scoped>
.settings-view {
  padding: 20px;
  max-width: 600px;
  margin: 0 auto;
  color: #e5e7eb;
}
.settings-view h2 { font-size: 18px; margin-bottom: 16px; }
.settings-sections { display: flex; flex-direction: column; gap: 16px; }
.settings-section {
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  padding: 16px;
}
.settings-section h3 {
  font-size: 14px;
  color: #9ca3af;
  margin: 0 0 12px 0;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}
.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 6px 0;
}
.setting-row label { font-size: 13px; color: #d1d5db; }
.setting-row select, .setting-row input[type="number"] {
  background: #374151;
  color: #e5e7eb;
  border: 1px solid #4b5563;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 13px;
}
.setting-row input[type="range"] {
  width: 120px;
  accent-color: #3b82f6;
}
.setting-row input[type="checkbox"] {
  accent-color: #3b82f6;
  width: 16px;
  height: 16px;
}
.setting-row input[type="color"] {
  width: 32px;
  height: 24px;
  border: 1px solid #4b5563;
  border-radius: 4px;
  cursor: pointer;
  background: transparent;
}
.setting-value { color: #6b7280; font-size: 12px; min-width: 36px; text-align: right; }
kbd {
  background: #374151;
  border: 1px solid #4b5563;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
  color: #d1d5db;
}
.btn-danger {
  background: #7f1d1d;
  color: #e5e7eb;
  border: none;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
}
.btn-danger:hover { background: #991b1b; }

.btn-toggle {
  padding: 4px 12px;
  border-radius: 4px;
  border: 1px solid #374151;
  background: transparent;
  color: #9ca3af;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}
.btn-toggle.active {
  background: rgba(59, 130, 246, 0.15);
  border-color: #3b82f6;
  color: #3b82f6;
}
.btn-toggle:hover { background: #374151; }

.setting-hint {
  font-size: 11px;
  color: #6b7280;
  margin-left: 8px;
}
.setting-row.vertical {
  flex-direction: column;
  align-items: flex-start;
  gap: 6px;
}
.setting-row.vertical label {
  width: 100%;
}
.hint {
  color: #6b7280;
  font-size: 11px;
  font-weight: normal;
}
.path-input-row {
  display: flex;
  width: 100%;
  gap: 6px;
}
.path-input, .pattern-input {
  background: #374151;
  color: #e5e7eb;
  border: 1px solid #4b5563;
  padding: 6px 10px;
  border-radius: 4px;
  font-size: 13px;
  width: 100%;
  box-sizing: border-box;
}
.path-input:focus, .pattern-input:focus {
  border-color: #3b82f6;
  outline: none;
}
.btn-browse {
  background: #374151;
  border: 1px solid #4b5563;
  color: #e5e7eb;
  padding: 4px 10px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 16px;
  line-height: 1;
  flex-shrink: 0;
}
.btn-browse:hover {
  border-color: #3b82f6;
  background: #4b5563;
}
.pattern-preview {
  color: #6b7280;
  font-size: 11px;
  font-family: monospace;
  background: #111827;
  padding: 4px 8px;
  border-radius: 4px;
  width: 100%;
  box-sizing: border-box;
}
.hotkey-input {
  background: #374151;
  color: #e5e7eb;
  border: 1px solid #4b5563;
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 12px;
  text-align: center;
  cursor: pointer;
  min-width: 120px;
}
.hotkey-input:focus {
  border-color: #3b82f6;
  outline: none;
}
.btn-reset-hotkey {
  background: none;
  border: 1px solid #4b5563;
  color: #9ca3af;
  padding: 2px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 14px;
}
.btn-reset-hotkey:hover { color: #e5e7eb; border-color: #9ca3af; }
.hotkey-hint {
  color: #6b7280;
  font-size: 11px;
  margin: 4px 0 0;
}
</style>
