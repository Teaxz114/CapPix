<template>
  <div class="history-view">
    <div class="history-header">
      <h2>截图历史</h2>
      <div class="history-search">
        <input
          v-model="searchText"
          placeholder="搜索 OCR 文字..."
          @input="onSearch"
          class="search-input"
        />
        <button @click="clearAll" class="btn-danger" v-if="records.length">清空</button>
      </div>
    </div>
    <div class="history-grid" v-if="records.length">
      <div
        v-for="rec in records"
        :key="rec.id"
        class="history-card"
        @click="openRecord(rec)"
      >
        <img
          :src="`data:image/jpeg;base64,${rec.thumbnail_base64}`"
          class="history-thumb"
        />
        <div class="history-info">
          <span class="history-time">{{ rec.timestamp }}</span>
          <span class="history-size">{{ rec.width }}×{{ rec.height }}</span>
          <button @click.stop="deleteRecord(rec.id)" class="btn-delete">×</button>
        </div>
        <div v-if="rec.ocr_text" class="history-ocr-preview">
          {{ rec.ocr_text.slice(0, 80) }}{{ rec.ocr_text.length > 80 ? '...' : '' }}
        </div>
      </div>
    </div>
    <div v-else class="history-empty">
      <p>暂无截图历史</p>
      <p class="hint">截图后会自动保存到历史记录</p>
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
const searchText = ref("");

onMounted(() => loadHistory());

async function loadHistory() {
  try {
    records.value = await invoke<ScreenshotRecord[]>("get_history", {
      limit: 100,
      offset: 0,
      search: searchText.value || null,
    });
  } catch (e) {
    console.error("Failed to load history:", e);
  }
}

let searchTimer: ReturnType<typeof setTimeout> | null = null;
function onSearch() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(loadHistory, 300);
}

async function deleteRecord(id: number) {
  try {
    await invoke("delete_history_item", { id });
    records.value = records.value.filter((r) => r.id !== id);
  } catch (e) {
    console.error("Failed to delete:", e);
  }
}

async function clearAll() {
  try {
    await invoke("clear_history");
    records.value = [];
  } catch (e) {
    console.error("Failed to clear:", e);
  }
}

function openRecord(rec: ScreenshotRecord) {
  // Open in annotate window
  invoke("open_annotate_window", { imageBase64: rec.image_base64 });
}
</script>

<style scoped>
.history-view {
  padding: 24px;
  max-width: 1200px;
  margin: 0 auto;
  min-height: 100vh;
  background: #111827;
  color: #e5e7eb;
}

.history-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 20px;
}
.history-header h2 { font-size: 20px; font-weight: 600; }

.history-search {
  display: flex;
  gap: 8px;
  align-items: center;
}

.search-input {
  padding: 6px 12px;
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 6px;
  color: #e5e7eb;
  font-size: 13px;
  width: 240px;
}
.search-input::placeholder { color: #6b7280; }
.search-input:focus { outline: none; border-color: #3b82f6; }

.btn-danger {
  padding: 6px 12px;
  background: #991b1b;
  color: #fca5a5;
  border: none;
  border-radius: 4px;
  font-size: 12px;
  cursor: pointer;
}
.btn-danger:hover { background: #b91c1c; }

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

.history-thumb {
  width: 100%;
  height: 140px;
  object-fit: cover;
  display: block;
}

.history-info {
  display: flex;
  justify-content: space-between;
  align-items: center;
  padding: 8px 10px;
}
.history-time { font-size: 11px; color: #9ca3af; }
.history-size { font-size: 11px; color: #6b7280; }

.btn-delete {
  background: none;
  border: none;
  color: #6b7280;
  font-size: 16px;
  cursor: pointer;
  padding: 0 4px;
}
.btn-delete:hover { color: #f87171; }

.history-ocr-preview {
  padding: 0 10px 8px;
  font-size: 11px;
  color: #6b7280;
  line-height: 1.4;
}

.history-empty {
  text-align: center;
  padding: 80px 20px;
  color: #6b7280;
}
.history-empty p { margin: 4px 0; }
.hint { font-size: 13px; }
</style>
