import { createRouter, createWebHistory } from "vue-router";
import AgentsView from "@/features/agents/AgentsView.vue";
import ConnectionsView from "@/features/connections/ConnectionsView.vue";
import DocsView from "@/features/docs/DocsView.vue";

export default createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/agents" },
    { path: "/agents", name: "agents", component: AgentsView },
    { path: "/agents/:agentId", name: "agent", component: AgentsView },
    { path: "/connections", name: "connections", component: ConnectionsView },
    { path: "/docs", name: "docs", component: DocsView },
    { path: "/:pathMatch(.*)*", redirect: "/agents" },
  ],
  scrollBehavior: () => ({ top: 0 }),
});
