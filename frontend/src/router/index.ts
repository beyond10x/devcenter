import { createRouter, createWebHistory } from "vue-router";
import AgentsView from "@/features/agents/AgentsView.vue";
import ConnectionsView from "@/features/connections/ConnectionsView.vue";
import DocsView from "@/features/docs/DocsView.vue";
import PublicationsView from "@/features/publications/PublicationsView.vue";
import ProjectsView from "@/features/projects/ProjectsView.vue";

export default createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/projects" },
    { path: "/projects", name: "projects", component: ProjectsView },
    { path: "/projects/:projectId", name: "project", component: ProjectsView },
    { path: "/agents", name: "agents", component: AgentsView },
    { path: "/agents/:agentId", name: "agent", component: AgentsView },
    { path: "/connections", name: "connections", component: ConnectionsView },
    { path: "/publications", name: "publications", component: PublicationsView },
    { path: "/docs", name: "docs", component: DocsView },
    { path: "/:pathMatch(.*)*", redirect: "/projects" },
  ],
  scrollBehavior: () => ({ top: 0 }),
});
