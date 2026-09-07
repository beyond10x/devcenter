//! Explicit authenticated lifecycle routes; all durable operations remain in Agent Platform.
use super::{
    AppState, AuthenticatedSession, agent_platform_error, agent_task_summary, authenticate,
    confidential_json, unavailable,
};
use agent_platform_client::{
    AgentId, AgentPlatformClient, ConversationId, ConversationRevision, CreateConversation,
    RevisionSpec, UpdateAgent, UpdateConversation,
};
use agent_platform_core::CapabilityProfileId;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, patch, post},
};
use serde::Deserialize;
use serde_json::json;

pub(super) fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/agents/{agent_id}",
            get(details).patch(update).delete(delete),
        )
        .route(
            "/api/agents/{agent_id}/conversations",
            get(list).post(create),
        )
        .route(
            "/api/agents/{agent_id}/conversations/{conversation_id}",
            patch(rename).delete(remove),
        )
        .route(
            "/api/agents/{agent_id}/conversations/{conversation_id}/clear",
            post(clear),
        )
        .route(
            "/api/agents/{agent_id}/conversations/{conversation_id}/tasks",
            get(tasks),
        )
}

async fn authorized(
    state: &AppState,
    headers: &HeaderMap,
    mutation: bool,
) -> Result<(AuthenticatedSession, AgentPlatformClient), Response> {
    let session = authenticate(state, headers, mutation).await?;
    let client = state
        .agent_platform
        .clone()
        .ok_or_else(|| unavailable("agent_platform_not_configured"))?;
    Ok((session, client))
}

async fn details(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<AgentId>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, false).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let agent = match client.get_agent(session.authorization.as_str(), &id).await {
        Ok(agent) => agent,
        Err(error) => return agent_platform_error(&error),
    };
    let revisions = match client
        .list_revisions(session.authorization.as_str(), &id)
        .await
    {
        Ok(revisions) => revisions,
        Err(error) => return agent_platform_error(&error),
    };
    let spec = revisions
        .into_iter()
        .find(|revision| Some(revision.revision) == agent.active_revision)
        .map(|revision| revision.spec);
    confidential_json(json!({"agent":agent,"spec":spec}))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Update {
    name: String,
    instructions: String,
    model: String,
    capability_profile_id: Option<CapabilityProfileId>,
    expected_active_revision: Option<u64>,
}

async fn update(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<AgentId>,
    Json(request): Json<Update>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, true).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    let revisions = match client
        .list_revisions(session.authorization.as_str(), &id)
        .await
    {
        Ok(revisions) => revisions,
        Err(error) => return agent_platform_error(&error),
    };
    let metadata = revisions
        .into_iter()
        .find(|revision| Some(revision.revision) == request.expected_active_revision)
        .and_then(|revision| revision.spec.metadata);
    let request = UpdateAgent {
        name: request.name,
        expected_active_revision: request.expected_active_revision,
        spec: RevisionSpec {
            instructions: request.instructions,
            model: request.model,
            capability_profile_id: request.capability_profile_id,
            metadata,
        },
    };
    match client
        .update_agent(session.authorization.as_str(), &id, &request)
        .await
    {
        Ok(agent) => confidential_json(agent),
        Err(error) => agent_platform_error(&error),
    }
}

async fn delete(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<AgentId>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, true).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match client
        .retire_agent(session.authorization.as_str(), &id)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

pub(super) async fn delete_profile(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<CapabilityProfileId>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, true).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match client
        .retire_capability_profile(session.authorization.as_str(), &id)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

async fn list(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<AgentId>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, false).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match client
        .list_conversations(session.authorization.as_str(), &id)
        .await
    {
        Ok(items) => confidential_json(items),
        Err(error) => agent_platform_error(&error),
    }
}

async fn create(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path(id): Path<AgentId>,
    Json(request): Json<CreateConversation>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, true).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match client
        .create_conversation(session.authorization.as_str(), &id, &request)
        .await
    {
        Ok(item) => (StatusCode::CREATED, confidential_json(item)).into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

async fn rename(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((agent, id)): Path<(AgentId, ConversationId)>,
    Json(request): Json<UpdateConversation>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, true).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match client
        .update_conversation(session.authorization.as_str(), &agent, &id, &request)
        .await
    {
        Ok(item) => confidential_json(item),
        Err(error) => agent_platform_error(&error),
    }
}

async fn remove(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((agent, id)): Path<(AgentId, ConversationId)>,
    Json(request): Json<ConversationRevision>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, true).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match client
        .delete_conversation(session.authorization.as_str(), &agent, &id, &request)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

async fn clear(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((agent, id)): Path<(AgentId, ConversationId)>,
    Json(request): Json<ConversationRevision>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, true).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match client
        .clear_conversation(session.authorization.as_str(), &agent, &id, &request)
        .await
    {
        Ok(item) => (StatusCode::CREATED, confidential_json(item)).into_response(),
        Err(error) => agent_platform_error(&error),
    }
}

async fn tasks(
    State(state): State<AppState>,
    headers: HeaderMap,
    Path((agent, id)): Path<(AgentId, ConversationId)>,
) -> Response {
    let (session, client) = match authorized(&state, &headers, false).await {
        Ok(pair) => pair,
        Err(response) => return response,
    };
    match client
        .list_conversation_tasks(session.authorization.as_str(), &agent, &id)
        .await
    {
        Ok(items) => confidential_json(
            items
                .into_iter()
                .map(agent_task_summary)
                .collect::<Vec<_>>(),
        ),
        Err(error) => agent_platform_error(&error),
    }
}
