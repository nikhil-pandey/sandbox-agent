//! Copilot CLI event converter
//!
//! Converts Copilot CLI session events to universal schema events.
//! Copilot CLI uses JSON-RPC with session events as notifications.

use std::sync::atomic::{AtomicU64, Ordering};

use serde_json::Value;

use crate::{
    ContentPart, ErrorData, EventConversion, ItemDeltaData, ItemEventData, ItemKind, ItemRole,
    ItemStatus, ReasoningVisibility, SessionStartedData, UniversalEventData, UniversalEventType,
    UniversalItem,
};

static TEMP_ID: AtomicU64 = AtomicU64::new(1);

fn next_temp_id(prefix: &str) -> String {
    let id = TEMP_ID.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{id}")
}

/// Convert a Copilot CLI session event to universal events.
///
/// Copilot CLI events have the structure:
/// ```json
/// {
///   "id": "uuid",
///   "timestamp": "iso8601",
///   "parentId": "uuid" | null,
///   "ephemeral": bool,
///   "type": "event.type",
///   "data": { ... }
/// }
/// ```
pub fn event_to_universal(event: &Value) -> Result<Vec<EventConversion>, String> {
    let event_type = event.get("type").and_then(Value::as_str).unwrap_or("");
    let data = event.get("data").cloned().unwrap_or(Value::Null);
    let raw = Some(event.clone());

    let conversions = match event_type {
        // Session lifecycle events
        "session.start" => vec![session_start_to_universal(&data, raw.clone())],
        "session.resume" => vec![session_resume_to_universal(&data, raw.clone())],
        "session.error" => vec![session_error_to_universal(&data, raw.clone())],
        "session.idle" => vec![status_event("session.idle", None, raw.clone())],
        "session.info" => {
            let info_type = data.get("infoType").and_then(Value::as_str).unwrap_or("");
            let msg = data.get("message").and_then(Value::as_str).unwrap_or("");
            let detail = format!("{}: {}", info_type, msg);
            vec![status_event("session.info", Some(detail), raw.clone())]
        }
        "session.model_change" => {
            let prev = data.get("previousModel").and_then(Value::as_str).unwrap_or("none");
            let new = data.get("newModel").and_then(Value::as_str).unwrap_or("unknown");
            let detail = format!("{} -> {}", prev, new);
            vec![status_event("session.model_change", Some(detail), raw.clone())]
        }
        "session.handoff" => vec![status_event("session.handoff", None, raw.clone())],
        "session.truncation" => vec![status_event("session.truncation", None, raw.clone())],
        "session.usage_info" => vec![status_event("session.usage_info", None, raw.clone())],
        "session.compaction_start" => vec![status_event("session.compaction_start", None, raw.clone())],
        "session.compaction_complete" => {
            let success = data.get("success").and_then(Value::as_bool).unwrap_or(false);
            let detail = if success { "success" } else { "failed" };
            vec![status_event("session.compaction_complete", Some(detail.to_string()), raw.clone())]
        }
        "session.import_legacy" => vec![status_event("session.import_legacy", None, raw.clone())],
        "session.snapshot_rewind" => vec![status_event("session.snapshot_rewind", None, raw.clone())],
        "session.shutdown" => {
            let shutdown_type = data.get("shutdownType").and_then(Value::as_str).unwrap_or("unknown");
            vec![status_event("session.shutdown", Some(shutdown_type.to_string()), raw.clone())]
        }

        // User messages
        "user.message" => vec![user_message_to_universal(&data, raw.clone())],
        "pending_messages.modified" => Vec::new(), // Ephemeral, ignore

        // Assistant messages
        "assistant.turn_start" => {
            let turn_id = data.get("turnId").and_then(Value::as_str);
            vec![status_event("assistant.turn_start", turn_id.map(String::from), raw.clone())]
        }
        "assistant.intent" => {
            let intent = data.get("intent").and_then(Value::as_str);
            vec![status_event("assistant.intent", intent.map(String::from), raw.clone())]
        }
        "assistant.reasoning" => vec![reasoning_to_universal(&data, raw.clone())],
        "assistant.reasoning_delta" => vec![reasoning_delta_to_universal(&data, raw.clone())],
        "assistant.message" => assistant_message_to_universal(&data, raw.clone()),
        "assistant.message_delta" => vec![message_delta_to_universal(&data, raw.clone())],
        "assistant.turn_end" => {
            let turn_id = data.get("turnId").and_then(Value::as_str);
            vec![status_event("assistant.turn_end", turn_id.map(String::from), raw.clone())]
        }
        "assistant.usage" => {
            // Extract usage details for richer status
            let model = data.get("model").and_then(Value::as_str).unwrap_or("unknown");
            let input = data.get("inputTokens").and_then(Value::as_u64).unwrap_or(0);
            let output = data.get("outputTokens").and_then(Value::as_u64).unwrap_or(0);
            let detail = format!("{}: {}in/{}out", model, input, output);
            vec![status_event("assistant.usage", Some(detail), raw.clone())]
        }

        // Tool execution
        "tool.user_requested" => vec![tool_start_to_universal(&data, raw.clone(), true)],
        "tool.execution_start" => vec![tool_start_to_universal(&data, raw.clone(), false)],
        "tool.execution_partial_result" => vec![tool_partial_to_universal(&data, raw.clone())],
        "tool.execution_progress" => {
            let msg = data.get("progressMessage").and_then(Value::as_str);
            vec![status_event("tool.progress", msg.map(String::from), raw.clone())]
        }
        "tool.execution_complete" => vec![tool_complete_to_universal(&data, raw.clone())],

        // Subagents - capture full context
        "subagent.started" => {
            let tool_call_id = data.get("toolCallId").and_then(Value::as_str).unwrap_or("");
            let name = data.get("agentDisplayName").and_then(Value::as_str).unwrap_or("");
            let agent_name = data.get("agentName").and_then(Value::as_str).unwrap_or("");
            let detail = format!("{} ({}) [{}]", name, agent_name, tool_call_id);
            vec![status_event("subagent.started", Some(detail), raw.clone())]
        }
        "subagent.completed" => {
            let tool_call_id = data.get("toolCallId").and_then(Value::as_str).unwrap_or("");
            let name = data.get("agentName").and_then(Value::as_str).unwrap_or("");
            let detail = format!("{} [{}]", name, tool_call_id);
            vec![status_event("subagent.completed", Some(detail), raw.clone())]
        }
        "subagent.failed" => {
            let tool_call_id = data.get("toolCallId").and_then(Value::as_str).unwrap_or("");
            let name = data.get("agentName").and_then(Value::as_str).unwrap_or("");
            let error = data.get("error").and_then(Value::as_str).unwrap_or("");
            let detail = format!("{} [{}]: {}", name, tool_call_id, error);
            vec![status_event("subagent.failed", Some(detail), raw.clone())]
        }
        "subagent.selected" => {
            let name = data.get("agentDisplayName").and_then(Value::as_str).unwrap_or("");
            let agent_name = data.get("agentName").and_then(Value::as_str).unwrap_or("");
            let tools = data.get("tools").cloned();
            let tools_str = match tools {
                Some(Value::Array(arr)) => arr.iter()
                    .filter_map(|v| v.as_str())
                    .collect::<Vec<_>>()
                    .join(", "),
                Some(Value::Null) | None => "*".to_string(),
                _ => "?".to_string(),
            };
            let detail = format!("{} ({}) tools=[{}]", name, agent_name, tools_str);
            vec![status_event("subagent.selected", Some(detail), raw.clone())]
        }

        // Skills
        "skill.invoked" => {
            let name = data.get("name").and_then(Value::as_str).unwrap_or("");
            let path = data.get("path").and_then(Value::as_str).unwrap_or("");
            let detail = format!("{} ({})", name, path);
            vec![status_event("skill.invoked", Some(detail), raw.clone())]
        }

        // System & hooks
        "system.message" => vec![system_message_to_universal(&data, raw.clone())],
        "hook.start" | "hook.end" => Vec::new(), // Internal, ignore
        "abort" => {
            let reason = data.get("reason").and_then(Value::as_str).unwrap_or("aborted");
            vec![error_event(reason, None, raw.clone())]
        }

        // Unknown event type
        _ => return Err(format!("unsupported Copilot event type: {event_type}")),
    };

    Ok(conversions)
}

