//! Product-owned task orchestration. Workspace keeps the existing durable project rows.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agent_platform_client::{
    ActivateRevision, AgentId, AgentPlatformClient, ClientError, CreateAgent, RevisionSpec,
    SubmitTask, Task, TaskId,
};
use agent_platform_core::{
    ConversationInput, ConversationMessage, ConversationRole, ProjectContext, TaskStatus,
};
use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::Response;
use workspace_client::WorkspaceClient;
use workspace_core::{
    CreateMessage, MessageRole, Project, ProjectTaskReply as Reply, ProjectTaskRequest as Request,
    StartWorkflow, WorkflowDefinition, WorkflowRunState,
};
use zeroize::Zeroizing;

use super::{
    AppState, AuthenticatedSession, confidential_json, problem, unavailable, workspace_error,
};

#[derive(Clone, Default)]
pub(super) struct Observers(Arc<Mutex<BTreeSet<String>>>);

impl Observers {
    fn begin(&self, key: &str) -> bool {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .insert(key.to_owned())
    }
    fn finish(&self, key: &str) {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .remove(key);
    }
}

fn clients(state: &AppState) -> Result<(&WorkspaceClient, &AgentPlatformClient), Response> {
    let workspace = state
        .workspace
        .as_ref()
        .ok_or_else(|| unavailable("workspace_not_configured"))?;
    let platform = state
        .agent_platform
        .as_ref()
        .ok_or_else(|| unavailable("agent_platform_not_configured"))?;
    if state.config.workspace_signing_key_file.is_none() {
        return Err(unavailable("project_coordinator_not_configured"));
    }
    Ok((workspace, platform))
}

async fn request(
    workspace: &WorkspaceClient,
    bearer: &str,
    request: Request,
) -> Result<Reply, Response> {
    workspace
        .project_task(bearer, &request)
        .await
        .map_err(|error| workspace_error(&error))
}

fn invalid_reply() -> Response {
    unavailable("project_bookkeeping_contract_invalid")
}

async fn context(
    workspace: &WorkspaceClient,
    bearer: &str,
    project_id: &str,
) -> Result<(Project, ProjectContext), Response> {
    let Reply::Context { project, context } = request(
        workspace,
        bearer,
        Request::Context {
            project_id: project_id.into(),
        },
    )
    .await?
    else {
        return Err(invalid_reply());
    };
    let context = serde_json::to_value(context)
        .and_then(serde_json::from_value)
        .map_err(|_| invalid_reply())?;
    Ok((*project, context))
}

async fn ensure_agent(
    state: &AppState,
    workspace: &WorkspaceClient,
    platform: &AgentPlatformClient,
    bearer: &str,
    project: &Project,
) -> Result<AgentId, Response> {
    let Reply::Agent { agent_id } = request(
        workspace,
        bearer,
        Request::ProjectAgent {
            project_id: project.id.clone(),
        },
    )
    .await?
    else {
        return Err(invalid_reply());
    };
    if let Some(agent_id) = agent_id {
        return AgentId::new(agent_id).map_err(|_| invalid_reply());
    }
    let name = format!("Repository project {}", project.id);
    let agent = platform
        .list_agents(bearer)
        .await
        .map_err(|_| unavailable("project_agent_unavailable"))?
        .into_iter()
        .find(|agent| agent.name == name);
    let agent = match agent {
        Some(agent) => agent,
        None => platform
            .create_agent(bearer, &CreateAgent { name })
            .await
            .map_err(|_| unavailable("project_agent_unavailable"))?,
    };
    if agent.active_revision.is_none() {
        let model = state
            .config
            .project_agent_model
            .clone()
            .ok_or_else(|| unavailable("project_agent_model_unconfigured"))?;
        let revision = platform.create_revision(bearer, &agent.id, &RevisionSpec {
            instructions: "You are the analysis-only agent for one repository project. Ground every answer in the exact commit and files supplied in the typed project context. State when the supplied context is insufficient. Never claim write, merge, or deployment authority.".into(),
            model, capability_profile_id: None, metadata: Some(serde_json::json!({"workspace_project_id": project.id})),
        }).await.map_err(|_| unavailable("project_agent_unavailable"))?;
        platform
            .activate_revision(
                bearer,
                &agent.id,
                &ActivateRevision {
                    revision: revision.revision,
                    expected_active_revision: None,
                },
            )
            .await
            .map_err(|_| unavailable("project_agent_unavailable"))?;
    }
    let Reply::Agent {
        agent_id: Some(agent_id),
    } = request(
        workspace,
        bearer,
        Request::RecordProjectAgent {
            project_id: project.id.clone(),
            agent_id: agent.id.to_string(),
        },
    )
    .await?
    else {
        return Err(invalid_reply());
    };
    AgentId::new(agent_id).map_err(|_| invalid_reply())
}

