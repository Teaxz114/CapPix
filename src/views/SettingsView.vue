<template>
  <div class="settings-view">
    <h2>设置</h2>

    <div class="settings-section">
      <h3>截图</h3>
      <div class="setting-row">
        <label>截图后自动复制到剪贴板</label>
        <input type="checkbox" v-model="config.autoCopyToClipboard" />
      </div>
      <div class="setting-row">
        <label>显示放大镜</label>
        <input type="checkbox" v-model="config.showMagnifier" />
      </div>
    </div>

    <div class="settings-section">
      <h3>保存</h3>
      <div class="setting-row">
        <label>默认保存格式</label>
        <select v-model="config.saveFormat">
          <option value="png">PNG (无损)</option>
          <option value="jpg">JPG (压缩)</option>
          <option value="bmp">BMP (位图)</option>
        </select>
      </div>
      <div class="setting-row">
        <label>JPG 质量 (1-100)</label>
        <input type="number" v-model.number="config.saveQuality" min="1" max="100" />
      </div>
    </div>

    <div class="settings-section">
      <h3>标注</h3>
      <div class="setting-row">
        <label>默认颜色</label>
        <input type="color" v-model="config.defaultColor" />
      </div>
      <div class="setting-row">
        <label>默认线宽</label>
        <input type="range" v-model.number="config.defaultStrokeWidth" min="1" max="20" />
        <span class="range-value">{{ config.defaultStrokeWidth }}px</span>
      </div>
      <div class="setting-row">
        <label>默认字体</label>
        <select v-model="config.defaultFontFamily">
          <option value="Microsoft YaHei">微软雅黑</option>
          <option value="SimHei">黑体</option>
          <option value="SimSun">宋体</option>
          <option value="Arial">Arial</option>
        </select>
      </div>
      <div class="setting-row">
        <label>默认字号</label>
        <input type="number" v-model.number="config.defaultFontSize" min="10" max="72" />
      </div>
    </div>

    <div class="settings-section">
      <h3>贴图</h3>
      <div class="setting-row">
        <label>贴图透明度</label>
        <input type="range" v-model.number="config.pinOpacity" min="10" max="100" step="5" />
        <span class="range-value">{{ config.pinOpacity }}%</span>
      </div>
    </div>

    <div class="settings-section">
      <h3>快捷键</h3>
      <div class="setting-row">
        <label>区域截图</label>
        <kbd>{{ config.hotkeyCaptureRegion }}</kbd>
      </div>
      <div class="setting-row">
        <label>全屏截图</label>
        <kbd>{{ config.hotkeyCaptureFullscreen }}</kbd>
      </div>
      <div class="setting-row">
        <label>窗口截图</label>
        <kbd>{{ config.hotkeyCaptureWindow }}</kbd>
      </div>
    </div>

    <div class="settings-actions">
      <button @click="resetConfig" class="reset-btn">恢复默认</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useConfigStore } from "../stores/config";
import { storeToRefs } from "pinia";

const configStore = useConfigStore();
const { config } = storeToRefs(configStore);

function resetConfig() {
  configStore.resetConfig();
}
</script>

<style scoped>
.settings-view {
  padding: 24px;
  max-width: 700px;
  margin: 0 auto;
  color: #e5e7eb;
}
.settings-view h2 {
  font-size: 20px;
  font-weight: 600;
  margin-bottom: 24px;
}
.settings-section {
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  padding: 16px 20px;
  margin-bottom: 16px;
}
.settings-section h3 {
  font-size: 14px;
  font-weight: 600;
  color: #9ca3af;
  text-transform: uppercase;
  letter-spacing: 0.05em;
  margin: 0 0 12px 0;
}
.setting-row {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 0;
  border-bottom: 1px solid #374151;
}
.setting-row:last-child { border-bottom: none; }
.setting-row label {
  font-size: 13px;
  color: #d1d5db;
}
input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: #3b82f6;
}
select, input[type="number"] {
  background: #374151;
  color: #e5e7eb;
  border: 1px solid #4b5563;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 13px;
}
input[type="color"] {
  width: 32px;
  height: 24px;
  border: none;
  background: none;
  cursor: pointer;
}
input[type="range"] {
  width: 120px;
  accent-color: #3b82f6;
}
.range-value {
  font-size: 12px;
  color: #9ca3af;
  min-width: 36px;
  text-align: right;
}
kbd {
  background: #374151;
  color: #e5e7eb;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-family: monospace;
  border: 1px solid #4b5563;
}
.settings-actions {
  text-align: right;
  padding: 8px 0;
}
.reset-btn {
  background: #7f1d1d;
  color: #fca5a5;
  border: none;
  padding: 8px 20px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}
.reset-btn:hover { background: #991b1b; }
</style>
