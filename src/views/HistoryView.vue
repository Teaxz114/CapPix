<template>
  <div class="history-view">
    <div class="history-header">
      <h2>截图历史</h2>
      <div class="history-actions">
        <input
          v-model="searchQuery"
          class="search-input"
          placeholder="搜索..."
          @input="onSearch"
        />
        <button class="btn-danger" @click="clearAll">清空全部</button>
      </div>
    </div>
    <div v-if="loading" class="loading">加载中...</div>
    <div v-else-if="items.length === 0" class="empty">
      <p>暂无截图历史</p>
      <p class="hint">按 Ctrl+Shift+A 截图后会自动保存</p>
    </div>
    <div v-else class="history-grid">
      <div
        v-for="item in filteredItems"
        :key="item.id"
        class="history-card"
        @click="openItem(item)"
      >
        <img
          v-if="item.thumbnail"
          :src="`data:image/png;base64,${item.thumbnail}`"
          class="card-thumb"
        />
        <div class="card-info">
          <span class="card-date">{{ formatDate(item.created_at) }}</span>
          <span v-if="item.ocr_text" class="card-ocr-badge">OCR</span>
        </div>
        <button class="card-delete" @click.stop="deleteItem(item.id)">×</button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface HistoryItem {
  id: number;
  image_base64: string;
  thumbnail: string;
  created_at: string;
  width: number;
  height: number;
  ocr_text: string | null;
}

const items = ref<HistoryItem[]>([]);
const loading = ref(true);
const searchQuery = ref("");

const filteredItems = computed(() => {
  if (!searchQuery.value) return items.value;
  const q = searchQuery.value.toLowerCase();
  return items.value.filter(
    (item) =>
      item.ocr_text?.toLowerCase().includes(q) ||
      formatDate(item.created_at).includes(q)
  );
});

onMounted(async () => {
  await loadHistory();
});

async function loadHistory() {
  loading.value = true;
  try {
    items.value = await invoke<HistoryItem[]>("get_history", { limit: 100, offset: 0 });
  } catch (e) {
    console.error("Failed to load history:", e);
  } finally {
    loading.value = false;
  }
}

async function deleteItem(id: number) {
  try {
    await invoke("delete_history_item", { id });
    items.value = items.value.filter((i) => i.id !== id);
  } catch (e) {
    console.error("Failed to delete:", e);
  }
}

async function clearAll() {
  if (!confirm("确定清空全部历史？")) return;
  try {
    await invoke("clear_history");
    items.value = [];
  } catch (e) {
    console.error("Failed to clear:", e);
  }
}

function openItem(item: HistoryItem) {
  // TODO: open in annotate view with the image
  console.log("Open item:", item.id);
}

function formatDate(dateStr: string): string {
  try {
    const d = new Date(dateStr);
    return d.toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  } catch {
    return dateStr;
  }
}

let searchTimer: ReturnType<typeof setTimeout> | null = null;
function onSearch() {
  if (searchTimer) clearTimeout(searchTimer);
  searchTimer = setTimeout(() => {
    // Reactive via computed
  }, 200);
}
</script>

<style scoped>
.history-view {
  padding: 24px;
  max-width: 960px;
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
  font-size: 18px;
  font-weight: 600;
  margin: 0;
}

.history-actions {
  display: flex;
  gap: 8px;
  align-items: center;
}

.search-input {
  background: #374151;
  border: 1px solid #4b5563;
  color: #e5e7eb;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 13px;
  width: 180px;
  outline: none;
}
.search-input:focus { border-color: #3b82f6; }

.btn-danger {
  background: #dc2626;
  color: #fff;
  border: none;
  padding: 6px 12px;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
}
.btn-danger:hover { background: #b91c1c; }

.loading, .empty {
  text-align: center;
  padding: 60px 20px;
  color: #6b7280;
}

.hint { font-size: 13px; margin-top: 8px; }

.history-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
  gap: 12px;
}

.history-card {
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  overflow: hidden;
  cursor: pointer;
  transition: border-color 0.15s;
  position: relative;
}
.history-card:hover { border-color: #3b82f6; }

.card-thumb {
  width: 100%;
  height: 120px;
  object-fit: cover;
  display: block;
}

.card-info {
  padding: 8px 10px;
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.card-date { font-size: 11px; color: #9ca3af; }

.card-ocr-badge {
  background: #3b82f6;
  color: #fff;
  font-size: 9px;
  padding: 1px 5px;
  border-radius: 3px;
}

.card-delete {
  position: absolute;
  top: 4px;
  right: 4px;
  background: rgba(0,0,0,0.6);
  color: #9ca3af;
  border: none;
  width: 22px;
  height: 22px;
  border-radius: 50%;
  font-size: 14px;
  cursor: pointer;
  opacity: 0;
  transition: opacity 0.15s;
  display: flex;
  align-items: center;
  justify-content: center;
}
.history-card:hover .card-delete { opacity: 1; }
.card-delete:hover { color: #ef4444; }
</style>
