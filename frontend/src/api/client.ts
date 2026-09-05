import type { components } from "./schema.gen";
import { assertServiceCatalog, type ServiceCatalog } from "@b10x/service-console-vue";

export type Session = components["schemas"]["Session"];
export type ConnectionStatus = components["schemas"]["ConnectionStatus"];
export type ClaudeOAuthStart = components["schemas"]["ClaudeOAuthStart"];
export type Agent = components["schemas"]["Agent"];
export type Task = components["schemas"]["Task"];
export type TaskApproval = components["schemas"]["TaskApproval"];
export type TaskApprovalDecision = components["schemas"]["TaskApprovalDecision"];
export type CreateAgent = components["schemas"]["CreateAgent"];
export type IdentityProvider = components["schemas"]["IdentityProvider"];
export type PublicationState = components["schemas"]["PublicationState"];
export type Publication = components["schemas"]["Publication"];
export type ClientAuthorization = components["schemas"]["ClientAuthorization"];
export type Approval = components["schemas"]["Approval"];
export type ConnectorCatalogPage = components["schemas"]["ConnectorCatalogPage"];
export type ConnectorCatalogOperation = components["schemas"]["ConnectorCatalogOperation"];
export type ConnectorProviderDescription = components["schemas"]["ConnectorProviderDescription"];
export type ConnectorProviderSummary = components["schemas"]["ConnectorProviderSummary"];
export type ConnectorSetupProfile = components["schemas"]["ConnectorSetupProfile"];
export type GeneratedServiceSummary = components["schemas"]["GeneratedServiceSummary"];
export type GeneratedServicePage = components["schemas"]["GeneratedServicePage"];
export type WorkflowLibrarySummary = components["schemas"]["WorkflowLibrarySummary"];
export type WorkflowDraftSummary = components["schemas"]["WorkflowDraftSummary"];
export type WorkflowRevisionSummary = components["schemas"]["WorkflowRevisionSummary"];
export type WorkflowLibraryPage = components["schemas"]["WorkflowLibraryPage"];
export type WorkflowLibraryDetail = components["schemas"]["WorkflowLibraryDetail"];
export type { ServiceCatalog };
export interface GeneratedServiceInvocation {
  output: unknown;
  connector_audit_ref: string;
}
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
  output?: string | null;
  created_at_ms: number;
}
export type CodingSessionState =
  "preparing" | "ready" | "refused" | "unknown" | "closing" | "closed";
export interface CodingSession {
  id: string;
  project_id: string;
  source_revision: string;
  materialization_ref?: string | null;
  manifest_sha256?: string | null;
  state: CodingSessionState;
  failure_code?: string | null;
  limits: { max_files: number; max_total_bytes: number; max_file_bytes: number };
  created_at_ms: number;
  updated_at_ms: number;
  coordination?: CoordinationSummary;
}
export interface CoordinationSummary {
  state: "ready" | "degraded" | "closed";
  through_version?: number | null;
  failure_code?: string | null;
  retryable: boolean;
}
export interface AgentIdeSessionSnapshot {
  session_id: string;
  workspace_session_id?: string | null;
  project_id?: string | null;
  source_revision?: string | null;
  manifest_digest?: string | null;
  objective: string;
  state: "Active" | "Closed";
}
export interface AgentIdeGrantSnapshot {
  grant_id: string;
  grantee: string;
  allowed_intents: string[];
  path_prefixes: string[];
  maximum_risk: "Low" | "Medium";
  expires_at?: string | null;
  revision: number;
  state: "Active" | "Revoked";
}
export interface AgentIdeContextPinSnapshot {
  pin_id: string;
  kind: string;
  reference: string;
  start_line?: number | null;
  end_line?: number | null;
  sha256: string;
  state: "Active" | "Removed";
}
export interface AgentIdeCheckpointSnapshot {
  checkpoint_id: string;
  attempt_ref: string;
  plan_digest: string;
  state: "Pending" | "Approved" | "Denied";
}
export interface CodingCoordinationView {
  summary: CoordinationSummary;
  session: AgentIdeSessionSnapshot;
  grants: AgentIdeGrantSnapshot[];
  pins: AgentIdeContextPinSnapshot[];
  checkpoints: AgentIdeCheckpointSnapshot[];
}
export type AgentIdeWorkbenchPaneKind =
  "Editor" | "Diff" | "Terminal" | "Chat" | "Timeline" | "Agents" | "Approvals" | "Evidence";
