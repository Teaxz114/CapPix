<template>
  <div class="ocr-panel" v-if="visible">
    <div class="ocr-header">
      <span class="ocr-title">OCR 识别结果</span>
      <button class="ocr-close" @click="$emit('close')">×</button>
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
      <div class="ocr-text">
        <pre>{{ result?.text || "" }}</pre>
      </div>
      <div v-if="result?.blocks?.length" class="ocr-blocks">
        <div v-for="(block, i) in result.blocks" :key="i" class="ocr-block">
          <span class="block-index">{{ i + 1 }}</span>
          <span class="block-text">{{ block.text }}</span>
          <span class="block-conf">{{ (block.confidence * 100).toFixed(1) }}%</span>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
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

defineProps<{
  visible: boolean;
  result: OcrResult | null;
  loading: boolean;
  error: string;
}>();

const emit = defineEmits<{
  close: [];
  copyText: [text: string];
}>();

function copyAll() {
  // Will be handled by parent via copyOcrText
  emit("copyText", "all");
}

function copyBlocks() {
  emit("copyText", "blocks");
}
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
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
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
  background: #111827;
  border-bottom: 1px solid #374151;
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
  animation: spin 0.8s linear infinite;
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
  overflow: auto;
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
}

.ocr-text pre {
  color: #e5e7eb;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  margin: 0;
  font-family: "Microsoft YaHei", sans-serif;
}

.ocr-blocks {
  padding: 8px 14px;
  border-top: 1px solid #374151;
  max-height: 200px;
  overflow-y: auto;
}

.ocr-block {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 4px 0;
  font-size: 12px;
}

.block-index {
  background: #3b82f6;
  color: #fff;
  border-radius: 50%;
  width: 20px;
  height: 20px;
  display: flex;
  align-items: center;
  justify-content: center;
  font-size: 10px;
  flex-shrink: 0;
}

.block-text {
  color: #d1d5db;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.block-conf {
  color: #6b7280;
  font-size: 10px;
  flex-shrink: 0;
}
</style>