fn owns_task(auth: &AuthenticatedSession, task: &Task, idempotency: &str) -> bool {
    task.tenant_id.as_str() == auth.principal.tenant_id
        && task.actor.as_str() == auth.principal.subject
        && task.idempotency_key == idempotency
}

pub(super) async fn create_message(
    state: &AppState,
    auth: AuthenticatedSession,
    thread_id: &str,
    input: CreateMessage,
) -> Response {
    let result = async {
        let (workspace, platform) = clients(state)?;
        let bearer = auth.authorization.as_str();
        let Reply::Thread { thread } = request(
            workspace,
            bearer,
            Request::Thread {
                thread_id: thread_id.into(),
            },
        )
        .await?
        else {
            return Err(invalid_reply());
        };
        let (project, context) = context(workspace, bearer, &thread.project_id).await?;
        if project.selected_branch != thread.branch
            || project.pinned_commit.as_deref() != Some(&thread.pinned_commit)
        {
            return Err(problem(StatusCode::CONFLICT, "thread_snapshot_stale"));
        }
        let agent_id = ensure_agent(state, workspace, platform, bearer, &project).await?;
        let prior = workspace
            .messages(bearer, thread_id)
            .await
            .map_err(|error| workspace_error(&error))?;
        let message = workspace
            .create_message(bearer, thread_id, &input)
            .await
            .map_err(|error| workspace_error(&error))?;
        let idempotency = format!("{}:{}", thread.id, message.sequence);
        let input = ConversationInput::ProjectConversation {
            prompt: input.content,
            messages: prior
                .into_iter()
                .map(|message| ConversationMessage {
                    role: match message.role {
                        MessageRole::User => ConversationRole::User,
                        MessageRole::Assistant => ConversationRole::Assistant,
                        MessageRole::System => ConversationRole::System,
                    },
                    content: message.content,
                })
                .collect(),
            context,
        };
        let submit = SubmitTask {
            agent_id,
            idempotency_key: idempotency.clone(),
            input: serde_json::to_value(input).map_err(|_| invalid_reply())?,
        };
        match platform.submit_task(bearer, &submit).await {
            Ok(task) => {
                if !owns_task(&auth, &task, &idempotency) || task.agent_id != submit.agent_id {
                    return Err(problem(
                        StatusCode::FORBIDDEN,
                        "project_task_binding_refused",
                    ));
                }
                request(
                    workspace,
                    bearer,
                    Request::RecordMessageTask {
                        thread_id: thread.id.clone(),
                        sequence: message.sequence,
                        task_id: task.id.to_string(),
                    },
                )
                .await?;
                observe_message(state, &auth, &thread.id, message.sequence, task.id);
            }
            Err(_) => {
                request(
                    workspace,
                    bearer,
                    Request::AppendMessage {
                        thread_id: thread.id,
                        role: MessageRole::System,
                        content: "The project agent refused this turn before execution.".into(),
                    },
                )
                .await?;
            }
        }
        Ok(confidential_json(message))
    }
    .await;
    result.unwrap_or_else(|response| response)
}

