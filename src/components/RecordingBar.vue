<template>
  <div class="recording-bar" v-if="recordingState.is_recording">
    <div class="rec-indicator">
      <span class="rec-dot" :class="{ paused: recordingState.is_paused }"></span>
      <span class="rec-text">{{ recordingState.is_paused ? "已暂停" : "录制中" }}</span>
      <span class="rec-time">{{ formatDuration(recordingState.duration_secs) }}</span>
    </div>
    <div class="rec-actions">
      <button @click="togglePause" :title="recordingState.is_paused ? '继续' : '暂停'">
        {{ recordingState.is_paused ? "▶" : "⏸" }}
      </button>
      <button @click="stopRecording" title="停止" class="btn-stop">⏹</button>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from "vue";
import { invoke } from "@tauri-apps/api/core";

interface RecordingStateInfo {
  is_recording: boolean;
  is_paused: boolean;
  output_path: string;
  duration_secs: number;
}

const recordingState = ref<RecordingStateInfo>({
  is_recording: false,
  is_paused: false,
  output_path: "",
  duration_secs: 0,
});

let pollTimer: ReturnType<typeof setInterval> | null = null;

onMounted(async () => {
  // Poll recording state
  pollTimer = setInterval(pollState, 1000);
  await pollState();
});

onUnmounted(() => {
  if (pollTimer) clearInterval(pollTimer);
});

async function pollState() {
  try {
    const state = await invoke<RecordingStateInfo>("get_recording_state");
    recordingState.value = state;
  } catch {
    // Not recording or error
  }
}

async function togglePause() {
  try {
    if (recordingState.value.is_paused) {
      await invoke("resume_recording");
    } else {
      await invoke("pause_recording");
    }
    await pollState();
  } catch (e) {
    console.error("Pause/resume failed:", e);
  }
}

async function stopRecording() {
  try {
    const path = await invoke<string>("stop_recording");
    recordingState.value.is_recording = false;
    alert(`录制已保存: ${path}`);
  } catch (e) {
    console.error("Stop recording failed:", e);
  }
}

function formatDuration(secs: number): string {
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  return `${m.toString().padStart(2, "0")}:${s.toString().padStart(2, "0")}`;
}

defineExpose({ recordingState });
</script>

<style scoped>
.recording-bar {
  position: fixed;
  bottom: 40px;
  left: 50%;
  transform: translateX(-50%);
  background: #1f2937;
  border: 1px solid #374151;
  border-radius: 12px;
  padding: 10px 20px;
  display: flex;
  align-items: center;
  gap: 16px;
  box-shadow: 0 4px 24px rgba(0, 0, 0, 0.5);
  z-index: 10000;
}

.rec-indicator {
  display: flex;
  align-items: center;
  gap: 8px;
}

.rec-dot {
  width: 10px;
  height: 10px;
  background: #ef4444;
  border-radius: 50%;
  animation: pulse 1s ease-in-out infinite;
}
.rec-dot.paused {
  background: #f59e0b;
  animation: none;
}

@keyframes pulse {
  0%, 100% { opacity: 1; }
  50% { opacity: 0.4; }
}

.rec-text {
  color: #e5e7eb;
  font-size: 13px;
  font-weight: 600;
}

.rec-time {
  color: #9ca3af;
  font-size: 14px;
  font-variant-numeric: tabular-nums;
}

.rec-actions {
  display: flex;
  gap: 8px;
}

.rec-actions button {
  background: #374151;
  color: #e5e7eb;
  border: none;
  width: 32px;
  height: 32px;
  border-radius: 6px;
  font-size: 14px;
  cursor: pointer;
  display: flex;
  align-items: center;
  justify-content: center;
}
.rec-actions button:hover { background: #4b5563; }

.btn-stop { background: #dc2626 !important; }
.btn-stop:hover { background: #b91c1c !important; }
</style>
