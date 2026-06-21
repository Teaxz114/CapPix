<template>
  <div class="history-view">
    <div class="history-header">
      <h2>截图历史</h2>
      <div class="history-actions">
        <input
          v-model="searchQuery"
          placeholder="搜索 OCR 文字..."
          class="search-input"
          @input="onSearch"
        />
        <button @click="loadHistory" class="refresh-btn">刷新</button>
      </div>
    </div>
    <div v-if="loading" class="loading">加载中...</div>
    <div v-else-if="records.length === 0" class="empty">
      <p>暂无截图历史</p>
      <p class="hint">截图后会自动保存到历史记录</p>
    </div>
    <div v-else class="history-grid">
      <div
        v-for="record in records"
        :key="record.id"
        class="history-card"
        @click="openRecord(record)"
      >
        <img
          :src="`data:image/png;base64,${record.image_base64}`"
          class="thumbnail"
          loading="lazy"
        />
        <div class="card-info">
          <span class="card-time">{{ record.timestamp }}</span>
          <span class="card-source">{{ record.source }}</span>
          <span class="card-size">{{ record.width }}×{{ record.height }}</span>
          <button class="card-delete" @click.stop="deleteRecord(record.id)">×</button>
        </div>
        <div v-if="record.ocr_text" class="card-ocr">
          {{ record.ocr_text.substring(0, 80) }}{{ record.ocr_text.length > 80 ? '...' : '' }}
        </div>
      </div>
    </div>
    <div v-if="records.length > 0" class="history-footer">
      共 {{ totalCount }} 条记录
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
  width: number;
  height: number;
  source: string;
  ocr_text: string | null;
}

const records = ref<ScreenshotRecord[]>([]);
const loading = ref(false);
const searchQuery = ref("");
const totalCount = ref(0);
let searchTimer: ReturnType<typeof setTimeout> | null = null;

onMounted(() => {
  loadHistory();
});

async function loadHistory() {
  loading.value = true;
  try {
    records.value = await invoke<ScreenshotRecord[]>("history_list", { limit: 50, offset: 0 });
    totalCount.value = await invoke<number>("history_count");
  } catch (e) {
    console.error("Failed to load history:", e);
  } finally {
    loading.value = false;
  }
}

function onSearch() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(async () => {
    if (!searchQuery.value.trim()) {
      loadHistory();
      return;
    }
    loading.value = true;
    try {
      records.value = await invoke<ScreenshotRecord[]>("history_search", {
        query: searchQuery.value,
        limit: 50,
      });
    } catch (e) {
      console.error("Search failed:", e);
    } finally {
      loading.value = false;
    }
  }, 300);
}

async function deleteRecord(id: number) {
  try {
    await invoke("history_delete", { id });
    records.value = records.value.filter(r => r.id !== id);
    totalCount.value--;
  } catch (e) {
    console.error("Delete failed:", e);
  }
}

function openRecord(record: ScreenshotRecord) {
  // Open in annotate window
  invoke("open_annotate_window", { imageBase64: record.image_base64 });
}
</script>

<style scoped>
.history-view {
  padding: 24px;
  max-width: 1200px;
  margin: 0 auto;
  color: #e5e7eb;
}
.history-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.history-header h2 {
  font-size: 20px;
  font-weight: 600;
  margin: 0;
}
.history-actions {
  display: flex;
  gap: 8px;
}
.search-input {
  background: #374151;
  border: 1px solid #4b5563;
  color: #e5e7eb;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 13px;
  width: 200px;
}
.search-input::placeholder { color: #6b7280; }
.refresh-btn {
  background: #374151;
  color: #e5e7eb;
  border: none;
  padding: 6px 16px;
  border-radius: 6px;
  cursor: pointer;
  font-size: 13px;
}
.refresh-btn:hover { background: #4b5563; }
.loading, .empty {
  text-align: center;
  padding: 60px 20px;
  color: #6b7280;
}
.empty .hint { font-size: 13px; margin-top: 8px; }
.history-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
  gap: 16px;
}
.history-card {
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  overflow: hidden;
  cursor: pointer;
  transition: border-color 0.2s;
}
.history-card:hover { border-color: #3b82f6; }
.thumbnail {
  width: 100%;
  height: 140px;
  object-fit: cover;
  display: block;
}
.card-info {
  padding: 8px 12px;
  display: flex;
  align-items: center;
  gap: 8px;
  font-size: 11px;
  color: #9ca3af;
}
.card-time { flex: 1; }
.card-source {
  background: #374151;
  padding: 1px 6px;
  border-radius: 3px;
}
.card-delete {
  background: none;
  border: none;
  color: #6b7280;
  cursor: pointer;
  font-size: 14px;
  padding: 0 2px;
}
.card-delete:hover { color: #f87171; }
.card-ocr {
  padding: 0 12px 8px;
  font-size: 11px;
  color: #6b7280;
  line-height: 1.4;
}
.history-footer {
  text-align: center;
  padding: 16px;
  color: #6b7280;
  font-size: 12px;
}
</style>
