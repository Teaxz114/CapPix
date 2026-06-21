<template>
  <div class="min-h-screen bg-gray-900 text-gray-100 p-6">
    <header class="mb-6 flex items-center gap-4">
      <button
        class="bg-gray-700 hover:bg-gray-600 text-gray-200 px-3 py-2 rounded-lg text-sm transition-colors"
        @click="$router.push('/')"
      >
        ← 返回
      </button>
      <h1 class="text-2xl font-bold text-white">历史记录</h1>
      <div class="flex-1"></div>
      <button
        v-if="records.length > 0"
        class="bg-red-700 hover:bg-red-600 text-white px-3 py-2 rounded-lg text-sm transition-colors"
        @click="handleClearAll"
      >
        清空全部
      </button>
    </header>

    <!-- Search bar -->
    <div class="mb-6">
      <div class="relative">
        <input
          v-model="searchText"
          type="text"
          placeholder="搜索 OCR 文本..."
          class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2.5 text-gray-200 text-sm focus:outline-none focus:border-blue-500 placeholder-gray-500"
          @input="onSearchInput"
        />
        <svg
          v-if="!searchText"
          class="absolute right-3 top-1/2 -translate-y-1/2 w-4 h-4 text-gray-500"
          fill="none"
          stroke="currentColor"
          viewBox="0 0 24 24"
        >
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
        </svg>
        <button
          v-else
          class="absolute right-3 top-1/2 -translate-y-1/2 text-gray-500 hover:text-gray-300"
          @click="searchText = ''; loadHistory()"
        >
          ✕
        </button>
      </div>
    </div>

    <!-- Loading -->
    <div v-if="loading" class="text-center text-gray-500 py-12">加载中...</div>

    <!-- Empty state -->
    <div v-else-if="records.length === 0" class="text-center text-gray-500 py-12">
      <div class="text-4xl mb-4">📷</div>
      <p>{{ searchText ? '没有找到匹配的记录' : '还没有截图记录' }}</p>
    </div>

    <!-- Grid of thumbnails -->
    <div v-else class="grid grid-cols-2 sm:grid-cols-3 md:grid-cols-4 lg:grid-cols-5 xl:grid-cols-6 gap-4">
      <div
        v-for="record in records"
        :key="record.id"
        class="bg-gray-800 rounded-lg overflow-hidden hover:bg-gray-750 transition-colors cursor-pointer group"
        @click="selectRecord(record)"
      >
        <div class="aspect-video bg-gray-950 flex items-center justify-center overflow-hidden">
          <img
            :src="'data:image/jpeg;base64,' + record.thumbnail_base64"
            :alt="'Screenshot ' + record.id"
            class="max-w-full max-h-full object-contain"
          />
        </div>
        <div class="p-2">
          <div class="text-xs text-gray-400 truncate">{{ record.timestamp }}</div>
          <div class="text-xs text-gray-500 truncate">{{ record.width }}×{{ record.height }}</div>
          <div v-if="record.ocr_text" class="text-xs text-gray-600 truncate mt-1">{{ record.ocr_text }}</div>
        </div>
        <div class="px-2 pb-2 flex justify-end opacity-0 group-hover:opacity-100 transition-opacity">
          <button
            class="text-red-400 hover:text-red-300 text-xs"
            @click.stop="handleDelete(record.id)"
          >
            删除
          </button>
        </div>
      </div>
    </div>

    <!-- Load more -->
    <div v-if="records.length > 0 && hasMore" class="text-center mt-6">
      <button
        class="bg-gray-700 hover:bg-gray-600 text-gray-200 px-4 py-2 rounded-lg text-sm transition-colors"
        @click="loadMore"
      >
        加载更多
      </button>
    </div>

    <!-- Preview overlay -->
    <div
      v-if="previewRecord"
      class="fixed inset-0 bg-black/80 z-50 flex items-center justify-center p-8"
      @click.self="previewRecord = null"
    >
      <div class="max-w-5xl max-h-full overflow-auto bg-gray-900 rounded-lg shadow-2xl">
        <div class="flex items-center justify-between p-3 border-b border-gray-700">
          <div>
            <span class="text-sm text-gray-400">{{ previewRecord.timestamp }}</span>
            <span class="text-xs text-gray-500 ml-2">{{ previewRecord.width }}×{{ previewRecord.height }}</span>
          </div>
          <div class="flex gap-2">
            <button
              class="text-gray-400 hover:text-gray-200 text-sm px-2 py-1"
              @click="previewRecord = null"
            >
              关闭
            </button>
          </div>
        </div>
        <div class="p-4 flex items-center justify-center">
          <img
            :src="'data:image/png;base64,' + previewRecord.image_base64"
            :alt="'Screenshot ' + previewRecord.id"
            class="max-w-full max-h-[75vh] object-contain"
          />
        </div>
        <div v-if="previewRecord.ocr_text" class="p-3 border-t border-gray-700">
          <div class="text-xs text-gray-500 mb-1">OCR 文本:</div>
          <div class="text-sm text-gray-300 whitespace-pre-wrap max-h-32 overflow-y-auto">{{ previewRecord.ocr_text }}</div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface ScreenshotRecord {
  id: number;
  timestamp: string;
  image_base64: string;
  thumbnail_base64: string;
  width: number;
  height: number;
  ocr_text: string | null;
}

const records = ref<ScreenshotRecord[]>([]);
const loading = ref(false);
const searchText = ref("");
const previewRecord = ref<ScreenshotRecord | null>(null);
const currentOffset = ref(0);
const hasMore = ref(true);
const PAGE_SIZE = 50;

let searchDebounce: ReturnType<typeof setTimeout> | null = null;

onMounted(() => {
  loadHistory();
});

async function loadHistory() {
  loading.value = true;
  currentOffset.value = 0;
  hasMore.value = true;
  try {
    const search = searchText.value.trim() || undefined;
    records.value = await invoke<ScreenshotRecord[]>("get_history", {
      limit: PAGE_SIZE,
      offset: 0,
      search,
    });
    currentOffset.value = records.value.length;
    hasMore.value = records.value.length === PAGE_SIZE;
  } catch (e) {
    console.error("Failed to load history:", e);
  } finally {
    loading.value = false;
  }
}

async function loadMore() {
  loading.value = true;
  try {
    const search = searchText.value.trim() || undefined;
    const more = await invoke<ScreenshotRecord[]>("get_history", {
      limit: PAGE_SIZE,
      offset: currentOffset.value,
      search,
    });
    records.value.push(...more);
    currentOffset.value = records.value.length;
    hasMore.value = more.length === PAGE_SIZE;
  } catch (e) {
    console.error("Failed to load more history:", e);
  } finally {
    loading.value = false;
  }
}

function onSearchInput() {
  if (searchDebounce) clearTimeout(searchDebounce);
  searchDebounce = setTimeout(() => {
    loadHistory();
  }, 400);
}

function selectRecord(record: ScreenshotRecord) {
  previewRecord.value = record;
}

async function handleDelete(id: number) {
  try {
    await invoke("delete_history_item", { id });
    records.value = records.value.filter((r) => r.id !== id);
    if (previewRecord.value?.id === id) {
      previewRecord.value = null;
    }
  } catch (e) {
    console.error("Failed to delete history item:", e);
  }
}

async function handleClearAll() {
  if (!confirm("确定要清空所有历史记录吗？此操作不可撤销。")) return;
  try {
    await invoke("clear_history");
    records.value = [];
    previewRecord.value = null;
  } catch (e) {
    console.error("Failed to clear history:", e);
  }
}
</script>