pub(super) async fn message_events(
    state: &AppState,
    auth: AuthenticatedSession,
    thread_id: &str,
    sequence: u64,
) -> Response {
    let result = async {
        let (workspace, platform) = clients(state)?;
        let Reply::MessageTask {
            task_id,
            needs_completion,
        } = request(
            workspace,
            &auth.authorization,
            Request::MessageTask {
                thread_id: thread_id.into(),
                sequence,
            },
        )
        .await?
        else {
            return Err(invalid_reply());
        };
        let task_id = TaskId::new(task_id).map_err(|_| invalid_reply())?;
        let task = platform
            .get_task(&auth.authorization, &task_id)
            .await
            .map_err(|_| unavailable("project_task_unavailable"))?;
        if !owns_task(&auth, &task, &format!("{thread_id}:{sequence}")) {
            return Err(problem(
                StatusCode::FORBIDDEN,
                "project_task_binding_refused",
            ));
        }
        // Preserve legacy links and streams. An in-flight legacy task can reattach an observer;
        // already-finished legacy messages remain untouched. New turns have durable completion markers.
        if needs_completion
            || matches!(
                task.status,
                TaskStatus::Accepted | TaskStatus::Running | TaskStatus::AwaitingApproval
            )
        {
            observe_message(state, &auth, thread_id, sequence, task_id.clone());
        }
        let upstream = platform
            .task_events(&auth.authorization, &task_id)
            .await
            .map_err(|_| unavailable("project_agent_stream_unavailable"))?;
        let mut response = Response::new(Body::from_stream(upstream.bytes_stream()));
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("text/event-stream"),
        );
        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
        Ok(response)
    }
    .await;
    result.unwrap_or_else(|response| response)
}

pub(super) async fn workflows(
    state: &AppState,
    auth: &AuthenticatedSession,
    project_id: &str,
) -> Response {
    let Some(workspace) = &state.workspace else {
        return unavailable("workspace_not_configured");
    };
    match workspace.project(&auth.authorization, project_id).await {
        Ok(_) => confidential_json(workflow_definitions()),
        Err(error) => workspace_error(&error),
    }
}

pub(super) async fn start_workflow(
    state: &AppState,
    auth: AuthenticatedSession,
    project_id: &str,
    input: StartWorkflow,
) -> Response {
    let result = async {
        if !workflow_definitions()
            .iter()
            .any(|definition| definition.id == input.definition_id)
        {
            return Err(problem(
                StatusCode::UNPROCESSABLE_ENTITY,
                "workflow_run_invalid",
            ));
        }
        let (workspace, platform) = clients(state)?;
        let bearer = auth.authorization.as_str();
        let (project, context) = context(workspace, bearer, project_id).await?;
        if project.selected_branch != input.branch
            || project.pinned_commit.as_deref() != Some(&input.commit)
        {
            return Err(problem(StatusCode::CONFLICT, "project_snapshot_stale"));
        }
        let agent_id = ensure_agent(state, workspace, platform, bearer, &project).await?;
        let run = workspace
            .start_workflow(bearer, project_id, &input)
            .await
            .map_err(|error| workspace_error(&error))?;
        if matches!(
            run.state,
            WorkflowRunState::Succeeded | WorkflowRunState::Failed | WorkflowRunState::Refused
        ) {
            return Ok(confidential_json(run));
        }
        let Reply::Task { task_id } = request(
            workspace,
            bearer,
            Request::WorkflowTask {
                run_id: run.id.clone(),
            },
        )
        .await?
        else {
            return Err(invalid_reply());
        };
        let task_id = if let Some(task_id) = task_id {
            TaskId::new(task_id).map_err(|_| unavailable("workflow_task_invalid"))?
        } else {
            let idempotency = format!("workspace-workflow:{}", run.id);
            let input = ConversationInput::ProjectConversation {
                prompt: workflow_prompt(&input.definition_id).to_owned(),
                messages: Vec::new(),
                context,
            };
            let submit = SubmitTask {
                agent_id,
                idempotency_key: idempotency.clone(),
                input: serde_json::to_value(input).map_err(|_| invalid_reply())?,
            };
            let task = platform
                .submit_task(bearer, &submit)
                .await
                .map_err(|_| problem(StatusCode::BAD_GATEWAY, "workflow_dispatch_refused"))?;
            if !owns_task(&auth, &task, &idempotency) || task.agent_id != submit.agent_id {
                return Err(problem(
                    StatusCode::FORBIDDEN,
                    "workflow_task_binding_refused",
                ));
            }
            request(
                workspace,
                bearer,
                Request::RecordWorkflowTask {
                    run_id: run.id.clone(),
                    task_id: task.id.to_string(),
                },
            )
            .await?;
            task.id
        };
        observe_workflow(state, &auth, &run.id, task_id);
        Ok(confidential_json(run))
    }
    .await;
    result.unwrap_or_else(|response| response)
}