fn session_start_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    let session_id = data.get("sessionId").and_then(Value::as_str).map(String::from);
    EventConversion::new(
        UniversalEventType::SessionStarted,
        UniversalEventData::SessionStarted(SessionStartedData {
            metadata: Some(data.clone()),
        }),
    )
    .with_native_session(session_id)
    .with_raw(raw)
}

fn session_resume_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    EventConversion::new(
        UniversalEventType::SessionStarted,
        UniversalEventData::SessionStarted(SessionStartedData {
            metadata: Some(data.clone()),
        }),
    )
    .with_raw(raw)
    .synthetic()
}

fn session_error_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    let message = data
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("Unknown error")
        .to_string();
    let code = data.get("errorType").and_then(Value::as_str).map(String::from);

    EventConversion::new(
        UniversalEventType::Error,
        UniversalEventData::Error(ErrorData {
            message,
            code,
            details: Some(data.clone()),
        }),
    )
    .with_raw(raw)
}

fn user_message_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    // Prefer transformedContent if available (includes datetime prefix etc.)
    let content = data
        .get("transformedContent")
        .and_then(Value::as_str)
        .or_else(|| data.get("content").and_then(Value::as_str))
        .unwrap_or("")
        .to_string();

    let mut parts = vec![ContentPart::Text { text: content }];

    // Add file attachments if present
    if let Some(attachments) = data.get("attachments").and_then(Value::as_array) {
        for attachment in attachments {
            if let Some(path) = attachment.get("path").and_then(Value::as_str) {
                let attachment_type = attachment.get("type").and_then(Value::as_str).unwrap_or("file");
                let display_name = attachment.get("displayName").and_then(Value::as_str).unwrap_or(path);
                // Represent attachments as file references with read action
                parts.push(ContentPart::FileRef {
                    path: path.to_string(),
                    action: crate::FileAction::Read,
                    diff: Some(format!("[{}: {}]", attachment_type, display_name)),
                });
            }
        }
    }

    let item = UniversalItem {
        item_id: next_temp_id("copilot_user"),
        native_item_id: None,
        parent_id: None,
        kind: ItemKind::Message,
        role: Some(ItemRole::User),
        content: parts,
        status: ItemStatus::Completed,
    };

    EventConversion::new(
        UniversalEventType::ItemCompleted,
        UniversalEventData::Item(ItemEventData { item }),
    )
    .with_raw(raw)
}

