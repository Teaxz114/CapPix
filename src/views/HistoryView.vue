<template>
  <div class="history-view">
    <div class="history-header">
      <h2>截图历史</h2>
      <div class="history-actions">
        <input v-model="searchQuery" placeholder="搜索 OCR 文字..." class="search-input" @input="onSearch" />
        <button @click="loadHistory" class="btn-refresh">刷新</button>
        <button @click="clearAll" class="btn-danger">清空</button>
      </div>
    </div>
    <div class="history-count">共 {{ totalCount }} 条记录</div>
    <div class="history-list" v-if="entries.length">
      <div v-for="entry in entries" :key="entry.id" class="history-item" @click="openEntry(entry)">
        <img :src="thumbnails[entry.id] || ''" class="history-thumb" />
        <div class="history-info">
          <div class="history-meta">
            <span class="history-source">{{ sourceLabel(entry.source) }}</span>
            <span class="history-size">{{ entry.width }}×{{ entry.height }}</span>
            <span class="history-time">{{ entry.timestamp }}</span>
          </div>
          <div v-if="entry.ocr_text" class="history-ocr">{{ truncate(entry.ocr_text, 80) }}</div>
        </div>
        <button class="btn-delete" @click.stop="deleteEntry(entry.id)">×</button>
      </div>
    </div>
    <div v-else class="history-empty">暂无截图记录</div>
    <div v-if="entries.length && hasMore" class="history-loadmore">
      <button @click="loadMore" :disabled="loadingMore" class="btn-loadmore">
        {{ loadingMore ? "加载中..." : "加载更多" }}
      </button>
      <span class="history-count-inline">已显示 {{ entries.length }} / {{ totalCount }} 条</span>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface ScreenshotRecord {
  id: number;
  timestamp: string;
  image_path: string;
  width: number;
  height: number;
  source: string;
  ocr_text: string | null;
}

const entries = ref<ScreenshotRecord[]>([]);
const thumbnails = ref<Record<number, string>>({}); // id -> base64 data URL
const totalCount = ref(0);
const searchQuery = ref("");
const hasMore = ref(true);
const loadingMore = ref(false);

onMounted(() => { loadHistory(); });

async function loadHistory() {
  try {
    entries.value = await invoke<ScreenshotRecord[]>("history_list", { limit: 50, offset: 0 });
    totalCount.value = await invoke<number>("history_count");
    hasMore.value = entries.value.length < totalCount.value;
    // Load thumbnails on demand (only visible items)
    for (const entry of entries.value) {
      loadThumbnail(entry.id, entry.image_path);
    }
  } catch (e) { console.error("Failed to load history:", e); }
}

async function loadMore() {
  if (loadingMore.value || !hasMore.value) return;
  loadingMore.value = true;
  try {
    const more = await invoke<ScreenshotRecord[]>("history_list", { limit: 50, offset: entries.value.length });
    entries.value.push(...more);
    hasMore.value = more.length === 50 && entries.value.length < totalCount.value;
    for (const entry of more) {
      loadThumbnail(entry.id, entry.image_path);
    }
  } catch (e) { console.error("Failed to load more:", e); }
  loadingMore.value = false;
}

async function loadThumbnail(id: number, imagePath: string) {
  if (thumbnails.value[id]) return;
  try {
    const base64 = await invoke<string>("get_screenshot_thumbnail", { imagePath });
    thumbnails.value[id] = `data:image/png;base64,${base64}`;
  } catch (_e) {
    // Fallback to full image if thumbnail not available
    try {
      const base64 = await invoke<string>("get_screenshot_image", { imagePath });
      thumbnails.value[id] = `data:image/png;base64,${base64}`;
    } catch (e) { console.error("Failed to load thumbnail:", id, e); }
  }
}

async function onSearch() {
  if (!searchQuery.value.trim()) { loadHistory(); return; }
  try {
    entries.value = await invoke<ScreenshotRecord[]>("history_search", { query: searchQuery.value, limit: 50 });
  } catch (e) { console.error("Search failed:", e); }
}

async function deleteEntry(id: number) {
  try {
    await invoke("history_delete", { id });
    loadHistory();
  } catch (e) { console.error("Delete failed:", e); }
}

async function clearAll() {
  if (!confirm("确定清空所有历史记录？")) return;
  try {
    await invoke("history_clear");
    loadHistory();
  } catch (e) { console.error("Clear failed:", e); }
}

async function openEntry(entry: ScreenshotRecord) {
  // Load image on demand, then open in annotate window
  try {
    const base64 = await invoke<string>("get_screenshot_image", { imagePath: entry.image_path });
    invoke("open_annotate_window", { imageBase64: base64 });
  } catch (e) { console.error("Failed to open entry:", e); }
}

function sourceLabel(s: string) {
  const map: Record<string, string> = { region: "区域", fullscreen: "全屏", window: "窗口" };
  return map[s] || s;
}

function truncate(s: string, n: number) {
  return s.length > n ? s.slice(0, n) + "..." : s;
}
</script>

<style scoped>
.history-view {
  padding: 20px;
  max-width: 800px;
  margin: 0 auto;
  color: #e5e7eb;
}
.history-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 16px;
}
.history-header h2 { font-size: 18px; margin: 0; }
.history-actions { display: flex; gap: 8px; align-items: center; }
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
.btn-refresh, .btn-danger {
  background: #374151;
  color: #e5e7eb;
  border: none;
  padding: 6px 14px;
  border-radius: 6px;
  font-size: 12px;
  cursor: pointer;
}
.btn-danger { background: #7f1d1d; }
.btn-danger:hover { background: #991b1b; }
.history-count { color: #6b7280; font-size: 12px; margin-bottom: 12px; }
.history-list { display: flex; flex-direction: column; gap: 8px; }
.history-item {
  display: flex;
  align-items: center;
  gap: 12px;
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 8px;
  padding: 10px;
  cursor: pointer;
  transition: background 0.15s;
}
.history-item:hover { background: #374151; }
.history-thumb {
  width: 80px;
  height: 60px;
  object-fit: cover;
  border-radius: 4px;
  background: #111827;
}
.history-info { flex: 1; min-width: 0; }
.history-meta { display: flex; gap: 8px; font-size: 11px; color: #6b7280; margin-bottom: 4px; }
.history-source { background: #374151; padding: 1px 6px; border-radius: 3px; }
.history-ocr { font-size: 12px; color: #9ca3af; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.btn-delete {
  background: none;
  border: none;
  color: #6b7280;
  font-size: 18px;
  cursor: pointer;
  padding: 4px 8px;
}
.btn-delete:hover { color: #ef4444; }
.history-empty { text-align: center; color: #6b7280; padding: 40px; }
.history-loadmore { text-align: center; padding: 16px; }
.btn-loadmore { background: #374151; color: #d1d5db; border: none; padding: 8px 24px; border-radius: 6px; cursor: pointer; font-size: 13px; }
.btn-loadmore:hover { background: #4b5563; }
.btn-loadmore:disabled { opacity: 0.5; cursor: not-allowed; }
.history-count-inline { color: #6b7280; font-size: 12px; margin-left: 12px; }
</style>
