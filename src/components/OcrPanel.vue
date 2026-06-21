<template>
  <div class="ocr-panel" :class="{ open: visible }">
    <div class="ocr-header">
      <span class="ocr-title">OCR 识别结果</span>
      <button class="ocr-close" @click="$emit('close')">×</button>
    </div>
    <div v-if="loading" class="ocr-loading">
      <span class="spinner"></span> 识别中...
    </div>
    <div v-else-if="error" class="ocr-error">{{ error }}</div>
    <div v-else-if="result" class="ocr-content">
      <div class="ocr-toolbar">
        <button @click="copyAll" title="复制全部">复制全部</button>
        <button @click="copyText" title="复制纯文本">复制文本</button>
      </div>
      <div class="ocr-text">
        <div
          v-for="(block, i) in result.blocks"
          :key="i"
          class="ocr-block"
          @click="selectedBlock = selectedBlock === i ? -1 : i"
          :class="{ selected: selectedBlock === i }"
        >
          <span class="block-index">{{ i + 1 }}</span>
          <span class="block-text">{{ block.text }}</span>
          <span class="block-confidence">{{ (block.confidence * 100).toFixed(0) }}%</span>
        </div>
      </div>
      <div v-if="result.blocks.length" class="ocr-stats">
        {{ result.blocks.length }} 个文本块 · 耗时 {{ result.elapsed?.toFixed(0) || '?' }}ms
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface OcrBlock {
  text: string;
  confidence: number;
  bbox: number[][];
}

interface OcrResult {
  text: string;
  blocks: OcrBlock[];
  elapsed: number | null;
  error?: string;
}

const props = defineProps<{
  visible: boolean;
  result: OcrResult | null;
  loading: boolean;
  error: string;
}>();

defineEmits<{
  close: [];
}>();

const selectedBlock = ref(-1);

const fullText = computed(() => props.result?.blocks.map(b => b.text).join("\n") || "");

async function copyAll() {
  try {
    await navigator.clipboard.writeText(fullText.value);
  } catch {
    const ta = document.createElement("textarea");
    ta.value = fullText.value;
    document.body.appendChild(ta);
    ta.select();
    document.execCommand("copy");
    document.body.removeChild(ta);
  }
}

async function copyText() {
  await copyAll();
}

// Also expose imperative runOcr for direct use
async function runOcr(imageBase64: string): Promise<OcrResult | null> {
  try {
    return await invoke<OcrResult>("ocr_image", { imageBase64 });
  } catch (e) {
    console.error("OCR failed:", e);
    return null;
  }
}

defineExpose({ runOcr });
</script>

<style scoped>
.ocr-panel {
  position: fixed;
  right: -400px;
  top: 60px;
  width: 380px;
  max-height: calc(100vh - 120px);
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4);
  z-index: 1000;
  transition: right 0.3s ease;
  display: flex;
  flex-direction: column;
  overflow: hidden;
}
.ocr-panel.open {
  right: 16px;
}
.ocr-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 12px 16px;
  border-bottom: 1px solid #374151;
}
.ocr-title {
  color: #e5e7eb;
  font-size: 14px;
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
.ocr-close:hover { color: #f87171; }
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
  color: #f87171;
  font-size: 13px;
}
.ocr-content {
  display: flex;
  flex-direction: column;
  flex: 1;
  overflow: hidden;
}
.ocr-toolbar {
  display: flex;
  gap: 8px;
  padding: 8px 16px;
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
  flex: 1;
  overflow-y: auto;
  padding: 8px;
}
.ocr-block {
  display: flex;
  align-items: center;
  gap: 8px;
  padding: 6px 8px;
  border-radius: 4px;
  cursor: pointer;
  font-size: 13px;
  color: #d1d5db;
}
.ocr-block:hover { background: #374151; }
.ocr-block.selected { background: #1e3a5f; border: 1px solid #3b82f6; }
.block-index {
  color: #6b7280;
  font-size: 11px;
  min-width: 20px;
}
.block-text { flex: 1; word-break: break-all; }
.block-confidence {
  color: #6b7280;
  font-size: 11px;
}
.ocr-stats {
  padding: 8px 16px;
  border-top: 1px solid #374151;
  color: #6b7280;
  font-size: 11px;
  text-align: center;
}
</style>
