<template>
  <div class="ocr-panel" v-if="visible">
    <div class="ocr-header">
      <span class="ocr-title">OCR 识别结果</span>
      <button class="ocr-close" @click="close">×</button>
    </div>
    <div v-if="loading" class="ocr-loading">
      <span class="spinner"></span> 识别中...
    </div>
    <div v-else-if="error" class="ocr-error">
      {{ error }}
    </div>
    <div v-else class="ocr-content">
      <div class="ocr-toolbar">
        <button @click="copyAll" title="复制全部">复制全部</button>
        <button @click="copyBlocks" title="复制分块">分块复制</button>
      </div>
      <div class="ocr-text">{{ result?.text || "无识别结果" }}</div>
      <div v-if="result?.blocks?.length" class="ocr-blocks">
        <div v-for="(block, i) in result.blocks" :key="i" class="ocr-block" @click="copyText(block.text)">
          <span class="block-index">{{ i + 1 }}</span>
          <span class="block-text">{{ block.text }}</span>
          <span class="block-conf">{{ (block.confidence * 100).toFixed(0) }}%</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface OcrBlock {
  text: string;
  confidence: number;
  bbox: number[][];
}

interface OcrResult {
  text: string;
  blocks: OcrBlock[];
  elapsed?: number;
  error?: string;
}

const visible = ref(false);
const loading = ref(false);
const error = ref("");
const result = ref<OcrResult | null>(null);

async function recognize(imageBase64: string) {
  visible.value = true;
  loading.value = true;
  error.value = "";
  result.value = null;
  try {
    result.value = await invoke<OcrResult>("ocr_image", { imageBase64 });
    if (result.value?.error) {
      error.value = result.value.error;
    }
  } catch (e) {
    error.value = String(e);
  } finally {
    loading.value = false;
  }
}

function close() {
  visible.value = false;
}

function copyText(text: string) {
  navigator.clipboard.writeText(text);
}

function copyAll() {
  if (result.value?.text) {
    navigator.clipboard.writeText(result.value.text);
  }
}

function copyBlocks() {
  if (result.value?.blocks) {
    const text = result.value.blocks.map((b, i) => `${i + 1}. ${b.text}`).join("\n");
    navigator.clipboard.writeText(text);
  }
}

defineExpose({ recognize, visible });
</script>

<style scoped>
.ocr-panel {
  position: fixed;
  right: 16px;
  top: 60px;
  width: 360px;
  max-height: 80vh;
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
  z-index: 1000;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.ocr-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 10px 14px;
  border-bottom: 1px solid #374151;
  background: #111827;
}
.ocr-title {
  color: #e5e7eb;
  font-size: 13px;
  font-weight: 600;
}
.ocr-close {
  background: none;
  border: none;
  color: #9ca3af;
  font-size: 18px;
  cursor: pointer;
  padding: 0 4px;
}
.ocr-close:hover { color: #ef4444; }
.ocr-loading {
  padding: 24px;
  text-align: center;
  color: #9ca3af;
  font-size: 13px;
}
.spinner {
  display: inline-block;
  width: 16px;
  height: 16px;
  border: 2px solid #4b5563;
  border-top-color: #3b82f6;
  border-radius: 50%;
  animation: spin 0.6s linear infinite;
  margin-right: 8px;
  vertical-align: middle;
}
@keyframes spin { to { transform: rotate(360deg); } }
.ocr-error {
  padding: 16px;
  color: #ef4444;
  font-size: 13px;
}
.ocr-content {
  display: flex;
  flex-direction: column;
  overflow-y: auto;
}
.ocr-toolbar {
  display: flex;
  gap: 8px;
  padding: 8px 14px;
  border-bottom: 1px solid #374151;
}
.ocr-toolbar button {
  background: #374151;
  color: #e5e7eb;
  border: none;
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
}
.ocr-toolbar button:hover { background: #4b5563; }
.ocr-text {
  padding: 12px 14px;
  color: #e5e7eb;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  max-height: 200px;
  overflow-y: auto;
  border-bottom: 1px solid #374151;
}
.ocr-blocks {
  padding: 8px 0;
}
.ocr-block {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 14px;
  cursor: pointer;
  transition: background 0.15s;
}
.ocr-block:hover { background: #374151; }
.block-index {
  color: #6b7280;
  font-size: 11px;
  min-width: 20px;
}
.block-text {
  flex: 1;
  color: #d1d5db;
  font-size: 12px;
}
.block-conf {
  color: #6b7280;
  font-size: 10px;
}
</style>
