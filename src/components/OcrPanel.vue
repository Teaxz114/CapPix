<template>
  <div class="ocr-panel" v-if="visible">
    <div class="ocr-header">
      <span class="ocr-title">OCR 识别结果</span>
      <button class="ocr-close" @click="emit('close')">×</button>
    </div>
    <div v-if="loading" class="ocr-loading">识别中...</div>
    <div v-else-if="error" class="ocr-error">{{ error }}</div>
    <div v-else class="ocr-content">
      <div class="ocr-text" ref="textRef">
        <div v-for="(block, i) in (result?.blocks || [])" :key="i" class="ocr-block">
          <span class="ocr-block-text">{{ block.text }}</span>
          <span class="ocr-block-conf">{{ (block.confidence * 100).toFixed(0) }}%</span>
        </div>
        <div v-if="!result?.blocks?.length" class="ocr-empty">未识别到文字</div>
      </div>
      <div class="ocr-actions">
        <button @click="copyAllText" class="ocr-btn">复制全部</button>
        <button @click="selectAll" class="ocr-btn">全选</button>
      </div>
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

const props = defineProps<{
  visible: boolean;
  result: OcrResult | null;
  loading: boolean;
  error: string;
}>();

const emit = defineEmits<{
  (e: "close"): void;
  (e: "copy-text", text: string): void;
}>();

const textRef = ref<HTMLDivElement | null>(null);

function copyAllText() {
  const text = props.result?.text || "";
  emit("copy-text", text);
  navigator.clipboard.writeText(text).catch(() => {
    const ta = document.createElement("textarea");
    ta.value = text;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    document.body.removeChild(ta);
  });
}

function selectAll() {
  if (textRef.value) {
    const range = document.createRange();
    range.selectNodeContents(textRef.value);
    const sel = window.getSelection();
    sel?.removeAllRanges();
    sel?.addRange(range);
  }
}
</script>

<style scoped>
.ocr-panel {
  position: fixed;
  right: 16px;
  top: 60px;
  width: 360px;
  max-height: 500px;
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.4);
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
}
.ocr-title { color: #e5e7eb; font-size: 13px; font-weight: 600; }
.ocr-close {
  background: none; border: none; color: #9ca3af;
  font-size: 18px; cursor: pointer; padding: 0 4px;
}
.ocr-close:hover { color: #f87171; }
.ocr-loading, .ocr-error, .ocr-empty {
  padding: 20px; color: #9ca3af; text-align: center; font-size: 13px;
}
.ocr-error { color: #f87171; }
.ocr-content { flex: 1; overflow-y: auto; padding: 10px 14px; }
.ocr-block {
  display: flex; justify-content: space-between; align-items: baseline;
  padding: 4px 0; border-bottom: 1px solid #1a1a2e;
}
.ocr-block:last-child { border-bottom: none; }
.ocr-block-text { color: #e5e7eb; font-size: 13px; flex: 1; user-select: text; }
.ocr-block-conf { color: #6b7280; font-size: 11px; margin-left: 8px; }
.ocr-actions {
  display: flex; gap: 8px; padding: 10px 14px; border-top: 1px solid #374151;
}
.ocr-btn {
  flex: 1; padding: 6px 12px; background: #374151; color: #e5e7eb;
  border: none; border-radius: 4px; font-size: 12px; cursor: pointer;
}
.ocr-btn:hover { background: #4b5563; }
</style>
