import { createRouter, createWebHashHistory } from "vue-router";
import BillsView from "./views/BillsView.vue";
import StatsView from "./views/StatsView.vue";
import CategoriesView from "./views/CategoriesView.vue";
import SettingsView from "./views/SettingsView.vue";

export default createRouter({
  history: createWebHashHistory(),
  routes: [
    { path: "/", name: "bills", component: BillsView },
    { path: "/stats", name: "stats", component: StatsView },
    { path: "/categories", name: "categories", component: CategoriesView },
    { path: "/settings", name: "settings", component: SettingsView },
  ],
});
