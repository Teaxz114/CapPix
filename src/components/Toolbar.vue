<template>
  <div class="toolbar">
    <!-- Tool buttons -->
    <div class="toolbar-section">
      <button
        v-for="tool in tools"
        :key="tool.id"
        :class="['tool-btn', { active: currentTool === tool.id }]"
        :title="tool.name"
        @click="$emit('tool-change', tool.id)"
      >
        <span class="tool-icon" v-html="tool.icon"></span>
        <span class="tool-label">{{ tool.name }}</span>
      </button>
    </div>

    <div class="toolbar-divider"></div>

    <!-- Color picker -->
    <div class="toolbar-section">
      <div class="color-grid">
        <button
          v-for="color in presetColors"
          :key="color"
          :class="['color-btn', { active: currentColor === color }]"
          :style="{ background: color }"
          :title="color"
          @click="$emit('style-change', { color })"
        ></button>
        <label class="color-btn custom-color" title="自定义颜色">
          <input
            type="color"
            :value="currentColor"
            @input="$emit('style-change', { color: ($event.target as HTMLInputElement).value })"
          />
          <span class="custom-color-icon">+</span>
        </label>
      </div>
    </div>

    <div class="toolbar-divider"></div>

    <!-- Stroke width -->
    <div class="toolbar-section">
      <label class="stroke-label">粗细</label>
      <input
        type="range"
        min="1"
        max="20"
        :value="currentStrokeWidth"
        class="stroke-slider"
        @input="$emit('style-change', { strokeWidth: Number(($event.target as HTMLInputElement).value) })"
      />
      <span class="stroke-value">{{ currentStrokeWidth }}px</span>
    </div>

    <div class="toolbar-divider"></div>

    <!-- Undo/Redo -->
    <div class="toolbar-section">
      <button
        class="action-btn"
        title="撤销 (Ctrl+Z)"
        :disabled="!canUndo"
        @click="$emit('undo')"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M3 10h10a5 5 0 0 1 0 10H9" /><path d="M3 10l4-4" /><path d="M3 10l4 4" />
        </svg>
      </button>
      <button
        class="action-btn"
        title="重做 (Ctrl+Y)"
        :disabled="!canRedo"
        @click="$emit('redo')"
      >
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M21 10H11a5 5 0 0 0 0 10h4" /><path d="M21 10l-4-4" /><path d="M21 10l-4 4" />
        </svg>
      </button>
    </div>

    <div class="toolbar-divider"></div>

    <!-- Save/Copy/Pin -->
    <div class="toolbar-section">
      <button class="action-btn" title="保存到文件" @click="$emit('save')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M19 21H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h11l5 5v11a2 2 0 0 1-2 2z" />
          <polyline points="17 21 17 13 7 13 7 21" /><polyline points="7 3 7 8 15 8" />
        </svg>
        <span class="action-label">保存</span>
      </button>
      <button class="action-btn" title="复制到剪贴板" @click="$emit('copy')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="9" y="9" width="13" height="13" rx="2" ry="2" />
          <path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1" />
        </svg>
        <span class="action-label">复制</span>
      </button>
      <button class="action-btn" title="贴图到桌面" @click="$emit('pin')">
        <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <path d="M12 2L2 7l10 5 10-5-10-5z" /><path d="M2 17l10 5 10-5" />
          <path d="M2 12l10 5 10-5" />
        </svg>
        <span class="action-label">贴图</span>
      </button>
    </div>
  </div>
</template>

<script setup lang="ts">
defineProps<{
  currentTool: string;
  currentColor: string;
  currentStrokeWidth: number;
  canUndo: boolean;
  canRedo: boolean;
}>();

defineEmits<{
  "tool-change": [tool: string];
  "style-change": [style: { color?: string; strokeWidth?: number }];
  undo: [];
  redo: [];
  save: [];
  copy: [];
  pin: [];
}>();

