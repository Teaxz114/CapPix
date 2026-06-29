import { createRouter, createWebHashHistory } from "vue-router";
import HomeView from "../views/HomeView.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: HomeView },
    {
      path: "/screenshot",
      name: "screenshot",
      component: () => import("../views/ScreenshotView.vue"),
    },
    {
      path: "/annotate",
      name: "annotate",
      component: () => import("../views/AnnotateView.vue"),
    },
    {
      path: "/history",
      name: "history",
      component: () => import("../views/HistoryView.vue"),
    },
    {
      path: "/settings",
      name: "settings",
      component: () => import("../views/SettingsView.vue"),
    },
    {
      path: "/pin",
      redirect: "/",
    },
  ],
});

export default router;