fn assistant_message_to_universal(data: &Value, raw: Option<Value>) -> Vec<EventConversion> {
    let mut conversions = Vec::new();
    let message_id = data
        .get("messageId")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| next_temp_id("copilot_msg"));

    let content = data
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    // Get parent tool call ID for subagent responses
    let parent_id = data
        .get("parentToolCallId")
        .and_then(Value::as_str)
        .map(String::from);

    // Handle reasoning if present (reasoningText is the decrypted version)
    let reasoning_text = data.get("reasoningText").and_then(Value::as_str);

    // Handle tool requests
    if let Some(tool_requests) = data.get("toolRequests").and_then(Value::as_array) {
        let mut parts = Vec::new();

        // Add reasoning if present
        if let Some(reasoning) = reasoning_text {
            if !reasoning.is_empty() {
                parts.push(ContentPart::Reasoning {
                    text: reasoning.to_string(),
                    visibility: ReasoningVisibility::Public,
                });
            }
        }

        // Add text content if present
        if !content.is_empty() {
            parts.push(ContentPart::Text { text: content.clone() });
        }

        // Add tool calls
        for req in tool_requests {
            let name = req.get("name").and_then(Value::as_str).unwrap_or("unknown");
            let call_id = req
                .get("toolCallId")
                .and_then(Value::as_str)
                .map(String::from)
                .unwrap_or_else(|| next_temp_id("copilot_tool"));
            let arguments = req.get("arguments").cloned().unwrap_or(Value::Null);

            parts.push(ContentPart::ToolCall {
                name: name.to_string(),
                arguments: serde_json::to_string(&arguments).unwrap_or_default(),
                call_id,
            });
        }

        let item = UniversalItem {
            item_id: message_id.clone(),
            native_item_id: Some(message_id),
            parent_id: parent_id.clone(),
            kind: ItemKind::Message,
            role: Some(ItemRole::Assistant),
            content: parts,
            status: ItemStatus::Completed,
        };

        conversions.push(
            EventConversion::new(
                UniversalEventType::ItemCompleted,
                UniversalEventData::Item(ItemEventData { item }),
            )
            .with_raw(raw),
        );
    } else if !content.is_empty() || reasoning_text.is_some() {
        // Plain text message (possibly with reasoning)
        let mut parts = Vec::new();

        if let Some(reasoning) = reasoning_text {
            if !reasoning.is_empty() {
                parts.push(ContentPart::Reasoning {
                    text: reasoning.to_string(),
                    visibility: ReasoningVisibility::Public,
                });
            }
        }

        if !content.is_empty() {
            parts.push(ContentPart::Text { text: content });
        }

        let item = UniversalItem {
            item_id: message_id.clone(),
            native_item_id: Some(message_id),
            parent_id,
            kind: ItemKind::Message,
            role: Some(ItemRole::Assistant),
            content: parts,
            status: ItemStatus::Completed,
        };

        conversions.push(
            EventConversion::new(
                UniversalEventType::ItemCompleted,
                UniversalEventData::Item(ItemEventData { item }),
            )
            .with_raw(raw),
        );
    }

    conversions
}

