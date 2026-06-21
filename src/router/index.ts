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
  ],
});

export default router;
