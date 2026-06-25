import { defineStore } from "pinia";
import { ref, watch } from "vue";

export interface AppConfig {
  // Annotation defaults
  defaultColor: string;
  defaultStrokeWidth: number;
  defaultFontFamily: string;
  defaultFontSize: number;
  // Mosaic
  mosaicBlockSize: number;
  // Blur
  blurRadius: number;
  // Save
  saveQuality: number; // 1-100
  saveFormat: "png" | "jpg" | "bmp";
  autoCopyToClipboard: boolean;
  // Hotkeys
  hotkeyCaptureRegion: string;
  hotkeyCaptureFullscreen: string;
  hotkeyCaptureWindow: string;
  // UI
  showMagnifier: boolean;
  pinOpacity: number; // 0.1-1.0
  theme: "dark" | "light";
  // OCR
  ocrLanguage: "ch" | "en" | "ch_en";
}

const defaultConfig: AppConfig = {
  defaultColor: "#ff4444",
  defaultStrokeWidth: 3,
  defaultFontFamily: "Microsoft YaHei",
  defaultFontSize: 20,
  mosaicBlockSize: 10,
  blurRadius: 12,
  saveQuality: 100,
  saveFormat: "png",
  autoCopyToClipboard: true,
  hotkeyCaptureRegion: "Ctrl+Shift+A",
  hotkeyCaptureFullscreen: "Ctrl+Shift+S",
  hotkeyCaptureWindow: "Ctrl+Shift+Q",
  showMagnifier: true,
  pinOpacity: 1.0,
  theme: "dark",
  ocrLanguage: "ch_en",
};

export const useConfigStore = defineStore("config", () => {
  const config = ref<AppConfig>({ ...defaultConfig });

  // Load from localStorage on init
  function loadConfig() {
    try {
      const saved = localStorage.getItem("cappix-config");
      if (saved) {
        const parsed = JSON.parse(saved);
        config.value = { ...defaultConfig, ...parsed };
      }
    } catch (e) {
      console.error("Failed to load config:", e);
    }
  }

  // Save to localStorage on change
  function saveConfig() {
    try {
      localStorage.setItem("cappix-config", JSON.stringify(config.value));
    } catch (e) {
      console.error("Failed to save config:", e);
    }
  }

  // Auto-save on config changes
  watch(config, saveConfig, { deep: true });

  // Initialize
  loadConfig();

  function updateConfig(partial: Partial<AppConfig>) {
    config.value = { ...config.value, ...partial };
  }

  function resetConfig() {
    config.value = { ...defaultConfig };
  }

  return {
    config,
    updateConfig,
    resetConfig,
    loadConfig,
  };
});
