//! Devcenter-owned publication, client authorization, and approval persistence.

use std::sync::Arc;

use devcenter_mcp::CompiledTool;
use serde::{Deserialize, Serialize};
use sqlx::any::{AnyPoolOptions, AnyRow};
use sqlx::{AnyPool, Row as _};
use tokio::sync::OnceCell;

const SCHEMA: [&str; 6] = [
    "CREATE TABLE IF NOT EXISTS mcp_publications (publication_id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, owner_subject TEXT NOT NULL, profile_id TEXT NOT NULL, active_revision BIGINT NOT NULL, toolset_digest TEXT NOT NULL, state TEXT NOT NULL, created_at_ms BIGINT NOT NULL, updated_at_ms BIGINT NOT NULL)",
    "CREATE TABLE IF NOT EXISTS mcp_publication_revisions (publication_id TEXT NOT NULL, revision BIGINT NOT NULL, profile_revision BIGINT NOT NULL, toolset_digest TEXT NOT NULL, tools_json TEXT NOT NULL, created_at_ms BIGINT NOT NULL, PRIMARY KEY (publication_id, revision), FOREIGN KEY (publication_id) REFERENCES mcp_publications(publication_id))",
    "CREATE TABLE IF NOT EXISTS mcp_client_authorizations (authorization_id TEXT PRIMARY KEY, publication_id TEXT NOT NULL, subject TEXT NOT NULL, client_id TEXT NOT NULL, display_name TEXT NOT NULL, state TEXT NOT NULL, first_used_at_ms BIGINT NOT NULL, last_used_at_ms BIGINT NOT NULL, FOREIGN KEY (publication_id) REFERENCES mcp_publications(publication_id))",
    "CREATE TABLE IF NOT EXISTS mcp_approvals (approval_id TEXT PRIMARY KEY, publication_id TEXT NOT NULL, authorization_id TEXT NOT NULL, subject TEXT NOT NULL, client_id TEXT NOT NULL, tool_name TEXT NOT NULL, operation_ref TEXT NOT NULL, connection_id TEXT NOT NULL, input_digest TEXT NOT NULL, state TEXT NOT NULL, expires_at_ms BIGINT NOT NULL, audit_ref TEXT, created_at_ms BIGINT NOT NULL, updated_at_ms BIGINT NOT NULL, FOREIGN KEY (publication_id) REFERENCES mcp_publications(publication_id), FOREIGN KEY (authorization_id) REFERENCES mcp_client_authorizations(authorization_id))",
    "CREATE INDEX IF NOT EXISTS mcp_approvals_queue ON mcp_approvals (publication_id, state, expires_at_ms)",
    "CREATE UNIQUE INDEX IF NOT EXISTS mcp_approvals_live_input ON mcp_approvals (publication_id, authorization_id, tool_name, operation_ref, connection_id, input_digest) WHERE state IN ('pending', 'approved')",
];

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PublicationState {
    Active,
    Suspended,
    Revoked,
}