export interface AgentIdeWorkbenchPane {
  id: string;
  kind: AgentIdeWorkbenchPaneKind;
  title: string;
  path?: string | null;
  line?: number | null;
  column?: number | null;
}
export interface AgentIdeWorkbenchState {
  panes: AgentIdeWorkbenchPane[];
  focused_pane?: string | null;
  open_files: string[];
}
export interface AgentIdeWorkbenchView extends AgentIdeWorkbenchState {
  session_id: string;
  through_version: number;
}
export type AgentIdeWorkbenchAction =
  | { kind: "initialize" }
  | { kind: "open_file"; path: string; line?: number | null; pane_id?: string | null }
  | { kind: "close_file"; path: string }
  | {
      kind: "open_pane";
      pane_id: string;
      pane_kind: AgentIdeWorkbenchPaneKind;
      split?: "Horizontal" | "Vertical" | null;
    }
  | { kind: "close_pane"; pane_id: string }
  | { kind: "focus_pane"; pane_id: string }
  | { kind: "move_cursor"; pane_id: string; path: string; line: number; column: number }
  | { kind: "show_diff"; pane_id?: string | null; path?: string | null; base?: string | null };
export interface MutateAgentIdeWorkbench extends AgentIdeWorkbenchState {
  action: AgentIdeWorkbenchAction;
  idempotency_key: string;
}
export interface CodingTreeEntry {
  path: string;
  kind: string;
  size?: number | null;
  sha256?: string | null;
}
export interface CodingTreeProjection {
  format: string;
  root: string;
  entries: CodingTreeEntry[];
  next_cursor?: string | null;
  truncated: boolean;
  omitted?: number | null;
}
export type FileModificationState = "unchanged" | "added" | "modified";
export interface FileRevision {
  path: string;
  sha256: string;
  size: number;
  language?: string | null;
  modification: FileModificationState;
}
export interface FileProjection {
  format: string;
  revision: FileRevision;
  content?: string | null;
  binary: boolean;
  truncated: boolean;
}
export interface FileConflict {
  code: string;
  base?: FileProjection | null;
  latest: FileProjection;
}
export type ChangeSelector =
  | { kind: "workspace" }
  | { kind: "plan"; digest: string }
  | { kind: "agent_attempt"; attempt_id: string }
  | { kind: "publication"; publication_id: string }
  | { kind: "revision_pair"; old: string; new: string };
export interface ConversationMessage {
  role: "user" | "assistant" | "system";
  content: string;
}
export interface CodingTurnSelection {
  format: "agentide.context-selection-draft/1";
  id: string;
  kind: "editor" | "diff_hunk" | "terminal" | "process" | "evidence";
  reference: string;
  start_line?: number | null;
  end_line?: number | null;
  content: string;
  sha256: string;
  truncated: boolean;
}
export interface CodingOpenFileReference {
  path: string;
  sha256: string;
  cursor?: { line: number; column: number } | null;
  dirty: boolean;
}
export interface SubmitCodingTurn {
  prompt: string;
  messages: ConversationMessage[];
  focused_selections: CodingTurnSelection[];
  open_files: CodingOpenFileReference[];
  active_diff?: ChangeSelector | null;
  idempotency_key: string;
}
export type DiffMode = "patch" | "stat" | "files_only";
export interface DiffLine {
  kind: string;
  old_line?: number | null;
  new_line?: number | null;
  content: string;
}
export interface DiffHunk {
  id: string;
  old: { start: number; lines: number };
  new: { start: number; lines: number };
  heading?: string | null;
  lines: DiffLine[];
}
export interface DiffFile {
  old_path?: string | null;
  new_path?: string | null;
  status: string;
  additions?: number | null;
  deletions?: number | null;
  old_sha256?: string | null;
  new_sha256?: string | null;
  hunks: DiffHunk[];
  attribution: string[];
}
export interface DiffProjection {
  format: string;
  selector: ChangeSelector;
  mode: DiffMode;
  digest: string;
  source_revision: string;
  files: DiffFile[];
  additions: number;
  deletions: number;
  partial: boolean;
}
export type TerminalWorkspaceAccess = "read_only" | "read_write";
export interface TerminalProfile {
  id: string;
  label: string;
  runtime_ref: string;
  shell: string;
  arguments: string[];
  working_directory: string;
  environment: Record<string, string>;
  workspace_access: TerminalWorkspaceAccess;
  network: "none";
  limits: {
    timeout_ms: number;
    cpu_millis: number;
    memory_bytes: number;
    processes: number;
    output_bytes: number;
    input_bytes: number;
    frame_bytes: number;
    queued_frames: number;
    lease_ttl_ms: number;
  };
}
export type TerminalState =
  "preparing" | "running" | "exited" | "terminated" | "refused" | "unknown";
