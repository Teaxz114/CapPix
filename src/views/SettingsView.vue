<template>
  <div class="settings-view">
    <div class="settings-header">
      <button class="back-btn" @click="goBack">← 返回</button>
      <h1>设置</h1>
    </div>

    <div class="settings-content">
      <!-- 保存设置 -->
      <section class="settings-section">
        <h2>保存</h2>
        <div class="setting-row">
          <label>默认保存格式</label>
          <select v-model="config.saveFormat">
            <option value="png">PNG</option>
            <option value="jpg">JPG</option>
            <option value="bmp">BMP</option>
          </select>
        </div>
        <div class="setting-row">
          <label>JPG 质量 (1-100)</label>
          <input type="number" v-model.number="config.saveQuality" min="1" max="100" />
        </div>
        <div class="setting-row">
          <label>
            <input type="checkbox" v-model="config.autoCopyToClipboard" />
            截图后自动复制到剪贴板
          </label>
        </div>
      </section>

      <!-- 标注设置 -->
      <section class="settings-section">
        <h2>标注</h2>
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
        <div class="setting-row">
          <label>马赛克块大小</label>
          <input type="range" v-model.number="config.mosaicBlockSize" min="4" max="30" />
          <span class="range-value">{{ config.mosaicBlockSize }}px</span>
        </div>
      </section>

      <!-- 截图设置 -->
      <section class="settings-section">
        <h2>截图</h2>
        <div class="setting-row">
          <label>
            <input type="checkbox" v-model="config.showMagnifier" />
            显示放大镜
          </label>
        </div>
      </section>

      <!-- 贴图设置 -->
      <section class="settings-section">
        <h2>贴图</h2>
        <div class="setting-row">
          <label>贴图透明度</label>
          <input type="range" v-model.number="config.pinOpacity" min="10" max="100" step="5" />
          <span class="range-value">{{ (config.pinOpacity * 100).toFixed(0) }}%</span>
        </div>
      </section>

      <!-- 主题 -->
      <section class="settings-section">
        <h2>外观</h2>
        <div class="setting-row">
          <label>主题</label>
          <select v-model="config.theme">
            <option value="dark">深色</option>
            <option value="light">浅色</option>
          </select>
        </div>
      </section>

      <!-- 重置 -->
      <section class="settings-section">
        <button class="reset-btn" @click="resetConfig">恢复默认设置</button>
      </section>
    </div>
  </div>
</template>

<script setup lang="ts">
import { useRouter } from "vue-router";
import { useConfigStore } from "../stores/config";
import { storeToRefs } from "pinia";

const router = useRouter();
const configStore = useConfigStore();
const { config } = storeToRefs(configStore);

function goBack() {
  router.push("/");
}

function resetConfig() {
  if (confirm("确定要恢复默认设置吗？")) {
    configStore.resetConfig();
  }
}
</script>

<style scoped>
.settings-view {
  width: 100vw;
  height: 100vh;
  background: #111827;
  color: #e5e7eb;
  overflow-y: auto;
  display: flex;
  flex-direction: column;
}

.settings-header {
  display: flex;
  align-items: center;
  gap: 16px;
  padding: 16px 24px;
  border-bottom: 1px solid #374151;
  flex-shrink: 0;
}

.settings-header h1 {
  font-size: 20px;
  margin: 0;
  color: #f3f4f6;
}

.back-btn {
  background: none;
  border: 1px solid #4b5563;
  color: #e5e7eb;
  padding: 6px 12px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
}
.back-btn:hover {
  background: #1f2937;
}

.settings-content {
  padding: 24px;
  max-width: 600px;
  width: 100%;
  margin: 0 auto;
}

.settings-section {
  margin-bottom: 32px;
}

.settings-section h2 {
  font-size: 16px;
  color: #9ca3af;
  margin: 0 0 16px 0;
  padding-bottom: 8px;
  border-bottom: 1px solid #1f2937;
}

.setting-row {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 10px 0;
  gap: 12px;
}

.setting-row label {
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 14px;
  color: #d1d5db;
}

.setting-row input[type="color"] {
  width: 40px;
  height: 30px;
  border: none;
  border-radius: 4px;
  cursor: pointer;
  background: none;
}

.setting-row select {
  background: #1f2937;
  color: #e5e7eb;
  border: 1px solid #4b5563;
  border-radius: 6px;
  padding: 6px 12px;
  font-size: 14px;
}

.setting-row input[type="number"] {
  width: 80px;
  background: #1f2937;
  color: #e5e7eb;
  border: 1px solid #4b5563;
  border-radius: 6px;
  padding: 6px 10px;
  font-size: 14px;
  text-align: center;
}

.setting-row input[type="range"] {
  flex: 1;
  max-width: 200px;
}

.range-value {
  font-size: 13px;
  color: #9ca3af;
  min-width: 40px;
  text-align: right;
}

.setting-row input[type="checkbox"] {
  width: 18px;
  height: 18px;
  accent-color: #3b82f6;
}

.reset-btn {
  background: #dc2626;
  color: white;
  border: none;
  padding: 10px 20px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 14px;
}
.reset-btn:hover {
  background: #b91c1c;
}
</style>
