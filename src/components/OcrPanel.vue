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
      <p>{{ error }}</p>
      <button class="retry-button" @click="retryRecognition" :disabled="!lastImageBase64">重试识别</button>
    </div>
    <div v-else class="ocr-content">
      <div class="ocr-toolbar">
        <button @click="copyAll" title="复制全部">复制全部</button>
        <button @click="copyBlocks" title="复制分块">分块复制</button>
        <button @click="translate" :disabled="translating" title="翻译">
          {{ translating ? "翻译中..." : "翻译" }}
        </button>
        <select v-model="targetLang" class="lang-select" title="目标语言">
          <option value="en">英语</option>
          <option value="zh">中文</option>
          <option value="ja">日语</option>
          <option value="ko">韩语</option>
        </select>
      </div>

      <!-- Original OCR text -->
      <div class="ocr-text-section">
        <div class="section-label">原文</div>
        <div class="ocr-text">{{ result?.text || "无识别结果" }}</div>
      </div>

      <!-- Translation result (shown after translation) -->
      <div v-if="translation" class="ocr-text-section translated">
        <div class="section-label">翻译 ({{ translation.source_lang }} → {{ translation.target_lang }})</div>
        <div class="ocr-text">{{ translation.translated }}</div>
        <button class="copy-translation" @click="copyTranslation">复制翻译</button>
      </div>
      <div v-if="translationError" class="translation-error">
        <span>{{ translationError }}</span>
        <button class="retry-button" @click="translate" :disabled="translating">重试翻译</button>
      </div>

      <!-- OCR blocks -->
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
import { useConfigStore } from "../stores/config";

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

interface TranslateResult {
  original: string;
  translated: string;
  source_lang: string;
  target_lang: string;
}

const visible = ref(false);
const loading = ref(false);
const error = ref("");
const result = ref<OcrResult | null>(null);
const translation = ref<TranslateResult | null>(null);
const translating = ref(false);
const targetLang = ref("zh");
const translationError = ref("");
const lastImageBase64 = ref("");
const lastAutoTranslate = ref(false);
let translationRequestId = 0;
const { config } = useConfigStore();

async function recognize(imageBase64: string, autoTranslate = false) {
  visible.value = true;
  loading.value = true;
  error.value = "";
  result.value = null;
  translation.value = null;
  translationError.value = "";
  translating.value = false;
  translationRequestId++;
  lastImageBase64.value = imageBase64;
  lastAutoTranslate.value = autoTranslate;
  try {
    result.value = await invoke<OcrResult>("ocr_image", {
      imageBase64,
      language: config.ocrLanguage,
      allowOnlineFallback: config.allowOnlineOcrFallback,
    });
    if (result.value?.error) {
      error.value = result.value.error;
    } else if (autoTranslate && result.value?.text) {
      // Auto-translate right after OCR completes (screenshot-translate flow)
      await translate();
    }
  } catch (e) {
    error.value = formatOcrError(e);
  } finally {
    loading.value = false;
  }
}

async function retryRecognition() {
  if (lastImageBase64.value) {
    await recognize(lastImageBase64.value, lastAutoTranslate.value);
  }
}

async function translate() {
  if (!result.value?.text) return;
  const text = result.value.text;
  const requestId = ++translationRequestId;
  translating.value = true;
  translation.value = null;
  translationError.value = "";
  try {
    const translated = await invoke<TranslateResult>("ocr_translate", {
      text,
      targetLang: targetLang.value,
    });
    if (requestId === translationRequestId && result.value?.text === text) {
      translation.value = translated;
    }
  } catch (e) {
    if (requestId === translationRequestId) {
      translationError.value = `翻译失败，请重试。${formatErrorDetail(e)}`;
    }
  } finally {
    if (requestId === translationRequestId) {
      translating.value = false;
    }
  }
}

function formatOcrError(error: unknown) {
  const message = String(error);
  if (message.includes("本地 OCR") || message.includes("图片过大")) return message;
  if (message.includes("not found") || message.includes("worker")) {
    return "本地 OCR 引擎未安装。为保护截图隐私，截图未上传到云端。请安装 cappix_ocr.exe 或 Python + rapidocr，或在设置中明确开启云端 OCR 回退后重试。";
  }
  return `OCR 识别失败，请重试。${formatErrorDetail(error)}`;
}

function formatErrorDetail(error: unknown) {
  const message = String(error).trim();
  return message ? `（${message}）` : "";
}

function close() {
  visible.value = false;
}

async function copyText(text: string) {
  try {
    const { writeText } = await import("@tauri-apps/plugin-clipboard-manager");
    await writeText(text);
  } catch (e) {
    console.error("Copy text failed:", e);
  }
}

async function copyAll() {
  if (result.value?.text) {
    await copyText(result.value.text);
  }
}

async function copyBlocks() {
  if (result.value?.blocks) {
    const text = result.value.blocks.map((b, i) => `${i + 1}. ${b.text}`).join("\n");
    await copyText(text);
  }
}

async function copyTranslation() {
  if (translation.value?.translated) {
    await copyText(translation.value.translated);
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
.ocr-error p { margin: 0 0 10px; white-space: pre-wrap; }
.retry-button {
  background: #374151;
  border: 1px solid #6b7280;
  border-radius: 4px;
  color: #e5e7eb;
  cursor: pointer;
  font-size: 12px;
  padding: 4px 10px;
}
.retry-button:hover { background: #4b5563; }
.retry-button:disabled { cursor: not-allowed; opacity: 0.5; }
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
  align-items: center;
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
.ocr-toolbar button:disabled { opacity: 0.5; cursor: not-allowed; }
.lang-select {
  background: #374151;
  color: #e5e7eb;
  border: none;
  padding: 4px 8px;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
}
.ocr-text-section {
  padding: 12px 14px;
  border-bottom: 1px solid #374151;
}
.section-label {
  color: #6b7280;
  font-size: 11px;
  margin-bottom: 4px;
}
.ocr-text-section.translated {
  background: rgba(59, 130, 246, 0.1);
}
.ocr-text {
  color: #e5e7eb;
  font-size: 13px;
  line-height: 1.6;
  white-space: pre-wrap;
  max-height: 200px;
  overflow-y: auto;
}
.copy-translation {
  background: #3b82f6;
  color: #fff;
  border: none;
  padding: 4px 12px;
  border-radius: 4px;
  font-size: 11px;
  cursor: pointer;
  margin-top: 6px;
}
.copy-translation:hover { background: #2563eb; }
.translation-error {
  align-items: center;
  color: #fca5a5;
  display: flex;
  font-size: 12px;
  gap: 8px;
  justify-content: space-between;
  padding: 8px 14px;
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