export interface TerminalSession {
  id: string;
  coding_session_id: string;
  agentide_session_id: string;
  authority_grant_id: string;
  profile: TerminalProfile;
  actor: string;
  process_id?: string | null;
  state: TerminalState;
  exit?: { code?: number | null; signal?: string | null } | null;
  failure_code?: string | null;
  created_at_ms: number;
  updated_at_ms: number;
}
export type CapabilityPosture = "allow" | "approval_required" | "deny";
export interface CapabilityConnection {
  connection_ref: string;
  label: string;
  provider: string;
  audiences: string[];
  purpose?: string | null;
}
export interface Capability {
  operation_ref: string;
  title: string;
  effect: "read_only" | "mutating" | "destructive";
  approval: "not_required" | "required";
  connections: CapabilityConnection[];
}
export interface CapabilityMapping {
  operation_ref: string;
  tool_name: string;
  connection_ref?: string;
  context?: string;
  posture: CapabilityPosture;
}
export interface CapabilityProfile {
  id: string;
  name: string;
  audience: "personal" | "tenant";
  revision: number;
  mappings: CapabilityMapping[];
  created_by: string;
  created_at_ms: number;
  updated_at_ms: number;
}
export type ConnectorConnection = components["schemas"]["ConnectorConnection"];
export type ConnectSession = components["schemas"]["ConnectSession"];

export class ApiError extends Error {
  constructor(
    public readonly status: number,
    public readonly code: string,
    public readonly details?: unknown,
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
    let details: unknown;
    try {
      const problem = (await response.json()) as { code?: unknown };
      details = problem;
      if (typeof problem.code === "string") code = problem.code;
    } catch {
      // The status remains useful when an intermediary returns a non-JSON response.
    }
    throw new ApiError(response.status, code, details);
  }
  if (response.status === 204) return undefined as T;
  return (await response.json()) as T;
}