const tools = [
  {
    id: "rect",
    name: "矩形",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/></svg>',
  },
  {
    id: "ellipse",
    name: "椭圆",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><ellipse cx="12" cy="12" rx="10" ry="8"/></svg>',
  },
  {
    id: "arrow",
    name: "箭头",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="12" x2="19" y2="12"/><polyline points="12 5 19 12 12 19"/></svg>',
  },
  {
    id: "line",
    name: "直线",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="5" y1="19" x2="19" y2="5"/></svg>',
  },
  {
    id: "text",
    name: "文字",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="4 7 4 4 20 4 20 7"/><line x1="9" y1="20" x2="15" y2="20"/><line x1="12" y1="4" x2="12" y2="20"/></svg>',
  },
  {
    id: "pencil",
    name: "画笔",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M17 3a2.828 2.828 0 1 1 4 4L7.5 20.5 2 22l1.5-5.5L17 3z"/></svg>',
  },
  {
    id: "mosaic",
    name: "马赛克",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="7" height="7"/><rect x="14" y="3" width="7" height="7"/><rect x="3" y="14" width="7" height="7"/><rect x="14" y="14" width="7" height="7"/></svg>',
  },
  {
    id: "highlighter",
    name: "荧光笔",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M18 2l4 4-12 12H6v-4L18 2z"/><rect x="2" y="20" width="10" height="2" fill="currentColor" opacity="0.3"/></svg>',
  },
  {
    id: "number",
    name: "序号",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><text x="12" y="16" text-anchor="middle" font-size="12" fill="currentColor" stroke="none">1</text></svg>',
  },
  {
    id: "eraser",
    name: "橡皮擦",
    icon: '<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 20H7L3 16l9-9 8 8-4 4z"/><path d="M6 11l8 8"/></svg>',
  },
];

const presetColors = [
  "#ff4444",
  "#ff8800",
  "#ffcc00",
  "#44cc44",
  "#4488ff",
  "#8844ff",
  "#ff44aa",
  "#ffffff",
  "#000000",
];
</script>

<style scoped>
.toolbar {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 6px 12px;
  background: #1f2937;
  border-bottom: 1px solid #374151;
  flex-wrap: wrap;
  min-height: 44px;
}

.toolbar-section {
  display: flex;
  align-items: center;
  gap: 2px;
}

.toolbar-divider {
  width: 1px;
  height: 28px;
  background: #374151;
  margin: 0 6px;
}

.tool-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: 4px;
  background: transparent;
  color: #9ca3af;
  font-size: 12px;
  transition: all 0.15s;
  border: 1px solid transparent;
}

.tool-btn:hover {
  background: #374151;
  color: #e5e7eb;
}

.tool-btn.active {
  background: #3b82f6;
  color: #ffffff;
  border-color: #60a5fa;
}

.tool-icon {
  display: flex;
  align-items: center;
}

.tool-icon :deep(svg) {
  width: 16px;
  height: 16px;
}

.tool-label {
  font-size: 11px;
  white-space: nowrap;
}

.color-grid {
  display: flex;
  gap: 3px;
  align-items: center;
}

.color-btn {
  width: 20px;
  height: 20px;
  border-radius: 3px;
  border: 2px solid transparent;
  cursor: pointer;
  transition: border-color 0.15s;
  padding: 0;
}

.color-btn:hover {
  border-color: #9ca3af;
}

.color-btn.active {
  border-color: #60a5fa;
  box-shadow: 0 0 0 1px #3b82f6;
}

.custom-color {
  position: relative;
  background: conic-gradient(red, yellow, lime, aqua, blue, magenta, red);
  display: flex;
  align-items: center;
  justify-content: center;
  overflow: hidden;
}

.custom-color input {
  position: absolute;
  width: 100%;
  height: 100%;
  opacity: 0;
  cursor: pointer;
}

.custom-color-icon {
  color: white;
  font-size: 12px;
  font-weight: bold;
  text-shadow: 0 0 2px rgba(0,0,0,0.8);
  pointer-events: none;
}

.stroke-label {
  color: #9ca3af;
  font-size: 11px;
  margin-right: 4px;
  white-space: nowrap;
}

.stroke-slider {
  width: 60px;
  height: 4px;
  -webkit-appearance: none;
  appearance: none;
  background: #374151;
  border-radius: 2px;
  outline: none;
}

.stroke-slider::-webkit-slider-thumb {
  -webkit-appearance: none;
  appearance: none;
  width: 14px;
  height: 14px;
  border-radius: 50%;
  background: #3b82f6;
  cursor: pointer;
}

.stroke-value {
  color: #9ca3af;
  font-size: 11px;
  min-width: 30px;
  text-align: center;
  font-family: monospace;
}

.action-btn {
  display: flex;
  align-items: center;
  gap: 4px;
  padding: 4px 8px;
  border-radius: 4px;
  background: transparent;
  color: #9ca3af;
  font-size: 12px;
  transition: all 0.15s;
}

.action-btn:hover:not(:disabled) {
  background: #374151;
  color: #e5e7eb;
}

.action-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

.action-label {
  font-size: 11px;
  white-space: nowrap;
}
</style>