pub(super) async fn workflow_runs(
    state: &AppState,
    auth: AuthenticatedSession,
    project_id: &str,
) -> Response {
    let result = async {
        let (workspace, _) = clients(state)?;
        let Reply::WorkflowTasks { tasks } = request(
            workspace,
            &auth.authorization,
            Request::RecoverableWorkflowTasks {
                project_id: project_id.into(),
            },
        )
        .await?
        else {
            return Err(invalid_reply());
        };
        for task in tasks {
            match TaskId::new(task.task_id) {
                Ok(task_id) => observe_workflow(state, &auth, &task.run_id, task_id),
                Err(_) => {
                    request(
                        workspace,
                        &auth.authorization,
                        Request::UpdateWorkflow {
                            run_id: task.run_id,
                            state: WorkflowRunState::Failed,
                            failure_code: Some("workflow_task_invalid".into()),
                            output: None,
                        },
                    )
                    .await?;
                }
            }
        }
        let runs = workspace
            .workflow_runs(&auth.authorization, project_id)
            .await
            .map_err(|error| workspace_error(&error))?;
        Ok(confidential_json(runs))
    }
    .await;
    result.unwrap_or_else(|response| response)
}

enum ObservedTarget {
    Message { thread_id: String, sequence: u64 },
    Workflow { run_id: String },
}

struct Observation {
    workspace: WorkspaceClient,
    platform: AgentPlatformClient,
    authorization: Zeroizing<String>,
    tenant: String,
    subject: String,
    task_id: TaskId,
    target: ObservedTarget,
    observers: Observers,
}

fn observe_message(
    state: &AppState,
    auth: &AuthenticatedSession,
    thread_id: &str,
    sequence: u64,
    task_id: TaskId,
) {
    observe(
        state,
        auth,
        task_id,
        ObservedTarget::Message {
            thread_id: thread_id.into(),
            sequence,
        },
    );
}
fn observe_workflow(state: &AppState, auth: &AuthenticatedSession, run_id: &str, task_id: TaskId) {
    observe(
        state,
        auth,
        task_id,
        ObservedTarget::Workflow {
            run_id: run_id.into(),
        },
    );
}
fn observe(state: &AppState, auth: &AuthenticatedSession, task_id: TaskId, target: ObservedTarget) {
    let Ok((workspace, platform)) = clients(state) else {
        return;
    };
    let key = task_id.to_string();
    if !state.project_observers.begin(&key) {
        return;
    }
    let observation = Observation {
        workspace: workspace.clone(),
        platform: platform.clone(),
        authorization: Zeroizing::new(auth.authorization.to_string()),
        tenant: auth.principal.tenant_id.clone(),
        subject: auth.principal.subject.clone(),
        task_id,
        target,
        observers: state.project_observers.clone(),
    };
    tokio::spawn(async move {
        observation
            .observe_window(300, Duration::from_millis(500))
            .await;
        observation.observers.finish(&key);
    });
}

