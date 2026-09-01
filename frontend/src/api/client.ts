import type { components } from "./schema.gen";

export type Session = components["schemas"]["Session"];
export type ConnectionStatus = components["schemas"]["ConnectionStatus"];
export type ClaudeOAuthStart = components["schemas"]["ClaudeOAuthStart"];
export type Agent = components["schemas"]["Agent"];
export type Task = components["schemas"]["Task"];
export type CreateAgent = components["schemas"]["CreateAgent"];
export type IdentityProvider = components["schemas"]["IdentityProvider"];
export type PublicationState = components["schemas"]["PublicationState"];
export type Publication = components["schemas"]["Publication"];
export type ClientAuthorization = components["schemas"]["ClientAuthorization"];
export type Approval = components["schemas"]["Approval"];
export interface RepositoryCandidate {
  forge_instance_ref: string;
  project_ref: string;
  path_with_namespace: string;
  name: string;
  default_branch?: string | null;
  visibility: string;
  web_url: string;
  opened_project_id?: string | null;
}
export interface Project {
  id: string;
  forge_instance_ref: string;
  project_ref: string;
  path_with_namespace: string;
  name: string;
  default_branch?: string | null;
  selected_branch: string;
  pinned_commit?: string | null;
  default_branch_fallback: boolean;
  web_url: string;
}
export interface Branch {
  name: string;
  commit: string;
  provider_default: boolean;
  protected: boolean;
}
export interface RepositoryEntry {
  object_id: string;
  name: string;
  path: string;
  kind: "blob" | "tree";
  mode: string;
}
export interface EngineeringArtifact {
  id: string;
  locator: string;
  entity_type: string;
  revision: number;
  title?: string | null;
  status?: string | null;
  updated_at_ms: number;
  source_revision?: string | null;
}
export interface EngineeringArtifactPage {
  artifacts: EngineeringArtifact[];
  has_more: boolean;
}
export interface ProjectThread {
  id: string;
  project_id: string;
  branch: string;
  pinned_commit: string;
  title: string;
  created_at_ms: number;
}
export interface ProjectMessage {
  sequence: number;
  role: "user" | "assistant" | "system";
  content: string;
  branch: string;
  commit: string;
  created_at_ms: number;
}
export interface WorkflowDefinition {
  id: string;
  name: string;
  description: string;
}
export interface WorkflowRun {
  id: string;
  definition_id: string;
  project_id: string;
  branch: string;
  commit: string;
  state: "accepted" | "running" | "succeeded" | "failed" | "refused";
  failure_code?: string | null;
  created_at_ms: number;
}

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
  ) {
    super(code);
    this.name = "ApiError";
  }
}

