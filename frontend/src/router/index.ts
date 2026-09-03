import { createRouter, createWebHistory } from "vue-router";
import AgentsView from "@/features/agents/AgentsView.vue";
import ConnectorsView from "@/features/connectors/ConnectorsView.vue";
import DocsView from "@/features/docs/DocsView.vue";
import PublicationsView from "@/features/publications/PublicationsView.vue";
import ProjectsView from "@/features/projects/ProjectsView.vue";
import ProfilesView from "@/features/profiles/ProfilesView.vue";
import ServicesView from "@/features/services/ServicesView.vue";
import WorkflowsView from "@/features/workflows/WorkflowsView.vue";

export default createRouter({
  history: createWebHistory(),
  routes: [
    { path: "/", redirect: "/projects" },
    { path: "/projects", name: "projects", component: ProjectsView },
    { path: "/projects/:projectId", name: "project", component: ProjectsView },
    {
      path: "/projects/:projectId/sessions/:sessionId",
      name: "coding-session",
      component: () => import("@/features/workbench/HostedWorkspaceView.vue"),
    },
    { path: "/agents", name: "agents", component: AgentsView },
    { path: "/agents/:agentId", name: "agent", component: AgentsView },
    { path: "/connectors", name: "connectors", component: ConnectorsView },
    { path: "/connectors/:providerRef", name: "connector", component: ConnectorsView },
    { path: "/services", name: "services", component: ServicesView },
    { path: "/workflows", name: "workflows", component: WorkflowsView },
    { path: "/workflows/:workflowId", name: "workflow", component: WorkflowsView },
    {
      path: "/connections",
      redirect: (to) => ({ path: "/connectors", query: { ...to.query, tab: "connections" } }),
    },
    { path: "/profiles", name: "profiles", component: ProfilesView },
    { path: "/publications", name: "publications", component: PublicationsView },
    { path: "/docs", name: "docs", component: DocsView },
    { path: "/:pathMatch(.*)*", redirect: "/projects" },
  ],
  scrollBehavior: () => ({ top: 0 }),
});