impl PublicationState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Revoked => "revoked",
        }
    }

    fn parse(value: &str) -> Result<Self, StoreError> {
        match value {
            "active" => Ok(Self::Active),
            "suspended" => Ok(Self::Suspended),
            "revoked" => Ok(Self::Revoked),
            _ => Err(StoreError::Corrupt),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationState {
    Active,
    Revoked,
}

impl AuthorizationState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Revoked => "revoked",
        }
    }

    fn parse(value: &str) -> Result<Self, StoreError> {
        match value {
            "active" => Ok(Self::Active),
            "revoked" => Ok(Self::Revoked),
            _ => Err(StoreError::Corrupt),
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalState {
    Pending,
    Approved,
    Denied,
    Consumed,
    Expired,
    OutcomeUnknown,
}

impl ApprovalState {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
            Self::Consumed => "consumed",
            Self::Expired => "expired",
            Self::OutcomeUnknown => "outcome_unknown",
        }
    }

    fn parse(value: &str) -> Result<Self, StoreError> {
        match value {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "denied" => Ok(Self::Denied),
            "consumed" => Ok(Self::Consumed),
            "expired" => Ok(Self::Expired),
            "outcome_unknown" => Ok(Self::OutcomeUnknown),
            _ => Err(StoreError::Corrupt),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Publication {
    pub publication_id: String,
    pub tenant_id: String,
    pub owner_subject: String,
    pub profile_id: String,
    pub active_revision: i64,
    pub toolset_digest: String,
    pub state: PublicationState,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct PublicationRevision {
    pub publication_id: String,
    pub revision: i64,
    pub profile_revision: i64,
    pub toolset_digest: String,
    pub tools: Vec<CompiledTool>,
    pub created_at_ms: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ClientAuthorization {
    pub authorization_id: String,
    pub publication_id: String,
    pub subject: String,
    pub client_id: String,
    pub display_name: String,
    pub state: AuthorizationState,
    pub first_used_at_ms: i64,
    pub last_used_at_ms: i64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Approval {
    pub approval_id: String,
    pub publication_id: String,
    pub authorization_id: String,
    pub subject: String,
    pub client_id: String,
    pub tool_name: String,
    pub operation_ref: String,
    pub connection_id: String,
    pub input_digest: String,
    pub state: ApprovalState,
    pub expires_at_ms: i64,
    pub audit_ref: Option<String>,
    pub created_at_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("publication store configuration is invalid")]
    Configuration,
    #[error("publication store is unavailable")]
    Database(#[source] sqlx::Error),
    #[error("stored publication data is invalid")]
    Corrupt,
    #[error("publication state transition was refused")]
    Conflict,
}

#[derive(Clone)]
pub struct Store {
    pool: AnyPool,
    initialized: Arc<OnceCell<()>>,
}

impl Store {
    pub fn connect_lazy(database_url: &str) -> Result<Self, StoreError> {
        if !(database_url.starts_with("sqlite:") || database_url.starts_with("postgres")) {
            return Err(StoreError::Configuration);
        }
        sqlx::any::install_default_drivers();
        let max_connections = if database_url.contains(":memory:") {
            1
        } else {
            5
        };
        let pool = AnyPoolOptions::new()
            .max_connections(max_connections)
            .connect_lazy(database_url)
            .map_err(StoreError::Database)?;
        Ok(Self {
            pool,
            initialized: Arc::new(OnceCell::new()),
        })
    }

    pub async fn ready(&self) -> Result<(), StoreError> {
        self.ensure_schema().await
    }

    async fn ensure_schema(&self) -> Result<(), StoreError> {
        self.initialized
            .get_or_try_init(|| async {
                for statement in SCHEMA {
                    sqlx::query(statement)
                        .execute(&self.pool)
                        .await
                        .map_err(StoreError::Database)?;
                }
                Ok::<(), StoreError>(())
            })
            .await
            .copied()
    }

    pub async fn create_publication(
        &self,
        publication: &Publication,
        revision: &PublicationRevision,
    ) -> Result<(), StoreError> {
        self.ensure_schema().await?;
        if publication.state != PublicationState::Active
            || publication.publication_id != revision.publication_id
            || publication.active_revision != revision.revision
            || publication.toolset_digest != revision.toolset_digest
        {
            return Err(StoreError::Conflict);
        }
        let tools = serde_json::to_string(&revision.tools).map_err(|_| StoreError::Corrupt)?;
        let mut transaction = self.pool.begin().await.map_err(StoreError::Database)?;
        sqlx::query("INSERT INTO mcp_publications (publication_id, tenant_id, owner_subject, profile_id, active_revision, toolset_digest, state, created_at_ms, updated_at_ms) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)")
            .bind(&publication.publication_id)
            .bind(&publication.tenant_id)
            .bind(&publication.owner_subject)
            .bind(&publication.profile_id)
            .bind(publication.active_revision)
            .bind(&publication.toolset_digest)
            .bind(publication.state.as_str())
            .bind(publication.created_at_ms)
            .bind(publication.updated_at_ms)
            .execute(&mut *transaction)
            .await
            .map_err(StoreError::Database)?;
        sqlx::query("INSERT INTO mcp_publication_revisions (publication_id, revision, profile_revision, toolset_digest, tools_json, created_at_ms) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(&revision.publication_id)
            .bind(revision.revision)
            .bind(revision.profile_revision)
            .bind(&revision.toolset_digest)
            .bind(tools)
            .bind(revision.created_at_ms)
            .execute(&mut *transaction)
            .await
            .map_err(StoreError::Database)?;
        transaction.commit().await.map_err(StoreError::Database)
    }

    pub async fn publication(
        &self,
        publication_id: &str,
    ) -> Result<Option<Publication>, StoreError> {
        self.ensure_schema().await?;
        sqlx::query("SELECT publication_id, tenant_id, owner_subject, profile_id, active_revision, toolset_digest, state, created_at_ms, updated_at_ms FROM mcp_publications WHERE publication_id = $1")
            .bind(publication_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(StoreError::Database)?
            .map(|row| publication_from_row(&row))
            .transpose()
    }

    pub async fn publications_for(
        &self,
        tenant_id: &str,
        owner_subject: &str,
    ) -> Result<Vec<Publication>, StoreError> {
        self.ensure_schema().await?;
        let rows = sqlx::query("SELECT publication_id, tenant_id, owner_subject, profile_id, active_revision, toolset_digest, state, created_at_ms, updated_at_ms FROM mcp_publications WHERE tenant_id = $1 AND owner_subject = $2 ORDER BY created_at_ms DESC")
            .bind(tenant_id)
            .bind(owner_subject)
            .fetch_all(&self.pool)
            .await
            .map_err(StoreError::Database)?;
        rows.iter().map(publication_from_row).collect()
    }

    pub async fn active_revision(
        &self,
        publication: &Publication,
    ) -> Result<PublicationRevision, StoreError> {
        self.ensure_schema().await?;
        let row = sqlx::query("SELECT publication_id, revision, profile_revision, toolset_digest, tools_json, created_at_ms FROM mcp_publication_revisions WHERE publication_id = $1 AND revision = $2")
            .bind(&publication.publication_id)
            .bind(publication.active_revision)
            .fetch_optional(&self.pool)
            .await
            .map_err(StoreError::Database)?
            .ok_or(StoreError::Corrupt)?;
        revision_from_row(&row)
    }

    pub async fn set_publication_state(
        &self,
        publication_id: &str,
        tenant_id: &str,
        owner_subject: &str,
        state: PublicationState,
        now_ms: i64,
    ) -> Result<Publication, StoreError> {
        self.ensure_schema().await?;
        let mut transaction = self.pool.begin().await.map_err(StoreError::Database)?;
        let result = sqlx::query("UPDATE mcp_publications SET state = $1, updated_at_ms = $2 WHERE publication_id = $3 AND tenant_id = $4 AND owner_subject = $5 AND state <> 'revoked'")
            .bind(state.as_str())
            .bind(now_ms)
            .bind(publication_id)
            .bind(tenant_id)
            .bind(owner_subject)
            .execute(&mut *transaction)
            .await
            .map_err(StoreError::Database)?;
        if result.rows_affected() != 1 {
            transaction.rollback().await.map_err(StoreError::Database)?;
            return Err(StoreError::Conflict);
        }
        if state == PublicationState::Revoked {
            sqlx::query("UPDATE mcp_client_authorizations SET state = 'revoked', last_used_at_ms = $1 WHERE publication_id = $2 AND state = 'active'")
                .bind(now_ms)
                .bind(publication_id)
                .execute(&mut *transaction)
                .await
                .map_err(StoreError::Database)?;
        }
        transaction.commit().await.map_err(StoreError::Database)?;
        self.publication(publication_id)
            .await?
            .ok_or(StoreError::Corrupt)
    }

    pub async fn record_client_use(
        &self,
        authorization: &ClientAuthorization,
    ) -> Result<(), StoreError> {
        self.ensure_schema().await?;
        sqlx::query("INSERT INTO mcp_client_authorizations (authorization_id, publication_id, subject, client_id, display_name, state, first_used_at_ms, last_used_at_ms) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) ON CONFLICT (authorization_id) DO UPDATE SET last_used_at_ms = EXCLUDED.last_used_at_ms")
            .bind(&authorization.authorization_id)
            .bind(&authorization.publication_id)
            .bind(&authorization.subject)
            .bind(&authorization.client_id)
            .bind(&authorization.display_name)
            .bind(authorization.state.as_str())
            .bind(authorization.first_used_at_ms)
            .bind(authorization.last_used_at_ms)
            .execute(&self.pool)
            .await
            .map_err(StoreError::Database)?;
        Ok(())
    }

    pub async fn client_authorizations(
        &self,
        publication_id: &str,
    ) -> Result<Vec<ClientAuthorization>, StoreError> {
        self.ensure_schema().await?;
        let rows = sqlx::query("SELECT authorization_id, publication_id, subject, client_id, display_name, state, first_used_at_ms, last_used_at_ms FROM mcp_client_authorizations WHERE publication_id = $1 ORDER BY first_used_at_ms DESC")
            .bind(publication_id)
            .fetch_all(&self.pool)
            .await
            .map_err(StoreError::Database)?;
        rows.iter().map(client_from_row).collect()
    }

    pub async fn revoke_client(
        &self,
        publication_id: &str,
        authorization_id: &str,
        subject: &str,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        self.ensure_schema().await?;
        let result = sqlx::query("UPDATE mcp_client_authorizations SET state = 'revoked', last_used_at_ms = $1 WHERE authorization_id = $2 AND publication_id = $3 AND subject = $4 AND state = 'active'")
            .bind(now_ms)
            .bind(authorization_id)
            .bind(publication_id)
            .bind(subject)
            .execute(&self.pool)
            .await
            .map_err(StoreError::Database)?;
        (result.rows_affected() == 1)
            .then_some(())
            .ok_or(StoreError::Conflict)
    }

    pub async fn create_approval(&self, approval: &Approval) -> Result<(), StoreError> {
        self.ensure_schema().await?;
        if approval.state != ApprovalState::Pending {
            return Err(StoreError::Conflict);
        }
        sqlx::query("INSERT INTO mcp_approvals (approval_id, publication_id, authorization_id, subject, client_id, tool_name, operation_ref, connection_id, input_digest, state, expires_at_ms, audit_ref, created_at_ms, updated_at_ms) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)")
            .bind(&approval.approval_id)
            .bind(&approval.publication_id)
            .bind(&approval.authorization_id)
            .bind(&approval.subject)
            .bind(&approval.client_id)
            .bind(&approval.tool_name)
            .bind(&approval.operation_ref)
            .bind(&approval.connection_id)
            .bind(&approval.input_digest)
            .bind(approval.state.as_str())
            .bind(approval.expires_at_ms)
            .bind(&approval.audit_ref)
            .bind(approval.created_at_ms)
            .bind(approval.updated_at_ms)
            .execute(&self.pool)
            .await
            .map_err(StoreError::Database)?;
        Ok(())
    }

    pub async fn pending_approvals(
        &self,
        publication_id: &str,
        subject: &str,
        now_ms: i64,
    ) -> Result<Vec<Approval>, StoreError> {
        self.ensure_schema().await?;
        sqlx::query("UPDATE mcp_approvals SET state = 'expired', updated_at_ms = $1 WHERE publication_id = $2 AND state IN ('pending', 'approved') AND expires_at_ms <= $3")
            .bind(now_ms)
            .bind(publication_id)
            .bind(now_ms)
            .execute(&self.pool)
            .await
            .map_err(StoreError::Database)?;
        let rows = sqlx::query("SELECT approval_id, publication_id, authorization_id, subject, client_id, tool_name, operation_ref, connection_id, input_digest, state, expires_at_ms, audit_ref, created_at_ms, updated_at_ms FROM mcp_approvals WHERE publication_id = $1 AND subject = $2 AND state = 'pending' ORDER BY created_at_ms")
            .bind(publication_id)
            .bind(subject)
            .fetch_all(&self.pool)
            .await
            .map_err(StoreError::Database)?;
        rows.iter().map(approval_from_row).collect()
    }

    pub async fn decide_approval(
        &self,
        approval_id: &str,
        subject: &str,
        decision: ApprovalState,
        audit_ref: Option<&str>,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        self.ensure_schema().await?;
        if !matches!(decision, ApprovalState::Approved | ApprovalState::Denied) {
            return Err(StoreError::Conflict);
        }
        let result = sqlx::query("UPDATE mcp_approvals SET state = $1, audit_ref = $2, updated_at_ms = $3 WHERE approval_id = $4 AND subject = $5 AND state = 'pending' AND expires_at_ms > $6")
            .bind(decision.as_str())
            .bind(audit_ref)
            .bind(now_ms)
            .bind(approval_id)
            .bind(subject)
            .bind(now_ms)
            .execute(&self.pool)
            .await
            .map_err(StoreError::Database)?;
        (result.rows_affected() == 1)
            .then_some(())
            .ok_or(StoreError::Conflict)
    }

    /// Atomically spend one approved, unexpired request for an identical client retry.
    #[allow(clippy::too_many_arguments)]
    pub async fn consume_approval(
        &self,
        approval_id: &str,
        publication_id: &str,
        authorization_id: &str,
        subject: &str,
        client_id: &str,
        tool_name: &str,
        operation_ref: &str,
        connection_id: &str,
        input_digest: &str,
        now_ms: i64,
    ) -> Result<(), StoreError> {
        self.ensure_schema().await?;
        let result = sqlx::query("UPDATE mcp_approvals SET state = 'consumed', updated_at_ms = $1 WHERE approval_id = $2 AND publication_id = $3 AND authorization_id = $4 AND subject = $5 AND client_id = $6 AND tool_name = $7 AND operation_ref = $8 AND connection_id = $9 AND input_digest = $10 AND state = 'approved' AND expires_at_ms > $11")
            .bind(now_ms)
            .bind(approval_id)
            .bind(publication_id)
            .bind(authorization_id)
            .bind(subject)
            .bind(client_id)
            .bind(tool_name)
            .bind(operation_ref)
            .bind(connection_id)
            .bind(input_digest)
            .bind(now_ms)
            .execute(&self.pool)
            .await
            .map_err(StoreError::Database)?;
        (result.rows_affected() == 1)
            .then_some(())
            .ok_or(StoreError::Conflict)
    }
}

fn publication_from_row(row: &AnyRow) -> Result<Publication, StoreError> {
    let state: String = get(row, "state")?;
    Ok(Publication {
        publication_id: get(row, "publication_id")?,
        tenant_id: get(row, "tenant_id")?,
        owner_subject: get(row, "owner_subject")?,
        profile_id: get(row, "profile_id")?,
        active_revision: get(row, "active_revision")?,
        toolset_digest: get(row, "toolset_digest")?,
        state: PublicationState::parse(&state)?,
        created_at_ms: get(row, "created_at_ms")?,
        updated_at_ms: get(row, "updated_at_ms")?,
    })
}

fn revision_from_row(row: &AnyRow) -> Result<PublicationRevision, StoreError> {
    let tools: String = get(row, "tools_json")?;
    Ok(PublicationRevision {
        publication_id: get(row, "publication_id")?,
        revision: get(row, "revision")?,
        profile_revision: get(row, "profile_revision")?,
        toolset_digest: get(row, "toolset_digest")?,
        tools: serde_json::from_str(&tools).map_err(|_| StoreError::Corrupt)?,
        created_at_ms: get(row, "created_at_ms")?,
    })
}

fn client_from_row(row: &AnyRow) -> Result<ClientAuthorization, StoreError> {
    let state: String = get(row, "state")?;
    Ok(ClientAuthorization {
        authorization_id: get(row, "authorization_id")?,
        publication_id: get(row, "publication_id")?,
        subject: get(row, "subject")?,
        client_id: get(row, "client_id")?,
        display_name: get(row, "display_name")?,
        state: AuthorizationState::parse(&state)?,
        first_used_at_ms: get(row, "first_used_at_ms")?,
        last_used_at_ms: get(row, "last_used_at_ms")?,
    })
}

fn approval_from_row(row: &AnyRow) -> Result<Approval, StoreError> {
    let state: String = get(row, "state")?;
    Ok(Approval {
        approval_id: get(row, "approval_id")?,
        publication_id: get(row, "publication_id")?,
        authorization_id: get(row, "authorization_id")?,
        subject: get(row, "subject")?,
        client_id: get(row, "client_id")?,
        tool_name: get(row, "tool_name")?,
        operation_ref: get(row, "operation_ref")?,
        connection_id: get(row, "connection_id")?,
        input_digest: get(row, "input_digest")?,
        state: ApprovalState::parse(&state)?,
        expires_at_ms: get(row, "expires_at_ms")?,
        audit_ref: get(row, "audit_ref")?,
        created_at_ms: get(row, "created_at_ms")?,
        updated_at_ms: get(row, "updated_at_ms")?,
    })
}

fn get<'a, T>(row: &'a AnyRow, field: &str) -> Result<T, StoreError>
where
    T: sqlx::Decode<'a, sqlx::Any> + sqlx::Type<sqlx::Any>,
{
    row.try_get(field).map_err(StoreError::Database)
}

#[cfg(test)]
mod tests {
    use devcenter_mcp::{ApprovalPosture, Effect, Toolset};
    use serde_json::json;

    use super::*;

    fn tool() -> CompiledTool {
        CompiledTool {
            name: "issue_get".to_owned(),
            title: "Get issue".to_owned(),
            description: "Read an issue".to_owned(),
            operation_ref: "git/issue.get".to_owned(),
            connection_id: "connection-1".to_owned(),
            input_schema: json!({"type":"object"}),
            output_schema: json!({"type":"object"}),
            effect: Effect::ReadOnly,
            approval: ApprovalPosture::NotRequired,
        }
    }

    async fn store() -> Store {
        let store = Store::connect_lazy("sqlite::memory:").unwrap();
        store.ready().await.unwrap();
        store
    }

    async fn seed_named(
        store: &Store,
        publication_id: &str,
        tenant_id: &str,
        owner_subject: &str,
    ) -> Publication {
        let tools = vec![tool()];
        let digest = Toolset::compile(tools.clone()).unwrap().digest().to_owned();
        let publication = Publication {
            publication_id: publication_id.to_owned(),
            tenant_id: tenant_id.to_owned(),
            owner_subject: owner_subject.to_owned(),
            profile_id: "profile-1".to_owned(),
            active_revision: 1,
            toolset_digest: digest.clone(),
            state: PublicationState::Active,
            created_at_ms: 10,
            updated_at_ms: 10,
        };
        store
            .create_publication(
                &publication,
                &PublicationRevision {
                    publication_id: publication.publication_id.clone(),
                    revision: 1,
                    profile_revision: 7,
                    toolset_digest: digest,
                    tools,
                    created_at_ms: 10,
                },
            )
            .await
            .unwrap();
        publication
    }

    async fn seed(store: &Store) -> Publication {
        seed_named(store, "pub_opaque", "tenant-1", "human-1").await
    }

    #[tokio::test]
    async fn publication_revision_and_owner_scope_are_durable() {
        let store = store().await;
        let publication = seed(&store).await;
        assert_eq!(
            store.publications_for("tenant-1", "human-1").await.unwrap(),
            vec![publication.clone()]
        );
        assert!(
            store
                .publications_for("tenant-1", "other")
                .await
                .unwrap()
                .is_empty()
        );
        let revision = store.active_revision(&publication).await.unwrap();
        assert_eq!(revision.profile_revision, 7);
        assert_eq!(revision.tools[0].name, "issue_get");
    }

    #[tokio::test]
    async fn revoked_publications_are_terminal_and_revoke_each_client() {
        let store = store().await;
        let publication = seed(&store).await;
        store
            .record_client_use(&ClientAuthorization {
                authorization_id: "authorization-1".to_owned(),
                publication_id: publication.publication_id.clone(),
                subject: publication.owner_subject.clone(),
                client_id: "client-1".to_owned(),
                display_name: "CLI".to_owned(),
                state: AuthorizationState::Active,
                first_used_at_ms: 20,
                last_used_at_ms: 20,
            })
            .await
            .unwrap();
        store
            .set_publication_state(
                &publication.publication_id,
                &publication.tenant_id,
                &publication.owner_subject,
                PublicationState::Revoked,
                30,
            )
            .await
            .unwrap();
        assert_eq!(
            store
                .client_authorizations(&publication.publication_id)
                .await
                .unwrap()[0]
                .state,
            AuthorizationState::Revoked
        );
        assert!(
            store
                .set_publication_state(
                    &publication.publication_id,
                    &publication.tenant_id,
                    &publication.owner_subject,
                    PublicationState::Active,
                    40,
                )
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn publication_revocation_rolls_back_when_client_revocation_fails() {
        let store = store().await;
        let publication = seed(&store).await;
        store
            .record_client_use(&ClientAuthorization {
                authorization_id: "authorization-1".to_owned(),
                publication_id: publication.publication_id.clone(),
                subject: publication.owner_subject.clone(),
                client_id: "client-1".to_owned(),
                display_name: "CLI".to_owned(),
                state: AuthorizationState::Active,
                first_used_at_ms: 20,
                last_used_at_ms: 20,
            })
            .await
            .unwrap();
        sqlx::query(
            "CREATE TRIGGER refuse_client_revocation BEFORE UPDATE ON mcp_client_authorizations BEGIN SELECT RAISE(FAIL, 'refused'); END",
        )
        .execute(&store.pool)
        .await
        .unwrap();

        assert!(
            store
                .set_publication_state(
                    &publication.publication_id,
                    &publication.tenant_id,
                    &publication.owner_subject,
                    PublicationState::Revoked,
                    30,
                )
                .await
                .is_err()
        );
        assert_eq!(
            store
                .publication(&publication.publication_id)
                .await
                .unwrap()
                .unwrap()
                .state,
            PublicationState::Active
        );
        assert_eq!(
            store
                .client_authorizations(&publication.publication_id)
                .await
                .unwrap()[0]
                .state,
            AuthorizationState::Active
        );
    }

    #[tokio::test]
    async fn approval_decision_is_exact_expiring_and_compare_and_swap() {
        let store = store().await;
        let publication = seed(&store).await;
        store
            .record_client_use(&ClientAuthorization {
                authorization_id: "authorization-1".to_owned(),
                publication_id: publication.publication_id.clone(),
                subject: publication.owner_subject.clone(),
                client_id: "client-1".to_owned(),
                display_name: "CLI".to_owned(),
                state: AuthorizationState::Active,
                first_used_at_ms: 20,
                last_used_at_ms: 20,
            })
            .await
            .unwrap();
        store
            .create_approval(&Approval {
                approval_id: "approval-1".to_owned(),
                publication_id: publication.publication_id.clone(),
                authorization_id: "authorization-1".to_owned(),
                subject: publication.owner_subject.clone(),
                client_id: "client-1".to_owned(),
                tool_name: "issue_close".to_owned(),
                operation_ref: "git/issue.close".to_owned(),
                connection_id: "connection-1".to_owned(),
                input_digest: "digest".to_owned(),
                state: ApprovalState::Pending,
                expires_at_ms: 600_000,
                audit_ref: None,
                created_at_ms: 10,
                updated_at_ms: 10,
            })
            .await
            .unwrap();
        store
            .decide_approval(
                "approval-1",
                "human-1",
                ApprovalState::Approved,
                Some("audit-1"),
                20,
            )
            .await
            .unwrap();
        assert!(
            store
                .decide_approval("approval-1", "human-1", ApprovalState::Denied, None, 21,)
                .await
                .is_err()
        );
        store
            .consume_approval(
                "approval-1",
                "pub_opaque",
                "authorization-1",
                "human-1",
                "client-1",
                "issue_close",
                "git/issue.close",
                "connection-1",
                "digest",
                21,
            )
            .await
            .unwrap();
        assert!(
            store
                .consume_approval(
                    "approval-1",
                    "pub_opaque",
                    "authorization-1",
                    "human-1",
                    "client-1",
                    "issue_close",
                    "git/issue.close",
                    "connection-1",
                    "digest",
                    22,
                )
                .await
                .is_err(),
            "an identical concurrent retry cannot replay consumed approval"
        );
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn postgresql_executes_the_complete_store_contract() {
        let Ok(database_url) = std::env::var("DEV_CENTER_TEST_POSTGRES_URL") else {
            return;
        };
        let store = Store::connect_lazy(&database_url).unwrap();
        store.ready().await.unwrap();
        let suffix = format!(
            "{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let publication_id = format!("pub_{suffix}");
        let tenant_id = format!("tenant-{suffix}");
        let subject = format!("human-{suffix}");
        let publication = seed_named(&store, &publication_id, &tenant_id, &subject).await;

        assert_eq!(
            store.publication(&publication_id).await.unwrap(),
            Some(publication.clone())
        );
        assert_eq!(
            store.publications_for(&tenant_id, &subject).await.unwrap(),
            vec![publication.clone()]
        );
        assert_eq!(
            store
                .active_revision(&publication)
                .await
                .unwrap()
                .profile_revision,
            7
        );

        let authorization_id = format!("authorization-{suffix}");
        let second_authorization_id = format!("authorization-second-{suffix}");
        for authorization_id in [&authorization_id, &second_authorization_id] {
            store
                .record_client_use(&ClientAuthorization {
                    authorization_id: authorization_id.clone(),
                    publication_id: publication_id.clone(),
                    subject: subject.clone(),
                    client_id: format!("client-{authorization_id}"),
                    display_name: "CLI".to_owned(),
                    state: AuthorizationState::Active,
                    first_used_at_ms: 20,
                    last_used_at_ms: 20,
                })
                .await
                .unwrap();
        }
        assert_eq!(
            store
                .client_authorizations(&publication_id)
                .await
                .unwrap()
                .len(),
            2
        );
        store
            .revoke_client(&publication_id, &second_authorization_id, &subject, 25)
            .await
            .unwrap();

        let approval_id = format!("approval-{suffix}");
        store
            .create_approval(&Approval {
                approval_id: approval_id.clone(),
                publication_id: publication_id.clone(),
                authorization_id: authorization_id.clone(),
                subject: subject.clone(),
                client_id: format!("client-{authorization_id}"),
                tool_name: "issue_close".to_owned(),
                operation_ref: "git/issue.close".to_owned(),
                connection_id: "connection-1".to_owned(),
                input_digest: "digest".to_owned(),
                state: ApprovalState::Pending,
                expires_at_ms: 600_000,
                audit_ref: None,
                created_at_ms: 10,
                updated_at_ms: 10,
            })
            .await
            .unwrap();
        assert_eq!(
            store
                .pending_approvals(&publication_id, &subject, 20)
                .await
                .unwrap()
                .len(),
            1
        );
        store
            .decide_approval(
                &approval_id,
                &subject,
                ApprovalState::Approved,
                Some("audit-1"),
                21,
            )
            .await
            .unwrap();
        store
            .consume_approval(
                &approval_id,
                &publication_id,
                &authorization_id,
                &subject,
                &format!("client-{authorization_id}"),
                "issue_close",
                "git/issue.close",
                "connection-1",
                "digest",
                22,
            )
            .await
            .unwrap();
        store
            .set_publication_state(
                &publication_id,
                &tenant_id,
                &subject,
                PublicationState::Revoked,
                30,
            )
            .await
            .unwrap();
        assert!(
            store
                .client_authorizations(&publication_id)
                .await
                .unwrap()
                .iter()
                .all(|authorization| authorization.state == AuthorizationState::Revoked)
        );
    }
}
