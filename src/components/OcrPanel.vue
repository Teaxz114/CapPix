<template>
  <div class="ocr-panel" v-if="visible">
    <div class="ocr-header">
      <span class="ocr-title">OCR 识别结果</span>
      <button class="ocr-close" @click="$emit('close')">×</button>
    </div>
    <div class="ocr-content">
      <div v-if="loading" class="ocr-loading">识别中...</div>
      <div v-else-if="error" class="ocr-error">{{ error }}</div>
      <div v-else>
        <div class="ocr-text" ref="ocrTextRef">{{ result?.text }}</div>
        <div class="ocr-blocks" v-if="result?.blocks?.length">
          <div v-for="(block, i) in result.blocks" :key="i" class="ocr-block">
            <span class="ocr-block-text">{{ block.text }}</span>
            <span class="ocr-block-conf">{{ (block.confidence * 100).toFixed(0) }}%</span>
          </div>
        </div>
      </div>
    </div>
    <div class="ocr-actions" v-if="result?.text">
      <button @click="$emit('copy-text')">复制文字</button>
      <button @click="selectAll">全选</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from "vue";

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

defineEmits<{
  close: [];
  "copy-text": [];
}>();

const ocrTextRef = ref<HTMLDivElement | null>(null);

function selectAll() {
  if (!ocrTextRef.value) return;
  const range = document.createRange();
  range.selectNodeContents(ocrTextRef.value);
  const selection = window.getSelection();
  if (selection) {
    selection.removeAllRanges();
    selection.addRange(range);
  }
}
</script>

<style scoped>
.ocr-panel {
  position: absolute;
  top: 52px;
  right: 8px;
  width: 320px;
  max-height: 400px;
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
  z-index: 200;
  display: flex;
  flex-direction: column;
  overflow: hidden;
  pointer-events: auto;
}

.ocr-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 8px 12px;
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
  line-height: 1;
  transition: color 0.15s;
}

.ocr-close:hover {
  color: #e5e7eb;
}

.ocr-content {
  flex: 1;
  overflow-y: auto;
  padding: 12px;
}

.ocr-loading {
  color: #9ca3af;
  font-size: 13px;
  text-align: center;
  padding: 16px 0;
}

.ocr-error {
  color: #f87171;
  font-size: 13px;
  padding: 8px;
  background: rgba(248, 113, 113, 0.1);
  border-radius: 4px;
}

.ocr-text {
  color: #f3f4f6;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  word-break: break-all;
  user-select: text;
  cursor: text;
  margin-bottom: 8px;
}

.ocr-blocks {
  border-top: 1px solid #374151;
  padding-top: 8px;
  margin-top: 4px;
}

.ocr-block {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 4px 0;
  border-bottom: 1px solid rgba(55, 65, 81, 0.5);
}

.ocr-block:last-child {
  border-bottom: none;
}

.ocr-block-text {
  color: #d1d5db;
  font-size: 12px;
  flex: 1;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
  margin-right: 8px;
}

.ocr-block-conf {
  color: #6b7280;
  font-size: 11px;
  font-family: monospace;
  flex-shrink: 0;
}

.ocr-actions {
  display: flex;
  gap: 8px;
  padding: 8px 12px;
  border-top: 1px solid #374151;
  background: #111827;
}

.ocr-actions button {
  flex: 1;
  padding: 6px 0;
  border-radius: 4px;
  border: 1px solid #374151;
  background: #1f2937;
  color: #e5e7eb;
  font-size: 12px;
  cursor: pointer;
  transition: all 0.15s;
}

.ocr-actions button:hover {
  background: #374151;
  border-color: #4b5563;
}
</style>