async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const headers = new Headers(init.headers);
  if (init.body !== undefined) {
    headers.set("content-type", "application/json");
  }
  const response = await fetch(path, {
    ...init,
    credentials: "same-origin",
    headers,
  });
  if (!response.ok) {
    let code = `http_${String(response.status)}`;
    try {
      const problem = (await response.json()) as { code?: unknown };
      if (typeof problem.code === "string") code = problem.code;
    } catch {
      // The status remains useful when an intermediary returns a non-JSON response.
    }
    throw new ApiError(response.status, code);
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export const api = {
  session: () => request<Session>("/api/session"),
  identityProviders: () => request<IdentityProvider[]>("/api/auth/providers"),
  logout: () => request<undefined>("/auth/logout", { method: "POST" }),
  repositories: () => request<RepositoryCandidate[]>("/api/repositories"),
  openProject: (repository: Pick<RepositoryCandidate, "forge_instance_ref" | "project_ref">) =>
    request<Project>("/api/projects", {
      method: "POST",
      body: JSON.stringify(repository),
    }),
  project: (projectId: string) =>
    request<Project>(`/api/projects/${encodeURIComponent(projectId)}`),
  branches: (projectId: string) =>
    request<Branch[]>(`/api/projects/${encodeURIComponent(projectId)}/branches`),
  repositoryTree: (projectId: string) =>
    request<RepositoryEntry[]>(`/api/projects/${encodeURIComponent(projectId)}/tree`),
  engineeringArtifacts: (projectId: string) =>
    request<EngineeringArtifactPage>(
      `/api/projects/${encodeURIComponent(projectId)}/engineering-artifacts`,
    ),
  selectBranch: (projectId: string, branch: string) =>
    request<Project>(`/api/projects/${encodeURIComponent(projectId)}/branch`, {
      method: "POST",
      body: JSON.stringify({ branch }),
    }),
  threads: (projectId: string) =>
    request<ProjectThread[]>(`/api/projects/${encodeURIComponent(projectId)}/threads`),
  createThread: (projectId: string, branch: string, pinnedCommit: string, title: string) =>
    request<ProjectThread>(`/api/projects/${encodeURIComponent(projectId)}/threads`, {
      method: "POST",
      body: JSON.stringify({ branch, pinned_commit: pinnedCommit, title }),
    }),
  messages: (threadId: string) =>
    request<ProjectMessage[]>(`/api/threads/${encodeURIComponent(threadId)}/messages`),
  createMessage: (threadId: string, content: string) =>
    request<ProjectMessage>(`/api/threads/${encodeURIComponent(threadId)}/messages`, {
      method: "POST",
      body: JSON.stringify({ content }),
    }),
  workflows: (projectId: string) =>
    request<WorkflowDefinition[]>(`/api/projects/${encodeURIComponent(projectId)}/workflows`),
  startWorkflow: (projectId: string, definitionId: string, branch: string, commit: string) =>
    request<WorkflowRun>(`/api/projects/${encodeURIComponent(projectId)}/workflow-runs`, {
      method: "POST",
      body: JSON.stringify({
        definition_id: definitionId,
        branch,
        commit,
        idempotency_key: crypto.randomUUID(),
      }),
    }),
  publications: () => request<Publication[]>("/api/mcp/publications"),
  publishProfile: (profileId: string) =>
    request<Publication>("/api/mcp/publications", {
      method: "POST",
      body: JSON.stringify({ profile_id: profileId }),
    }),
  changePublicationState: (publicationId: string, state: PublicationState) =>
    request<Publication>(`/api/mcp/publications/${encodeURIComponent(publicationId)}`, {
      method: "PATCH",
      body: JSON.stringify({ state }),
    }),
  publicationClients: (publicationId: string) =>
    request<ClientAuthorization[]>(
      `/api/mcp/publications/${encodeURIComponent(publicationId)}/clients`,
    ),
  revokePublicationClient: (publicationId: string, authorizationId: string) =>
    request<undefined>(
      `/api/mcp/publications/${encodeURIComponent(publicationId)}/clients/${encodeURIComponent(authorizationId)}`,
      { method: "DELETE" },
    ),
  publicationApprovals: (publicationId: string) =>
    request<Approval[]>(`/api/mcp/publications/${encodeURIComponent(publicationId)}/approvals`),
  connection: () => request<ConnectionStatus>("/api/connectors/claude-code"),
  startOAuth: () =>
    request<ClaudeOAuthStart>("/api/connectors/claude-code/oauth/start", { method: "POST" }),
  completeOAuth: (flowId: string, code: string) =>
    request<ConnectionStatus>("/api/connectors/claude-code/oauth/complete", {
      method: "POST",
      body: JSON.stringify({ flow_id: flowId, code }),
    }),
  disconnect: () => request<ConnectionStatus>("/api/connectors/claude-code", { method: "DELETE" }),
  agents: () => request<Agent[]>("/api/agents"),
  createAgent: (agent: CreateAgent) =>
    request<Agent>("/api/agents", { method: "POST", body: JSON.stringify(agent) }),
  submitTask: (agentId: string, prompt: string) =>
    request<Task>(`/api/agents/${encodeURIComponent(agentId)}/tasks`, {
      method: "POST",
      body: JSON.stringify({ prompt, idempotency_key: crypto.randomUUID() }),
    }),
};

const FRIENDLY_ERRORS: Record<string, string> = {
  agent_platform_not_configured: "Agent Platform is not configured for this environment.",
  agent_platform_unavailable: "Agent Platform is temporarily unavailable.",
  connectors_unavailable: "The connection service is temporarily unavailable.",
  identity_access_unavailable: "Identity could not authorize this operation.",
  claude_connection_start_refused: "Claude authorization could not be started.",
  claude_connection_refused: "The authorization code was refused or expired.",
  claude_connection_code_invalid: "Enter the complete one-time authorization code.",
  agent_platform_capability_profiles_unavailable:
    "Capability profile publication is waiting for the released Agent Platform profile client.",
  workspace_not_configured: "Repository projects are not configured for this environment.",
  workspace_unavailable: "Repository projects are temporarily unavailable.",
  workspace_access_refused: "Your current GitLab grant does not admit this repository.",
  workspace_snapshot_conflict: "The branch snapshot changed. Refresh the project and try again.",
  workspace_request_refused: "The central engineering plan query was refused.",
  identity_publication_revocation_unavailable:
    "Identity cannot yet revoke every authorization for this publication safely.",
  identity_client_revocation_unavailable:
    "Identity cannot yet revoke this client authorization safely.",
  identity_authentication_required: "Your session has expired. Sign in again.",
};

export function errorMessage(error: unknown): string {
  if (error instanceof ApiError) {
    return FRIENDLY_ERRORS[error.code] ?? `The request was refused (${error.code}).`;
  }
  return "Devcenter could not reach the service. Try again.";
}

export interface TaskEventEnvelope {
  event?:
    | { kind: "accepted" }
    | { kind: "running" }
    | { kind: "text_delta"; text: string }
    | { kind: "succeeded"; output: string }
    | { kind: "failed"; failure?: { message?: string } };
}