export const api = {
  workflowLibrary: () => request<WorkflowLibraryPage>("/api/workflows"),
  installStarterWorkflowLibrary: () =>
    request<WorkflowLibraryPage>("/api/workflows", { method: "POST" }),
  workflowLibraryItem: (workflowId: string) =>
    request<WorkflowLibraryDetail>(`/api/workflows/${encodeURIComponent(workflowId)}`),
  session: () => request<Session>("/api/session"),
  identityProviders: () => request<IdentityProvider[]>("/api/auth/providers"),
  logout: () => request<undefined>("/auth/logout", { method: "POST" }),
  repositories: (query = "") => {
    const normalized = query.trim();
    const suffix = normalized ? `?query=${encodeURIComponent(normalized)}` : "";
    return request<RepositoryCandidate[]>(`/api/repositories${suffix}`);
  },
  openProject: (repository: Pick<RepositoryCandidate, "forge_instance_ref" | "project_ref">) =>
    request<Project>("/api/projects", {
      method: "POST",
      body: JSON.stringify({
        forge_instance_ref: repository.forge_instance_ref,
        project_ref: repository.project_ref,
      }),
    }),
  project: (projectId: string, signal?: AbortSignal) =>
    request<Project>(`/api/projects/${encodeURIComponent(projectId)}`, { signal }),
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
  workflowRuns: (projectId: string) =>
    request<WorkflowRun[]>(`/api/projects/${encodeURIComponent(projectId)}/workflow-runs`),
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
  codingSessions: (projectId: string) =>
    request<CodingSession[]>(`/api/projects/${encodeURIComponent(projectId)}/sessions`),
  createCodingSession: (projectId: string, sourceRevision: string, signal?: AbortSignal) =>
    request<CodingSession>(`/api/projects/${encodeURIComponent(projectId)}/sessions`, {
      method: "POST",
      signal,
      body: JSON.stringify({
        source_revision: sourceRevision,
        idempotency_key: crypto.randomUUID(),
      }),
    }),
  codingSession: (sessionId: string, signal?: AbortSignal) =>
    request<CodingSession>(`/api/project-sessions/${encodeURIComponent(sessionId)}`, { signal }),
  resumeCodingSession: (sessionId: string, signal?: AbortSignal) =>
    request<CodingSession>(`/api/project-sessions/${encodeURIComponent(sessionId)}/resume`, {
      method: "POST",
      signal,
    }),
  closeCodingSession: (sessionId: string) =>
    request<CodingSession>(`/api/project-sessions/${encodeURIComponent(sessionId)}`, {
      method: "DELETE",
    }),
  codingCoordination: (sessionId: string, signal?: AbortSignal) =>
    request<CodingCoordinationView>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/coordination`,
      { signal },
    ),
  codingWorkbench: (sessionId: string, signal?: AbortSignal) =>
    request<AgentIdeWorkbenchView>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/workbench`,
      { signal },
    ),
  mutateCodingWorkbench: (
    sessionId: string,
    input: MutateAgentIdeWorkbench,
    signal?: AbortSignal,
  ) =>
    request<AgentIdeWorkbenchView>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/workbench`,
      { method: "POST", body: JSON.stringify(input), signal },
    ),
  pinCodingContext: (
    sessionId: string,
    input: {
      kind: string;
      reference: string;
      start_line?: number | null;
      end_line?: number | null;
      sha256: string;
      idempotency_key: string;
    },
  ) =>
    request<CodingCoordinationView>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/coordination/pins`,
      { method: "POST", body: JSON.stringify(input) },
    ),
  removeCodingContextPin: (sessionId: string, pinId: string) =>
    request<CodingCoordinationView>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/coordination/pins/${encodeURIComponent(pinId)}`,
      { method: "DELETE" },
    ),
  createCodingGrant: (sessionId: string, grantee: string) =>
    request<CodingCoordinationView>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/coordination/grants`,
      {
        method: "POST",
        body: JSON.stringify({ grantee, idempotency_key: crypto.randomUUID() }),
      },
    ),
  revokeCodingGrant: (sessionId: string, grantId: string) =>
    request<CodingCoordinationView>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/coordination/grants/${encodeURIComponent(grantId)}`,
      { method: "DELETE" },
    ),
  decideCodingCheckpoint: (sessionId: string, checkpointId: string, decision: "approve" | "deny") =>
    request<CodingCoordinationView>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/coordination/checkpoints/${encodeURIComponent(checkpointId)}`,
      {
        method: "POST",
        body: JSON.stringify({ decision, idempotency_key: crypto.randomUUID() }),
      },
    ),
  codingTree: (
    sessionId: string,
    path = "",
    cursor?: string,
    limit = 500,
    signal?: AbortSignal,
  ) => {
    const parameters = new URLSearchParams({ path, limit: String(limit) });
    if (cursor) parameters.set("cursor", cursor);
    return request<CodingTreeProjection>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/tree?${parameters.toString()}`,
      { signal },
    );
  },
  codingFile: (sessionId: string, path: string, signal?: AbortSignal) =>
    request<FileProjection>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/files/${encodeWorkspacePath(path)}`,
      { signal },
    ),
  saveCodingFile: (sessionId: string, path: string, content: string, expectedSha256: string) =>
    request<FileProjection>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/files/${encodeWorkspacePath(path)}`,
      {
        method: "PUT",
        body: JSON.stringify({
          content,
          expected: { state: "sha256", sha256: expectedSha256 },
          create_parents: false,
          operation_id: crypto.randomUUID(),
        }),
      },
    ),
  codingDiff: (sessionId: string, selector: ChangeSelector, mode: DiffMode) =>
    request<DiffProjection>(`/api/project-sessions/${encodeURIComponent(sessionId)}/diff`, {
      method: "POST",
      body: JSON.stringify({ selector, mode }),
    }),
  terminalProfiles: (sessionId: string, signal?: AbortSignal) =>
    request<TerminalProfile[]>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/terminal-profiles`,
      { signal },
    ),
  terminals: (sessionId: string, signal?: AbortSignal) =>
    request<TerminalSession[]>(`/api/project-sessions/${encodeURIComponent(sessionId)}/terminals`, {
      signal,
    }),
  createTerminal: (
    sessionId: string,
    input: {
      profile_id: string;
      columns: number;
      rows: number;
      idempotency_key: string;
    },
  ) =>
    request<TerminalSession>(`/api/project-sessions/${encodeURIComponent(sessionId)}/terminals`, {
      method: "POST",
      body: JSON.stringify(input),
    }),
  terminal: (terminalId: string, signal?: AbortSignal) =>
    request<TerminalSession>(`/api/project-terminals/${encodeURIComponent(terminalId)}`, {
      signal,
    }),
  terminateTerminal: (terminalId: string) =>
    request<TerminalSession>(`/api/project-terminals/${encodeURIComponent(terminalId)}`, {
      method: "DELETE",
    }),
  terminalSocketUrl: (terminalId: string, fromSequence?: bigint) => {
    const scheme = window.location.protocol === "https:" ? "wss:" : "ws:";
    const endpoint = new URL(
      `${scheme}//${window.location.host}/api/project-terminals/${encodeURIComponent(terminalId)}/attach`,
    );
    if (fromSequence != null) endpoint.searchParams.set("from_sequence", fromSequence.toString());
    return endpoint.toString();
  },
  connections: () => request<ConnectorConnection[]>("/api/connections"),
  connectorCatalog: (query = "", offset = 0, limit = 24) => {
    const parameters = new URLSearchParams({
      query,
      offset: String(offset),
      limit: String(limit),
    });
    return request<ConnectorCatalogPage>(`/api/connectors/catalog?${parameters.toString()}`);
  },
  connectorCatalogProvider: (providerRef: string) =>
    request<ConnectorProviderDescription>(
      `/api/connectors/catalog/${encodeURIComponent(providerRef)}`,
    ),
  generatedServices: () => request<GeneratedServicePage>("/api/services"),
  generatedServiceCatalog: async (serviceRef: string) => {
    const catalog = await request<unknown>("/api/services/catalog", {
      method: "POST",
      body: JSON.stringify({ service_ref: serviceRef }),
    });
    assertServiceCatalog(catalog);
    return catalog;
  },
  invokeGeneratedService: (
    operationRef: string,
    input: Record<string, unknown>,
    confirmed = false,
  ) =>
    request<GeneratedServiceInvocation>("/api/services/invoke", {
      method: "POST",
      body: JSON.stringify({ operation_ref: operationRef, input, confirmed }),
    }),
  startConnection: (integrationRef: string, label: string, authProfile?: string) =>
    request<ConnectSession>("/api/connections", {
      method: "POST",
      body: JSON.stringify({
        integration_ref: integrationRef,
        label,
        auth_profile: authProfile,
      }),
    }),
  connectionSession: (sessionRef: string) =>
    request<ConnectSession>(`/api/connect-sessions/${encodeURIComponent(sessionRef)}`),
  capabilities: () => request<Capability[]>("/api/capabilities"),
  capabilityProfiles: () => request<CapabilityProfile[]>("/api/capability-profiles"),
  createCapabilityProfile: (
    name: string,
    audience: CapabilityProfile["audience"],
    mappings: CapabilityMapping[],
  ) =>
    request<CapabilityProfile>("/api/capability-profiles", {
      method: "POST",
      body: JSON.stringify({ name, audience, mappings }),
    }),
  updateCapabilityProfile: (
    profile: CapabilityProfile,
    mappings: CapabilityMapping[],
    name = profile.name,
  ) =>
    request<CapabilityProfile>(`/api/capability-profiles/${encodeURIComponent(profile.id)}`, {
      method: "PATCH",
      body: JSON.stringify({
        expected_revision: profile.revision,
        name,
        mappings,
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
  decidePublicationApproval: (
    publicationId: string,
    approvalId: string,
    decision: "approve" | "deny",
  ) =>
    request<undefined>(
      `/api/mcp/publications/${encodeURIComponent(publicationId)}/approvals/${encodeURIComponent(approvalId)}`,
      { method: "POST", body: JSON.stringify({ decision }) },
    ),
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
  agentTasks: (agentId: string) =>
    request<Task[]>(`/api/agents/${encodeURIComponent(agentId)}/tasks`),
  createAgent: (agent: CreateAgent) =>
    request<Agent>("/api/agents", { method: "POST", body: JSON.stringify(agent) }),
  submitTask: (agentId: string, prompt: string) =>
    request<Task>(`/api/agents/${encodeURIComponent(agentId)}/tasks`, {
      method: "POST",
      body: JSON.stringify({ prompt, idempotency_key: crypto.randomUUID() }),
    }),
  submitCodingTurn: (sessionId: string, agentId: string, input: SubmitCodingTurn) =>
    request<Task>(
      `/api/project-sessions/${encodeURIComponent(sessionId)}/agents/${encodeURIComponent(agentId)}/turns`,
      { method: "POST", body: JSON.stringify(input) },
    ),
  taskApprovals: (taskId: string) =>
    request<TaskApproval[]>(`/api/tasks/${encodeURIComponent(taskId)}/approvals`),
  resolveTaskApproval: (taskId: string, approvalId: string, decision: TaskApprovalDecision) =>
    request<TaskApproval>(
      `/api/tasks/${encodeURIComponent(taskId)}/approvals/${encodeURIComponent(approvalId)}`,
      { method: "POST", body: JSON.stringify(decision) },
    ),
};

const FRIENDLY_ERRORS: Record<string, string> = {
  agent_platform_not_configured: "Agent Platform is not configured for this environment.",
  workflow_not_configured:
    "The standalone Workflow library is not configured for this environment.",
  workflow_unavailable: "The standalone Workflow service is temporarily unavailable.",
  workflow_access_refused: "Your current authority does not admit this workflow.",
  workflow_not_found: "That workflow is no longer visible.",
  agent_platform_unavailable: "Agent Platform is temporarily unavailable.",
  connectors_unavailable: "The connection service is temporarily unavailable.",
  connectors_invalid_response: "The connection service returned an invalid response.",
  connector_approval_refused:
    "The exact call is no longer approvable. Refresh the task before deciding again.",
  task_approval_not_found: "This approval is no longer pending.",
  connection_start_refused: "This connection cannot be started with your current grant.",
  capability_search_refused: "Capabilities could not be read with your current grant.",
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
  workspace_request_refused: "Workspace could not load the requested repository data. Try again.",
  agentide_workspace_disabled: "The hosted coding workbench is disabled in this environment.",
  workspace_file_conflict:
    "This file changed after it was loaded. Review the conflict before editing further.",
  coding_turn_invalid:
    "The coding turn exceeded its attachment boundary or contained stale attachment metadata.",
  coding_session_not_ready: "The workspace is not ready for an agent turn.",
  identity_publication_revocation_unavailable:
    "Identity cannot yet revoke every authorization for this publication safely.",
  identity_client_revocation_unavailable:
    "Identity cannot yet revoke this client authorization safely.",
  identity_authentication_required: "Your session has expired. Sign in again.",
};

function encodeWorkspacePath(path: string): string {
  return path.split("/").map(encodeURIComponent).join("/");
}

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
    | { kind: "context_changed"; revision: string }
    | { kind: "inventory_changed"; revision: string; published_tools: string[] }
    | {
        kind: "approval_requested";
        approval_id: string;
        call_id: string;
        operation_ref: string;
        connection_ref: string;
      }
    | { kind: "approval_resolved"; approval_id: string; approved: boolean }
    | { kind: "succeeded"; output: string }
    | { kind: "failed"; failure?: { code?: string; message?: string } };
}

const TASK_FAILURES: Record<string, string> = {
  model_credential_unavailable:
    "Connect a user-bound model in Connections, then retry with a new task.",
  model_provider_unavailable: "The model provider is temporarily unavailable. Retry shortly.",
  model_provider_rate_limited: "The model provider rate-limited this task. Retry later.",
  model_route_refused: "The selected model route refused this task.",
  model_request_too_large: "This request is too large for the selected model route.",
  execution_interrupted: "The service restarted before this task finished. Submit it again.",
};

export function taskFailureMessage(failure?: { code?: string; message?: string }): string {
  const friendly = failure?.code ? TASK_FAILURES[failure.code] : undefined;
  if (friendly) return friendly;
  return failure?.message ?? "The task failed without a reason.";
}