fn message_delta_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    let message_id = data
        .get("messageId")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_default();
    let delta = data
        .get("deltaContent")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    EventConversion::new(
        UniversalEventType::ItemDelta,
        UniversalEventData::ItemDelta(ItemDeltaData {
            item_id: message_id.clone(),
            native_item_id: Some(message_id),
            delta,
        }),
    )
    .with_raw(raw)
}

fn reasoning_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    let reasoning_id = data
        .get("reasoningId")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| next_temp_id("copilot_reasoning"));
    let content = data
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let item = UniversalItem {
        item_id: reasoning_id.clone(),
        native_item_id: Some(reasoning_id),
        parent_id: None,
        kind: ItemKind::Message,
        role: Some(ItemRole::Assistant),
        content: vec![ContentPart::Reasoning {
            text: content,
            visibility: ReasoningVisibility::Public,
        }],
        status: ItemStatus::Completed,
    };

    EventConversion::new(
        UniversalEventType::ItemCompleted,
        UniversalEventData::Item(ItemEventData { item }),
    )
    .with_raw(raw)
}

fn reasoning_delta_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    let reasoning_id = data
        .get("reasoningId")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_default();
    let delta = data
        .get("deltaContent")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    EventConversion::new(
        UniversalEventType::ItemDelta,
        UniversalEventData::ItemDelta(ItemDeltaData {
            item_id: reasoning_id.clone(),
            native_item_id: Some(reasoning_id),
            delta,
        }),
    )
    .with_raw(raw)
}

fn tool_start_to_universal(data: &Value, raw: Option<Value>, user_requested: bool) -> EventConversion {
    let call_id = data
        .get("toolCallId")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| next_temp_id("copilot_tool"));
    let name = data
        .get("toolName")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();
    let arguments = data.get("arguments").cloned().unwrap_or(Value::Null);

    let mut item = UniversalItem {
        item_id: call_id.clone(),
        native_item_id: Some(call_id.clone()),
        parent_id: None,
        kind: ItemKind::ToolCall,
        role: Some(ItemRole::Assistant),
        content: vec![ContentPart::ToolCall {
            name,
            arguments: serde_json::to_string(&arguments).unwrap_or_default(),
            call_id,
        }],
        status: ItemStatus::InProgress,
    };

    // Handle parent tool call for subagent tool calls
    if let Some(parent_id) = data.get("parentToolCallId").and_then(Value::as_str) {
        item.parent_id = Some(parent_id.to_string());
    }

    let mut conversion = EventConversion::new(
        UniversalEventType::ItemStarted,
        UniversalEventData::Item(ItemEventData { item }),
    )
    .with_raw(raw);

    if user_requested {
        conversion = conversion.synthetic();
    }

    conversion
}

