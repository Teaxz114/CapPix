import { createRouter, createWebHashHistory } from "vue-router";
import HomeView from "../views/HomeView.vue";

const router = createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "home", component: HomeView },
    {
      path: "/annotate",
      name: "annotate",
      component: () => import("../views/AnnotateView.vue"),
    },
    {
      path: "/pin",
      name: "pin",
      component: () => import("../views/PinView.vue"),
    },
  ],
});

export default router;