impl Observation {
    fn owns(&self, task: &Task) -> bool {
        let key = match &self.target {
            ObservedTarget::Message {
                thread_id,
                sequence,
            } => format!("{thread_id}:{sequence}"),
            ObservedTarget::Workflow { run_id } => format!("workspace-workflow:{run_id}"),
        };
        task.id == self.task_id
            && task.tenant_id.as_str() == self.tenant
            && task.actor.as_str() == self.subject
            && task.idempotency_key == key
    }

    async fn observe_window(&self, attempts: usize, interval: Duration) {
        for attempt in 0..attempts {
            let task = match self
                .platform
                .get_task(&self.authorization, &self.task_id)
                .await
            {
                Ok(task) => task,
                Err(error) if retryable_workflow_observation(&error) => {
                    if attempt + 1 < attempts {
                        tokio::time::sleep(interval).await;
                    }
                    continue;
                }
                Err(_) => return,
            };
            if !self.owns(&task) {
                let _ = self
                    .transition(WorkflowTaskOutcome::Failed("workflow_task_binding_refused"))
                    .await;
                return;
            }
            let outcome = match (&self.target, task.status) {
                (ObservedTarget::Message { .. }, TaskStatus::Succeeded) => {
                    WorkflowTaskOutcome::Succeeded(task.output.unwrap_or_else(|| {
                        "The project agent completed without a text response.".into()
                    }))
                }
                _ => workflow_task_outcome(task.status, task.output),
            };
            let terminal = matches!(
                outcome,
                WorkflowTaskOutcome::Succeeded(_) | WorkflowTaskOutcome::Failed(_)
            );
            match self.transition(outcome).await {
                Ok(()) if terminal => return,
                Ok(()) => {}
                Err(error) if retryable_bookkeeping(&error) => {}
                Err(_) => return,
            }
            if attempt + 1 < attempts {
                tokio::time::sleep(interval).await;
            }
        }
    }

