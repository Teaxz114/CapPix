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

      <!-- Hotkey display -->
      <section class="settings-section">
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
import { useConfigStore } from "../stores/config";
import { storeToRefs } from "pinia";

const configStore = useConfigStore();
const { config } = storeToRefs(configStore);

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
</style>
