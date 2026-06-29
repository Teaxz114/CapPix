import { createApp } from "vue";
import { createPinia } from "pinia";
import router from "./router";
import App from "./App.vue";
import "./style.css";

// Global error handler — prevent unhandled promise rejections from crashing silently
window.addEventListener("unhandledrejection", (event) => {
  console.error("[CapPix] Unhandled promise rejection:", event.reason);
});

const vueApp = createApp(App);
vueApp.config.errorHandler = (err, _instance, info) => {
  console.error(`[CapPix] Vue error (${info}):`, err);
};
vueApp.use(createPinia());
vueApp.use(router);
vueApp.mount("#app");