fn tool_partial_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    let call_id = data
        .get("toolCallId")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_default();
    let partial = data
        .get("partialOutput")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    EventConversion::new(
        UniversalEventType::ItemDelta,
        UniversalEventData::ItemDelta(ItemDeltaData {
            item_id: call_id.clone(),
            native_item_id: Some(call_id),
            delta: partial,
        }),
    )
    .with_raw(raw)
}

fn tool_complete_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    let call_id = data
        .get("toolCallId")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| next_temp_id("copilot_tool"));
    let success = data.get("success").and_then(Value::as_bool).unwrap_or(false);

    let output = if success {
        // Try detailedContent first, then content
        data.get("result")
            .and_then(|r| {
                r.get("detailedContent")
                    .and_then(Value::as_str)
                    .or_else(|| r.get("content").and_then(Value::as_str))
            })
            .unwrap_or("")
            .to_string()
    } else {
        // Get error message and optionally error code
        let msg = data.get("error")
            .and_then(|e| e.get("message"))
            .and_then(Value::as_str)
            .unwrap_or("Tool execution failed");
        let code = data.get("error")
            .and_then(|e| e.get("code"))
            .and_then(Value::as_str);
        match code {
            Some(c) => format!("[{}] {}", c, msg),
            None => msg.to_string(),
        }
    };

    let mut item = UniversalItem {
        item_id: next_temp_id("copilot_result"),
        native_item_id: Some(call_id.clone()),
        parent_id: None,
        kind: ItemKind::ToolResult,
        role: Some(ItemRole::Tool),
        content: vec![ContentPart::ToolResult {
            call_id,
            output,
        }],
        status: if success {
            ItemStatus::Completed
        } else {
            ItemStatus::Failed
        },
    };

    // Handle parent tool call
    if let Some(parent_id) = data.get("parentToolCallId").and_then(Value::as_str) {
        item.parent_id = Some(parent_id.to_string());
    }

    EventConversion::new(
        UniversalEventType::ItemCompleted,
        UniversalEventData::Item(ItemEventData { item }),
    )
    .with_raw(raw)
}

fn system_message_to_universal(data: &Value, raw: Option<Value>) -> EventConversion {
    let content = data
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    // Handle role: "system" or "developer"
    let role_str = data.get("role").and_then(Value::as_str).unwrap_or("system");
    let role = if role_str == "developer" {
        // Developer messages are still system-level but from the developer
        // We don't have a separate Developer role, so use System
        ItemRole::System
    } else {
        ItemRole::System
    };

    // Get optional name for the message source
    let name = data.get("name").and_then(Value::as_str);
    let item_id = match name {
        Some(n) => format!("copilot_system_{}", n),
        None => next_temp_id("copilot_system"),
    };

    let item = UniversalItem {
        item_id,
        native_item_id: None,
        parent_id: None,
        kind: ItemKind::System,
        role: Some(role),
        content: vec![ContentPart::Text { text: content }],
        status: ItemStatus::Completed,
    };

    EventConversion::new(
        UniversalEventType::ItemCompleted,
        UniversalEventData::Item(ItemEventData { item }),
    )
    .with_raw(raw)
}

fn status_event(label: &str, detail: Option<String>, raw: Option<Value>) -> EventConversion {
    let item = UniversalItem {
        item_id: next_temp_id("copilot_status"),
        native_item_id: None,
        parent_id: None,
        kind: ItemKind::Status,
        role: None,
        content: vec![ContentPart::Status {
            label: label.to_string(),
            detail,
        }],
        status: ItemStatus::Completed,
    };

    EventConversion::new(
        UniversalEventType::ItemCompleted,
        UniversalEventData::Item(ItemEventData { item }),
    )
    .with_raw(raw)
}

