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
      <h3>标注</h3>
      <div class="setting-row">
        <label>默认颜色</label>
        <input type="color" v-model="config.defaultColor" />
      </div>
      <div class="setting-row">
        <label>默认线宽</label>
        <input type="range" v-model.number="config.defaultStrokeWidth" min="1" max="20" />
        <span class="range-val">{{ config.defaultStrokeWidth }}px</span>
      </div>
      <div class="setting-row">
        <label>马赛克块大小</label>
        <input type="range" v-model.number="config.mosaicBlockSize" min="4" max="30" />
        <span class="range-val">{{ config.mosaicBlockSize }}px</span>
      </div>
      <div class="setting-row">
        <label>字体</label>
        <select v-model="config.defaultFontFamily">
          <option value="Microsoft YaHei">微软雅黑</option>
          <option value="SimSun">宋体</option>
          <option value="SimHei">黑体</option>
          <option value="Arial">Arial</option>
        </select>
      </div>
      <div class="setting-row">
        <label>字号</label>
        <input type="number" v-model.number="config.defaultFontSize" min="10" max="72" />
      </div>
    </div>

    <div class="settings-section">
      <h3>保存</h3>
      <div class="setting-row">
        <label>默认格式</label>
        <select v-model="config.saveFormat">
          <option value="png">PNG</option>
          <option value="jpg">JPG</option>
          <option value="bmp">BMP</option>
        </select>
      </div>
      <div class="setting-row">
        <label>JPG 质量</label>
        <input type="range" v-model.number="config.saveQuality" min="10" max="100" />
        <span class="range-val">{{ config.saveQuality }}%</span>
      </div>
    </div>

    <div class="settings-section">
      <h3>贴图</h3>
      <div class="setting-row">
        <label>透明度</label>
        <input type="range" v-model.number="config.pinOpacity" min="10" max="100" step="5" />
        <span class="range-val">{{ (config.pinOpacity * 100).toFixed(0) }}%</span>
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
      <button class="btn-reset" @click="resetConfig">恢复默认</button>
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
  padding: 24px;
  max-width: 640px;
  margin: 0 auto;
  color: #e5e7eb;
}

h2 {
  font-size: 18px;
  font-weight: 600;
  margin: 0 0 20px;
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
  margin: 0 0 12px;
  color: #9ca3af;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
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

input[type="color"] {
  width: 36px;
  height: 28px;
  border: 1px solid #4b5563;
  border-radius: 4px;
  background: none;
  cursor: pointer;
}

input[type="range"] {
  width: 120px;
  accent-color: #3b82f6;
}

.range-val {
  font-size: 12px;
  color: #9ca3af;
  min-width: 40px;
  text-align: right;
}

select {
  background: #374151;
  color: #e5e7eb;
  border: 1px solid #4b5563;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 13px;
}

input[type="number"] {
  background: #374151;
  color: #e5e7eb;
  border: 1px solid #4b5563;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 13px;
  width: 60px;
}

kbd {
  background: #374151;
  color: #d1d5db;
  padding: 2px 8px;
  border-radius: 4px;
  font-size: 12px;
  font-family: inherit;
  border: 1px solid #4b5563;
}

.settings-actions {
  display: flex;
  justify-content: flex-end;
}

.btn-reset {
  background: #374151;
  color: #e5e7eb;
  border: 1px solid #4b5563;
  padding: 8px 16px;
  border-radius: 6px;
  font-size: 13px;
  cursor: pointer;
}
.btn-reset:hover { background: #4b5563; }
</style>