    async fn transition(
        &self,
        outcome: WorkflowTaskOutcome,
    ) -> Result<(), workspace_client::ClientError> {
        let operation = match (&self.target, outcome) {
            (_, WorkflowTaskOutcome::Accepted)
            | (ObservedTarget::Message { .. }, WorkflowTaskOutcome::Running) => return Ok(()),
            (ObservedTarget::Workflow { run_id }, outcome) => {
                let (state, failure_code, output) = match outcome {
                    WorkflowTaskOutcome::Running => (WorkflowRunState::Running, None, None),
                    WorkflowTaskOutcome::Succeeded(output) => {
                        (WorkflowRunState::Succeeded, None, Some(output))
                    }
                    WorkflowTaskOutcome::Failed(code) => {
                        (WorkflowRunState::Failed, Some(code.into()), None)
                    }
                    WorkflowTaskOutcome::Accepted => return Ok(()),
                };
                Request::UpdateWorkflow {
                    run_id: run_id.clone(),
                    state,
                    failure_code,
                    output,
                }
            }
            (
                ObservedTarget::Message {
                    thread_id,
                    sequence,
                },
                outcome,
            ) => {
                let (role, content) = match outcome {
                    WorkflowTaskOutcome::Succeeded(output) => (MessageRole::Assistant, output),
                    WorkflowTaskOutcome::Failed(_) => (
                        MessageRole::System,
                        "The project agent did not complete this turn.".into(),
                    ),
                    _ => return Ok(()),
                };
                Request::CompleteMessageTask {
                    thread_id: thread_id.clone(),
                    sequence: *sequence,
                    task_id: self.task_id.to_string(),
                    role,
                    content,
                }
            }
        };
        match self
            .workspace
            .project_task(&self.authorization, &operation)
            .await?
        {
            Reply::Updated { .. } | Reply::Message { .. } => Ok(()),
            _ => Err(workspace_client::ClientError::Configuration),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum WorkflowTaskOutcome {
    Accepted,
    Running,
    Succeeded(String),
    Failed(&'static str),
}

fn retryable_bookkeeping(error: &workspace_client::ClientError) -> bool {
    match error {
        workspace_client::ClientError::Transport => true,
        workspace_client::ClientError::Refused(status) => {
            matches!(*status, 408 | 425 | 429 | 500..=599)
        }
        workspace_client::ClientError::Configuration
        | workspace_client::ClientError::FileConflict(_) => false,
    }
}

fn retryable_workflow_observation(error: &ClientError) -> bool {
    match error {
        ClientError::Transport(_) => true,
        ClientError::Refused(status) => {
            matches!(*status, 408 | 425 | 429 | 500..=599)
        }
        ClientError::Configuration => false,
    }
}

fn workflow_task_outcome(status: TaskStatus, output: Option<String>) -> WorkflowTaskOutcome {
    match status {
        TaskStatus::Accepted => WorkflowTaskOutcome::Accepted,
        TaskStatus::Running | TaskStatus::AwaitingApproval => WorkflowTaskOutcome::Running,
        TaskStatus::Succeeded => WorkflowTaskOutcome::Succeeded(
            output
                .filter(|output| !output.trim().is_empty())
                .unwrap_or_else(|| {
                    "# Workflow result\n\nThe workflow completed without a Markdown result."
                        .to_owned()
                }),
        ),
        TaskStatus::Failed => WorkflowTaskOutcome::Failed("workflow_execution_failed"),
        TaskStatus::Cancelled => WorkflowTaskOutcome::Failed("workflow_execution_cancelled"),
        TaskStatus::Refused => WorkflowTaskOutcome::Failed("workflow_execution_refused"),
        TaskStatus::OutcomeUnknown => WorkflowTaskOutcome::Failed("workflow_outcome_unknown"),
    }
}

fn workflow_prompt(definition_id: &str) -> &'static str {
    match definition_id {
        "review.code/v1" => {
            "Review this exact repository snapshot for correctness, regressions, maintainability risks, and missing tests. Return a concise Markdown report ordered by severity. Cite every finding with repository paths from the supplied context and say explicitly when the bounded context is insufficient."
        }
        "review.security/v1" => {
            "Perform a security review of this exact repository snapshot. Return a concise Markdown report with severity, exploit preconditions, impact, and remediation for each finding. Cite repository paths from the supplied context and do not claim evidence that was not supplied."
        }
        "reverse.aep-ess/v1" => {
            "Reverse-engineer this exact repository snapshot into an evidence-backed current-state system specification and a proposed AEP plan. Return Markdown with system boundaries, interfaces, invariants, risks, and sequenced work. Cite repository paths from the supplied context and distinguish observed facts from proposals."
        }
        _ => {
            "Analyze this exact repository snapshot and return an evidence-backed Markdown report."
        }
    }
}

fn workflow_definitions() -> Vec<WorkflowDefinition> {
    vec![
        WorkflowDefinition {
            id: "review.code/v1".to_owned(),
            name: "Code review".to_owned(),
            description:
                "Commit-pinned correctness and maintainability findings with file citations."
                    .to_owned(),
        },
        WorkflowDefinition {
            id: "review.security/v1".to_owned(),
            name: "Security review".to_owned(),
            description: "Commit-pinned security findings with typed severity and evidence."
                .to_owned(),
        },
        WorkflowDefinition {
            id: "reverse.aep-ess/v1".to_owned(),
            name: "Reverse AEP + ESS".to_owned(),
            description:
                "Evidence-backed draft planning entities and a current-state system specification."
                    .to_owned(),
        },
    ]
}

#[cfg(test)]
mod tests;
