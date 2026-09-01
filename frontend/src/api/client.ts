import type { components } from "./schema.gen";

export type Session = components["schemas"]["Session"];
export type ConnectionStatus = components["schemas"]["ConnectionStatus"];
export type ClaudeOAuthStart = components["schemas"]["ClaudeOAuthStart"];
export type Agent = components["schemas"]["Agent"];
export type Task = components["schemas"]["Task"];
export type CreateAgent = components["schemas"]["CreateAgent"];
export interface IdentityProvider {
  id: string;
  display_name: string;
}
export type PublicationState = "active" | "suspended" | "revoked";
export interface Publication {
  publication_id: string;
  tenant_id: string;
  owner_subject: string;
  profile_id: string;
  active_revision: number;
  toolset_digest: string;
  state: PublicationState;
  created_at_ms: number;
  updated_at_ms: number;
}
export interface ClientAuthorization {
  authorization_id: string;
  publication_id: string;
  subject: string;
  client_id: string;
  display_name: string;
  state: "active" | "revoked";
  first_used_at_ms: number;
  last_used_at_ms: number;
}
export interface Approval {
  approval_id: string;
  publication_id: string;
  authorization_id: string;
  subject: string;
  client_id: string;
  tool_name: string;
  operation_ref: string;
  connection_id: string;
  input_digest: string;
  state: "pending" | "approved" | "denied" | "consumed" | "expired" | "outcome_unknown";
  expires_at_ms: number;
  audit_ref?: string | null;
  created_at_ms: number;
  updated_at_ms: number;
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
  identity_publication_revocation_unavailable:
    "Identity cannot yet revoke every authorization for this publication safely.",
  identity_client_revocation_unavailable:
    "Identity cannot yet revoke this client authorization safely.",
  authentication_required: "Your session has expired. Sign in again.",
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
