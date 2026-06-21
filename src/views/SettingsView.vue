<template>
  <div class="settings-view">
    <h2>设置</h2>

    <div class="settings-section">
      <h3>快捷键</h3>
      <div class="setting-row">
        <label>区域截图</label>
        <span class="hotkey-display">{{ config.hotkeyCaptureRegion }}</span>
      </div>
      <div class="setting-row">
        <label>全屏截图</label>
        <span class="hotkey-display">{{ config.hotkeyCaptureFullscreen }}</span>
      </div>
      <div class="setting-row">
        <label>窗口截图</label>
        <span class="hotkey-display">{{ config.hotkeyCaptureWindow }}</span>
      </div>
    </div>

    <div class="settings-section">
      <h3>标注</h3>
      <div class="setting-row">
        <label>默认颜色</label>
        <input type="color" v-model="config.defaultColor" />
      </div>
      <div class="setting-row">
        <label>默认线条粗细</label>
        <input type="range" min="1" max="20" v-model.number="config.defaultStrokeWidth" />
        <span class="range-val">{{ config.defaultStrokeWidth }}px</span>
      </div>
      <div class="setting-row">
        <label>马赛克块大小</label>
        <input type="range" min="4" max="30" v-model.number="config.mosaicBlockSize" />
        <span class="range-val">{{ config.mosaicBlockSize }}px</span>
      </div>
      <div class="setting-row">
        <label>字体</label>
        <select v-model="config.defaultFontFamily" class="setting-select">
          <option value="Microsoft YaHei">微软雅黑</option>
          <option value="SimHei">黑体</option>
          <option value="SimSun">宋体</option>
          <option value="Arial">Arial</option>
          <option value="Consolas">Consolas</option>
        </select>
      </div>
    </div>

    <div class="settings-section">
      <h3>保存</h3>
      <div class="setting-row">
        <label>保存格式</label>
        <select v-model="config.saveFormat" class="setting-select">
          <option value="png">PNG</option>
          <option value="jpg">JPG</option>
          <option value="bmp">BMP</option>
        </select>
      </div>
      <div class="setting-row">
        <label>保存质量</label>
        <input type="range" min="10" max="100" step="10" v-model.number="config.saveQuality" />
        <span class="range-val">{{ config.saveQuality }}%</span>
      </div>
      <div class="setting-row">
        <label>截图后自动复制到剪贴板</label>
        <input type="checkbox" v-model="config.autoCopyToClipboard" />
      </div>
    </div>

    <div class="settings-section">
      <h3>界面</h3>
      <div class="setting-row">
        <label>贴图透明度</label>
        <input type="range" min="10" max="100" step="5" :value="config.pinOpacity * 100" @input="config.pinOpacity = $event.target.value / 100" />
        <span class="range-val">{{ Math.round(config.pinOpacity * 100) }}%</span>
      </div>
      <div class="setting-row">
        <label>显示放大镜</label>
        <input type="checkbox" v-model="config.showMagnifier" />
      </div>
    </div>

    <div class="settings-actions">
      <button @click="resetSettings" class="btn-reset">恢复默认</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useConfigStore } from "../stores/config";
import { storeToRefs } from "pinia";

const configStore = useConfigStore();
const { config } = storeToRefs(configStore);

function resetSettings() {
  configStore.resetConfig();
}
</script>

<style scoped>
.settings-view {
  padding: 24px;
  max-width: 700px;
  margin: 0 auto;
  min-height: 100vh;
  background: #111827;
  color: #e5e7eb;
}
.settings-view h2 { font-size: 20px; font-weight: 600; margin-bottom: 20px; }

.settings-section {
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  padding: 16px 20px;
  margin-bottom: 16px;
}
.settings-section h3 { font-size: 14px; font-weight: 600; margin-bottom: 12px; color: #9ca3af; }

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 0;
  border-bottom: 1px solid #1a1a2e;
}
.setting-row:last-child { border-bottom: none; }
.setting-row label { font-size: 13px; color: #d1d5db; }

.hotkey-display {
  background: #374151;
  padding: 3px 10px;
  border-radius: 4px;
  font-size: 12px;
  color: #9ca3af;
}

input[type="color"] {
  width: 32px;
  height: 24px;
  border: none;
  cursor: pointer;
  background: none;
}

input[type="range"] {
  width: 120px;
  accent-color: #3b82f6;
}

.range-val {
  font-size: 12px;
  color: #6b7280;
  min-width: 40px;
  text-align: right;
}

input[type="checkbox"] {
  accent-color: #3b82f6;
  width: 16px;
  height: 16px;
}

.setting-select {
  padding: 4px 8px;
  background: #374151;
  border: 1px solid #4b5563;
  border-radius: 4px;
  color: #e5e7eb;
  font-size: 13px;
}

.settings-actions {
  display: flex;
  justify-content: flex-end;
  margin-top: 16px;
}

.btn-reset {
  padding: 8px 16px;
  background: #374151;
  color: #d1d5db;
  border: none;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
}
.btn-reset:hover { background: #4b5563; }
</style>
