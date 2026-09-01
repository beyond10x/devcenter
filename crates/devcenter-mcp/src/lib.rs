//! Credential-free MCP protocol and direct capability-profile projection.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";
pub const SUPPORTED_PROTOCOL_VERSIONS: [&str; 2] = ["2025-11-25", "2025-06-18"];

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Effect {
    ReadOnly,
    Idempotent,
    Mutation,
    Destructive,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPosture {
    NotRequired,
    Required,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CompiledTool {
    pub name: String,
    pub title: String,
    pub description: String,
    pub operation_ref: String,
    pub connection_id: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub effect: Effect,
    pub approval: ApprovalPosture,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProjectionError {
    DuplicateName,
    InvalidField,
    InvalidSchema,
}

#[derive(Clone, Debug)]
pub struct Toolset {
    tools: Vec<CompiledTool>,
    digest: String,
}

impl Toolset {
    pub fn compile(mut tools: Vec<CompiledTool>) -> Result<Self, ProjectionError> {
        let mut names = BTreeSet::new();
        for tool in &tools {
            if !valid_tool_name(&tool.name)
                || tool.title.trim().is_empty()
                || tool.description.trim().is_empty()
                || tool.operation_ref.trim().is_empty()
                || tool.connection_id.trim().is_empty()
            {
                return Err(ProjectionError::InvalidField);
            }
            if !names.insert(tool.name.clone()) {
                return Err(ProjectionError::DuplicateName);
            }
            if !is_object_schema(&tool.input_schema) || !is_object_schema(&tool.output_schema) {
                return Err(ProjectionError::InvalidSchema);
            }
        }
        tools.sort_by(|left, right| left.name.cmp(&right.name));
        let bytes = serde_json::to_vec(&tools).map_err(|_| ProjectionError::InvalidField)?;
        let digest = hex_digest(&Sha256::digest(bytes));
        Ok(Self { tools, digest })
    }

    pub fn digest(&self) -> &str {
        &self.digest
    }

    pub fn tools(&self) -> &[CompiledTool] {
        &self.tools
    }

    pub fn find(&self, name: &str) -> Option<&CompiledTool> {
        self.tools
            .binary_search_by_key(&name, |tool| tool.name.as_str())
            .ok()
            .map(|index| &self.tools[index])
    }

    fn list_result(&self) -> Value {
        let tools = self
            .tools
            .iter()
            .map(|tool| {
                let read_only = tool.effect == Effect::ReadOnly;
                let destructive = tool.effect == Effect::Destructive;
                json!({
                    "name": tool.name,
                    "title": tool.title,
                    "description": tool.description,
                    "inputSchema": tool.input_schema,
                    "outputSchema": tool.output_schema,
                    "annotations": {
                        "readOnlyHint": read_only,
                        "destructiveHint": destructive,
                        "idempotentHint": matches!(tool.effect, Effect::ReadOnly | Effect::Idempotent)
                    },
                    "_meta": {
                        "securitySchemes": [{"type": "oauth2", "scopes": ["mcp.tools.call"]}],
                        "devcenter/operation": tool.operation_ref,
                        "devcenter/connection": tool.connection_id,
                        "devcenter/effect": tool.effect,
                        "devcenter/approval": tool.approval
                    }
                })
            })
            .collect::<Vec<_>>();
        json!({"tools": tools})
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    #[serde(default)]
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ToolCall {
    pub request_id: Value,
    pub tool: CompiledTool,
    pub arguments: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Outcome {
    Reply(Value),
    AcceptedNotification,
    Call(Box<ToolCall>),
}

pub fn handle(request: Request, tools: &Toolset) -> Outcome {
    if request.jsonrpc != "2.0" {
        return Outcome::Reply(error(request.id.as_ref(), -32_600, "invalid request"));
    }
    let Some(id) = request.id else {
        return Outcome::AcceptedNotification;
    };
    match request.method.as_str() {
        "initialize" => initialize(&id, request.params.as_ref()),
        "ping" => Outcome::Reply(success(&id, &json!({}))),
        "tools/list" => Outcome::Reply(success(&id, &tools.list_result())),
        "tools/call" => tool_call(id, request.params, tools),
        _ => Outcome::Reply(error(Some(&id), -32_601, "method not found")),
    }
}

pub fn call_success(id: &Value, content: &Value) -> Value {
    success(
        id,
        &json!({
            "content": [{"type": "text", "text": content.to_string()}],
            "structuredContent": content,
            "isError": false
        }),
    )
}

pub fn call_error(id: &Value, code: &str, message: &str, details: &Value) -> Value {
    success(
        id,
        &json!({
            "content": [{"type": "text", "text": message}],
            "structuredContent": {"code": code, "details": details},
            "isError": true
        }),
    )
}

fn initialize(id: &Value, params: Option<&Value>) -> Outcome {
    let requested = params
        .and_then(|value| value.get("protocolVersion"))
        .and_then(Value::as_str)
        .unwrap_or(LATEST_PROTOCOL_VERSION);
    let protocol = if SUPPORTED_PROTOCOL_VERSIONS.contains(&requested) {
        requested
    } else {
        LATEST_PROTOCOL_VERSION
    };
    Outcome::Reply(success(
        id,
        &json!({
            "protocolVersion": protocol,
            "capabilities": {"tools": {"listChanged": true}},
            "serverInfo": {"name": "devcenter", "version": env!("CARGO_PKG_VERSION")}
        }),
    ))
}

fn tool_call(id: Value, params: Option<Value>, tools: &Toolset) -> Outcome {
    let Some(params) = params else {
        return Outcome::Reply(error(Some(&id), -32_602, "invalid params"));
    };
    let Some(name) = params.get("name").and_then(Value::as_str) else {
        return Outcome::Reply(error(Some(&id), -32_602, "invalid params"));
    };
    let Some(tool) = tools.find(name) else {
        return Outcome::Reply(call_error(
            &id,
            "tool_not_available",
            "tool not available",
            &json!({}),
        ));
    };
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if !arguments.is_object() {
        return Outcome::Reply(error(Some(&id), -32_602, "invalid params"));
    }
    Outcome::Call(Box::new(ToolCall {
        request_id: id,
        tool: tool.clone(),
        arguments,
    }))
}

fn success(id: &Value, result: &Value) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "result": result})
}

fn error(id: Option<&Value>, code: i64, message: &str) -> Value {
    json!({"jsonrpc": "2.0", "id": id, "error": {"code": code, "message": message}})
}

fn valid_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
}

fn is_object_schema(schema: &Value) -> bool {
    schema.as_object().is_some_and(|object| {
        object.get("type").and_then(Value::as_str) == Some("object")
            && object.keys().all(|key| !key.trim().is_empty())
    })
}

fn hex_digest(digest: &[u8]) -> String {
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

pub fn canonical_input_digest(input: &Value) -> String {
    let canonical = canonicalize(input);
    hex_digest(&Sha256::digest(canonical.to_string().as_bytes()))
}

fn canonicalize(value: &Value) -> Value {
    match value {
        Value::Object(object) => Value::Object(
            object
                .iter()
                .map(|(key, value)| (key.clone(), canonicalize(value)))
                .collect::<BTreeMap<_, _>>()
                .into_iter()
                .collect(),
        ),
        Value::Array(values) => Value::Array(values.iter().map(canonicalize).collect()),
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tool(name: &str, effect: Effect, approval: ApprovalPosture) -> CompiledTool {
        CompiledTool {
            name: name.to_owned(),
            title: "Issue reader".to_owned(),
            description: "Read one issue.".to_owned(),
            operation_ref: "git/issue.get".to_owned(),
            connection_id: "connection-1".to_owned(),
            input_schema: json!({"type": "object", "properties": {"id": {"type":"string"}}}),
            output_schema: json!({"type": "object", "properties": {"title": {"type":"string"}}}),
            effect,
            approval,
        }
    }

    #[test]
    fn projection_is_sorted_deterministic_and_direct() {
        let left = Toolset::compile(vec![
            tool("z_read", Effect::ReadOnly, ApprovalPosture::NotRequired),
            tool("a_write", Effect::Mutation, ApprovalPosture::Required),
        ])
        .unwrap();
        let right = Toolset::compile(vec![
            tool("a_write", Effect::Mutation, ApprovalPosture::Required),
            tool("z_read", Effect::ReadOnly, ApprovalPosture::NotRequired),
        ])
        .unwrap();
        assert_eq!(left.digest(), right.digest());
        assert_eq!(left.tools()[0].name, "a_write");
        let Outcome::Reply(listed) = handle(
            Request {
                jsonrpc: "2.0".to_owned(),
                id: Some(json!(1)),
                method: "tools/list".to_owned(),
                params: None,
            },
            &left,
        ) else {
            panic!("reply");
        };
        assert_eq!(listed["result"]["tools"][0]["name"], "a_write");
        assert_eq!(
            listed["result"]["tools"][0]["annotations"]["destructiveHint"],
            false
        );
        assert_eq!(
            listed["result"]["tools"][1]["annotations"]["readOnlyHint"],
            true
        );
    }

    #[test]
    fn duplicates_and_malformed_schemas_are_refused() {
        let duplicate = tool("same", Effect::ReadOnly, ApprovalPosture::NotRequired);
        assert_eq!(
            Toolset::compile(vec![duplicate.clone(), duplicate]).unwrap_err(),
            ProjectionError::DuplicateName
        );
        let mut invalid = tool("invalid", Effect::ReadOnly, ApprovalPosture::NotRequired);
        invalid.input_schema = json!({"type": "string"});
        assert_eq!(
            Toolset::compile(vec![invalid]).unwrap_err(),
            ProjectionError::InvalidSchema
        );
    }

    #[test]
    fn canonical_digest_ignores_object_key_order_but_not_input_changes() {
        assert_eq!(
            canonical_input_digest(&json!({"a": 1, "b": {"c": 2}})),
            canonical_input_digest(&json!({"b": {"c": 2}, "a": 1}))
        );
        assert_ne!(
            canonical_input_digest(&json!({"a": 1})),
            canonical_input_digest(&json!({"a": 2}))
        );
    }

    #[test]
    fn both_protocol_versions_initialize_and_calls_are_resolved() {
        let tools = Toolset::compile(vec![tool(
            "issue_get",
            Effect::ReadOnly,
            ApprovalPosture::NotRequired,
        )])
        .unwrap();
        for version in SUPPORTED_PROTOCOL_VERSIONS {
            let Outcome::Reply(reply) = handle(
                Request {
                    jsonrpc: "2.0".to_owned(),
                    id: Some(json!(1)),
                    method: "initialize".to_owned(),
                    params: Some(json!({"protocolVersion": version})),
                },
                &tools,
            ) else {
                panic!("reply");
            };
            assert_eq!(reply["result"]["protocolVersion"], version);
        }
        let Outcome::Call(call) = handle(
            Request {
                jsonrpc: "2.0".to_owned(),
                id: Some(json!(2)),
                method: "tools/call".to_owned(),
                params: Some(json!({"name": "issue_get", "arguments": {"id": "1"}})),
            },
            &tools,
        ) else {
            panic!("call");
        };
        assert_eq!(call.tool.operation_ref, "git/issue.get");
        assert_eq!(call.arguments, json!({"id": "1"}));
    }
}
