//! The former Workspace observer acceptance scenarios now exercise the product-owned HTTP seam.
use super::*;
use axum::{
    Json, Router,
    extract::State,
    http::HeaderMap,
    response::IntoResponse,
    routing::{get, post},
};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone)]
struct Fixture {
    status: Arc<Mutex<String>>,
    task_errors: Arc<AtomicUsize>,
    write_errors: Arc<AtomicUsize>,
    task_reads: Arc<AtomicUsize>,
    writes: Arc<Mutex<Vec<Request>>>,
}

impl Fixture {
    fn new(status: &str) -> Self {
        Self {
            status: Arc::new(Mutex::new(status.into())),
            task_errors: Arc::new(AtomicUsize::new(0)),
            write_errors: Arc::new(AtomicUsize::new(0)),
            task_reads: Arc::new(AtomicUsize::new(0)),
            writes: Arc::new(Mutex::new(Vec::new())),
        }
    }

    async fn serve(&self) -> (String, tokio::task::JoinHandle<()>) {
        async fn task(State(state): State<Fixture>, headers: HeaderMap) -> Response {
            assert_eq!(
                headers.get(header::AUTHORIZATION).unwrap(),
                "Bearer fresh-session"
            );
            state.task_reads.fetch_add(1, Ordering::SeqCst);
            if state
                .task_errors
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_sub(1))
                .is_ok()
            {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
            Json(serde_json::json!({
                "id":"task-one", "tenant_id":"tenant-one", "actor":"person:owner",
                "agent_id":"agent-one", "agent_revision":1, "capability_profile_id":null,
                "idempotency_key":"workspace-workflow:run-one", "input":{},
                "status":state.status.lock().unwrap().clone(), "attempt_id":"attempt-one",
                "output":"# Reviewed\n\nResult from the original task.", "executor":null,
                "delegation_id":null, "request_id":"request-one", "accepted_at_ms":1,
                "completed_at_ms":null
            }))
            .into_response()
        }
        async fn bookkeeping(
            State(state): State<Fixture>,
            headers: HeaderMap,
            Json(request): Json<Request>,
        ) -> Response {
            assert_eq!(
                headers.get(header::AUTHORIZATION).unwrap(),
                "Bearer fresh-session"
            );
            if state
                .write_errors
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |v| v.checked_sub(1))
                .is_ok()
            {
                return StatusCode::SERVICE_UNAVAILABLE.into_response();
            }
            assert!(
                matches!(&request, Request::UpdateWorkflow { run_id, .. } if run_id == "run-one"),
                "recovery must write the original workflow and never submit a replacement"
            );
            state.writes.lock().unwrap().push(request);
            Json(Reply::Updated { changed: true }).into_response()
        }
        let router = Router::new()
            .route("/v1/tasks/{task_id}", get(task))
            .route("/v1/project-tasks", post(bookkeeping))
            .with_state(self.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = format!("http://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        (address, server)
    }
}

fn observation(address: &str) -> Observation {
    Observation {
        workspace: WorkspaceClient::new(address).unwrap(),
        platform: AgentPlatformClient::new(address).unwrap(),
        authorization: Zeroizing::new("Bearer fresh-session".into()),
        tenant: "tenant-one".into(),
        subject: "person:owner".into(),
        task_id: TaskId::new("task-one").unwrap(),
        target: ObservedTarget::Workflow {
            run_id: "run-one".into(),
        },
        observers: Observers::default(),
    }
}

#[tokio::test]
async fn persisted_workflow_task_resumes_with_fresh_observer_and_session() {
    let fixture = Fixture::new("running");
    let (address, server) = fixture.serve().await;
    observation(&address)
        .observe_window(1, Duration::ZERO)
        .await;
    assert!(
        matches!(
            fixture.writes.lock().unwrap().last(),
            Some(Request::UpdateWorkflow {
                state: WorkflowRunState::Running,
                ..
            })
        ),
        "ending an observation window must leave the durable workflow recoverable"
    );
    *fixture.status.lock().unwrap() = "succeeded".into();
    // Reconstruct every product-side observer object; only the external task reference survives.
    observation(&address)
        .observe_window(1, Duration::ZERO)
        .await;
    assert!(
        matches!(fixture.writes.lock().unwrap().last(),
        Some(Request::UpdateWorkflow { state: WorkflowRunState::Succeeded, output: Some(output), .. })
        if output.contains("original task")),
        "recovery must project the original task output"
    );
    assert_eq!(fixture.task_reads.load(Ordering::SeqCst), 2);
    server.abort();
}

#[tokio::test]
async fn transient_workflow_observation_and_result_write_failures_are_retried() {
    let fixture = Fixture::new("succeeded");
    fixture.task_errors.store(1, Ordering::SeqCst);
    fixture.write_errors.store(1, Ordering::SeqCst);
    let (address, server) = fixture.serve().await;
    observation(&address)
        .observe_window(3, Duration::ZERO)
        .await;
    assert_eq!(fixture.task_reads.load(Ordering::SeqCst), 3);
    assert_eq!(
        fixture.writes.lock().unwrap().len(),
        1,
        "a transient result-write failure must not discard a successful task"
    );
    assert!(matches!(
        fixture.writes.lock().unwrap()[0],
        Request::UpdateWorkflow {
            state: WorkflowRunState::Succeeded,
            ..
        }
    ));
    server.abort();
}

#[tokio::test]
async fn long_running_workflow_and_exhausted_transient_errors_remain_recoverable() {
    for status in ["accepted", "running", "awaiting_approval"] {
        let fixture = Fixture::new(status);
        let (address, server) = fixture.serve().await;
        observation(&address)
            .observe_window(2, Duration::ZERO)
            .await;
        assert!(
            fixture.writes.lock().unwrap().iter().all(|write| matches!(
                write,
                Request::UpdateWorkflow {
                    state: WorkflowRunState::Running,
                    ..
                }
            )),
            "an elapsed observer window must not manufacture terminal failure"
        );
        fixture.task_errors.store(2, Ordering::SeqCst);
        let writes = fixture.writes.lock().unwrap().len();
        observation(&address)
            .observe_window(2, Duration::ZERO)
            .await;
        assert_eq!(
            fixture.writes.lock().unwrap().len(),
            writes,
            "transport failures must leave durable recovery state intact"
        );
        server.abort();
    }
}

#[tokio::test]
async fn another_subject_cannot_observe_the_owners_task() {
    let fixture = Fixture::new("succeeded");
    let (address, server) = fixture.serve().await;
    let mut other = observation(&address);
    other.subject = "another-person".into();
    other.observe_window(1, Duration::ZERO).await;
    assert!(
        matches!(fixture.writes.lock().unwrap().last(), Some(Request::UpdateWorkflow {
        state: WorkflowRunState::Failed, failure_code: Some(code), output: None, ..
    }) if code == "workflow_task_binding_refused"),
        "wrong-owner task output must never enter the project"
    );
    server.abort();
}

fn product_state(address: &str) -> AppState {
    AppState {
        config: Arc::new(devcenter_core::Config {
            tenant_id: "tenant-one".into(),
            public_origin: "https://product.example.test".into(),
            authentication: devcenter_auth::Authentication::Unconfigured,
            identity_web_client_id: None,
            identity_redirect_uri: None,
            identity_providers: Vec::new(),
            database_url: "sqlite::memory:".into(),
            agent_platform_origin: Some(address.into()),
            connectors_api_base: None,
            connectors_docs_available: false,
            workspace_origin: Some(address.into()),
            workspace_signing_key_file: Some("injected-test-client".into()),
            project_agent_model: None,
            workflow_origin: None,
            agentide_workspace_enabled: false,
        }),
        agent_platform: Some(AgentPlatformClient::new(address).unwrap()),
        connectors: None,
        workspace: Some(WorkspaceClient::new(address).unwrap()),
        project_observers: Observers::default(),
        workflow: None,
        pending_logins: Arc::new(Mutex::new(std::collections::BTreeMap::new())),
        publications: devcenter_store::Store::connect_lazy("sqlite::memory:").unwrap(),
    }
}

fn authenticated() -> AuthenticatedSession {
    AuthenticatedSession {
        authorization: Zeroizing::new("Bearer fresh-session".into()),
        principal: devcenter_auth::Principal {
            tenant_id: "tenant-one".into(),
            subject: "person:owner".into(),
            email: None,
            groups: Vec::new(),
        },
    }
}

#[tokio::test]
#[allow(clippy::too_many_lines)] // Two real client adapters exercise ambiguous admission and durable task reuse.
async fn ambiguous_workflow_submit_failure_remains_idempotently_retryable() {
    let submitted = Arc::new(Mutex::new(Vec::<SubmitTask>::new()));
    let task_link = Arc::new(Mutex::new(None::<String>));
    let accepted = workspace_core::WorkflowRun {
        id: "run-one".into(),
        definition_id: "review.code/v1".into(),
        project_id: "project-one".into(),
        branch: "trunk".into(),
        commit: "c".repeat(40),
        state: WorkflowRunState::Accepted,
        failure_code: None,
        output: None,
        created_at_ms: 1,
    };
    let run = accepted.clone();
    let links = task_link.clone();
    let attempts = submitted.clone();
    let router = Router::new().route("/v1/project-tasks", post(move |Json(request): Json<Request>| {
        let links = links.clone();
        async move {
            let reply = match request {
                Request::Context { project_id } => Reply::Context {
                    project: Box::new(Project { id: project_id.clone(), forge_instance_ref: "connection:git:one".into(),
                        project_ref: "42".into(), path_with_namespace: "group/project".into(), name: "project".into(),
                        default_branch: Some("trunk".into()), selected_branch: "trunk".into(), pinned_commit: Some("c".repeat(40)),
                        web_url: "https://git.example.test/group/project".into() }),
                    context: workspace_core::ProjectContext { project_id, provider: "git".into(), provider_project_ref: "42".into(),
                        path_with_namespace: "group/project".into(), branch: "trunk".into(), commit: "c".repeat(40), files: Vec::new() },
                },
                Request::ProjectAgent { .. } => Reply::Agent { agent_id: Some("agent-one".into()) },
                Request::WorkflowTask { .. } => Reply::Task { task_id: links.lock().unwrap().clone() },
                Request::RecordWorkflowTask { run_id, task_id } => {
                    assert_eq!(run_id, "run-one");
                    *links.lock().unwrap() = Some(task_id.clone());
                    Reply::Task { task_id: Some(task_id) }
                },
                _ => panic!("ambiguous submission must not mark the accepted workflow failed"),
            };
            Json(reply)
        }
    })).route("/v1/projects/{id}/workflow-runs", post(move |Json(input): Json<StartWorkflow>| {
        let run = run.clone();
        async move { assert_eq!(input.idempotency_key, "request-one"); Json(run) }
    })).route("/v1/tasks", post(move |Json(input): Json<SubmitTask>| {
        let attempts = attempts.clone();
        async move {
            let first = {
                let mut attempts = attempts.lock().unwrap();
                attempts.push(input.clone()); attempts.len() == 1
            };
            if first { return StatusCode::SERVICE_UNAVAILABLE.into_response(); }
            Json(serde_json::json!({
                "id":"task-one", "tenant_id":"tenant-one", "actor":"person:owner", "agent_id":input.agent_id,
                "agent_revision":1, "capability_profile_id":null, "idempotency_key":input.idempotency_key,
                "input":input.input, "status":"accepted", "attempt_id":"attempt-one", "output":null,
                "executor":null, "delegation_id":null, "request_id":"request-one", "accepted_at_ms":1,
                "completed_at_ms":null
            })).into_response()
        }
    }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let input = StartWorkflow {
        definition_id: accepted.definition_id.clone(),
        branch: accepted.branch.clone(),
        commit: accepted.commit.clone(),
        idempotency_key: "request-one".into(),
    };
    let state = product_state(&address);
    let first = start_workflow(&state, authenticated(), "project-one", input.clone()).await;
    assert_eq!(
        first.status(),
        StatusCode::BAD_GATEWAY,
        "ambiguous admission remains retryable"
    );
    assert_eq!(*task_link.lock().unwrap(), None);
    let restarted = product_state(&address);
    let second = start_workflow(&restarted, authenticated(), "project-one", input.clone()).await;
    assert_eq!(second.status(), StatusCode::OK);
    assert_eq!(task_link.lock().unwrap().as_deref(), Some("task-one"));
    let third = start_workflow(&restarted, authenticated(), "project-one", input).await;
    assert_eq!(third.status(), StatusCode::OK);
    let attempts = submitted.lock().unwrap();
    assert_eq!(
        attempts.len(),
        2,
        "an existing durable task link must avoid another submission"
    );
    assert_eq!(
        serde_json::to_value(&attempts[0]).unwrap(),
        serde_json::to_value(&attempts[1]).unwrap(),
        "restart retries must preserve the original task intent and idempotency key"
    );
    assert_eq!(attempts[0].idempotency_key, "workspace-workflow:run-one");
    server.abort();
}

#[tokio::test]
async fn recoverable_workflow_survives_a_product_without_task_authority() {
    let mut state = product_state("http://127.0.0.1:1");
    state.agent_platform = None;
    let response = workflow_runs(&state, authenticated(), "project-one").await;
    let body = http_body_util::BodyExt::collect(response.into_body())
        .await
        .unwrap()
        .to_bytes();
    assert_eq!(
        body, r#"{"code":"agent_platform_not_configured"}"#,
        "absence of execution authority must refuse before trying to mutate recovery state"
    );
}

#[test]
fn workflow_task_terminal_states_project_to_named_safe_results() {
    assert_eq!(
        workflow_task_outcome(TaskStatus::Succeeded, None),
        WorkflowTaskOutcome::Succeeded(
            "# Workflow result\n\nThe workflow completed without a Markdown result.".to_owned()
        )
    );
    for (status, code) in [
        (TaskStatus::Failed, "workflow_execution_failed"),
        (TaskStatus::Cancelled, "workflow_execution_cancelled"),
        (TaskStatus::Refused, "workflow_execution_refused"),
        (TaskStatus::OutcomeUnknown, "workflow_outcome_unknown"),
    ] {
        assert_eq!(
            workflow_task_outcome(status, None),
            WorkflowTaskOutcome::Failed(code)
        );
    }
}