fn error_event(message: &str, code: Option<String>, raw: Option<Value>) -> EventConversion {
    EventConversion::new(
        UniversalEventType::Error,
        UniversalEventData::Error(ErrorData {
            message: message.to_string(),
            code,
            details: raw.clone(),
        }),
    )
    .with_raw(raw)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_session_start() {
        let event = json!({
            "type": "session.start",
            "data": {
                "sessionId": "test-session-123",
                "version": 1,
                "producer": "copilot-agent",
                "copilotVersion": "0.0.401-1",
                "startTime": "2026-02-03T00:00:00Z"
            },
            "id": "event-1",
            "timestamp": "2026-02-03T00:00:00Z",
            "parentId": null
        });

        let conversions = event_to_universal(&event).unwrap();
        assert_eq!(conversions.len(), 1);
        assert_eq!(conversions[0].event_type, UniversalEventType::SessionStarted);
        assert_eq!(conversions[0].native_session_id, Some("test-session-123".to_string()));
    }

    #[test]
    fn test_user_message() {
        let event = json!({
            "type": "user.message",
            "data": {
                "content": "Hello, world!"
            },
            "id": "event-2",
            "timestamp": "2026-02-03T00:00:01Z",
            "parentId": null
        });

        let conversions = event_to_universal(&event).unwrap();
        assert_eq!(conversions.len(), 1);
        assert_eq!(conversions[0].event_type, UniversalEventType::ItemCompleted);
    }

    #[test]
    fn test_assistant_message_with_tools() {
        let event = json!({
            "type": "assistant.message",
            "data": {
                "messageId": "msg-1",
                "content": "",
                "toolRequests": [{
                    "toolCallId": "tool-1",
                    "name": "view",
                    "arguments": {"path": "/home/user"},
                    "type": "function"
                }]
            },
            "id": "event-3",
            "timestamp": "2026-02-03T00:00:02Z",
            "parentId": null
        });

        let conversions = event_to_universal(&event).unwrap();
        assert_eq!(conversions.len(), 1);
        assert_eq!(conversions[0].event_type, UniversalEventType::ItemCompleted);
    }

    #[test]
    fn test_tool_execution() {
        let start_event = json!({
            "type": "tool.execution_start",
            "data": {
                "toolCallId": "tool-1",
                "toolName": "view",
                "arguments": {"path": "/home/user"}
            },
            "id": "event-4",
            "timestamp": "2026-02-03T00:00:03Z",
            "parentId": null
        });

        let conversions = event_to_universal(&start_event).unwrap();
        assert_eq!(conversions.len(), 1);
        assert_eq!(conversions[0].event_type, UniversalEventType::ItemStarted);

        let complete_event = json!({
            "type": "tool.execution_complete",
            "data": {
                "toolCallId": "tool-1",
                "success": true,
                "result": {"content": "file1\nfile2"}
            },
            "id": "event-5",
            "timestamp": "2026-02-03T00:00:04Z",
            "parentId": null
        });

        let conversions = event_to_universal(&complete_event).unwrap();
        assert_eq!(conversions.len(), 1);
        assert_eq!(conversions[0].event_type, UniversalEventType::ItemCompleted);
    }

    #[test]
    fn test_session_error() {
        let event = json!({
            "type": "session.error",
            "data": {
                "errorType": "api_error",
                "message": "Something went wrong"
            },
            "id": "event-6",
            "timestamp": "2026-02-03T00:00:05Z",
            "parentId": null
        });

        let conversions = event_to_universal(&event).unwrap();
        assert_eq!(conversions.len(), 1);
        assert_eq!(conversions[0].event_type, UniversalEventType::Error);
    }

    #[test]
    fn test_user_message_with_attachments() {
        let event = json!({
            "type": "user.message",
            "data": {
                "content": "Check this file",
                "transformedContent": "<current_datetime>...</current_datetime>\n\nCheck this file",
                "attachments": [{
                    "type": "file",
                    "path": "/home/user/test.rs",
                    "displayName": "test.rs"
                }]
            },
            "id": "event-7",
            "timestamp": "2026-02-03T00:00:06Z",
            "parentId": null
        });

        let conversions = event_to_universal(&event).unwrap();
        assert_eq!(conversions.len(), 1);
        if let UniversalEventData::Item(ref item_data) = conversions[0].data {
            // Should have text + file attachment
            assert_eq!(item_data.item.content.len(), 2);
        } else {
            panic!("Expected Item data");
        }
    }

    #[test]
    fn test_assistant_message_with_parent() {
        let event = json!({
            "type": "assistant.message",
            "data": {
                "messageId": "msg-2",
                "content": "Subagent response",
                "parentToolCallId": "parent-tool-1"
            },
            "id": "event-8",
            "timestamp": "2026-02-03T00:00:07Z",
            "parentId": null
        });

        let conversions = event_to_universal(&event).unwrap();
        assert_eq!(conversions.len(), 1);
        if let UniversalEventData::Item(ref item_data) = conversions[0].data {
            assert_eq!(item_data.item.parent_id, Some("parent-tool-1".to_string()));
        } else {
            panic!("Expected Item data");
        }
    }

    #[test]
    fn test_assistant_message_with_reasoning() {
        let event = json!({
            "type": "assistant.message",
            "data": {
                "messageId": "msg-3",
                "content": "Here's the answer",
                "reasoningText": "I need to think about this..."
            },
            "id": "event-9",
            "timestamp": "2026-02-03T00:00:08Z",
            "parentId": null
        });

        let conversions = event_to_universal(&event).unwrap();
        assert_eq!(conversions.len(), 1);
        if let UniversalEventData::Item(ref item_data) = conversions[0].data {
            // Should have reasoning + text
            assert_eq!(item_data.item.content.len(), 2);
        } else {
            panic!("Expected Item data");
        }
    }

    #[test]
    fn test_missing_event_types() {
        // Test new event types that were missing
        let events = vec![
            json!({"type": "session.import_legacy", "data": {}, "id": "1", "timestamp": "2026-02-03T00:00:00Z", "parentId": null}),
            json!({"type": "session.snapshot_rewind", "data": {}, "id": "2", "timestamp": "2026-02-03T00:00:00Z", "parentId": null}),
            json!({"type": "session.shutdown", "data": {"shutdownType": "normal"}, "id": "3", "timestamp": "2026-02-03T00:00:00Z", "parentId": null}),
            json!({"type": "skill.invoked", "data": {"name": "test-skill", "path": "/skills/test"}, "id": "4", "timestamp": "2026-02-03T00:00:00Z", "parentId": null}),
        ];

        for event in events {
            let result = event_to_universal(&event);
            assert!(result.is_ok(), "Event type {:?} should be handled", event.get("type"));
        }
    }

    #[test]
    fn test_subagent_events() {
        let started = json!({
            "type": "subagent.started",
            "data": {
                "toolCallId": "tool-123",
                "agentName": "explore",
                "agentDisplayName": "Explore Agent",
                "agentDescription": "Explores the codebase"
            },
            "id": "1",
            "timestamp": "2026-02-03T00:00:00Z",
            "parentId": null
        });

        let conversions = event_to_universal(&started).unwrap();
        assert_eq!(conversions.len(), 1);
        if let UniversalEventData::Item(ref item_data) = conversions[0].data {
            // Check that the detail contains all the important info
            if let ContentPart::Status { ref detail, .. } = item_data.item.content[0] {
                let d = detail.as_ref().unwrap();
                assert!(d.contains("Explore Agent"));
                assert!(d.contains("explore"));
                assert!(d.contains("tool-123"));
            }
        }
    }

    #[test]
    fn test_system_message_with_role() {
        let event = json!({
            "type": "system.message",
            "data": {
                "content": "System prompt content",
                "role": "developer",
                "name": "custom_instructions"
            },
            "id": "1",
            "timestamp": "2026-02-03T00:00:00Z",
            "parentId": null
        });

        let conversions = event_to_universal(&event).unwrap();
        assert_eq!(conversions.len(), 1);
        if let UniversalEventData::Item(ref item_data) = conversions[0].data {
            assert_eq!(item_data.item.kind, ItemKind::System);
            assert!(item_data.item.item_id.contains("custom_instructions"));
        }
    }
}
