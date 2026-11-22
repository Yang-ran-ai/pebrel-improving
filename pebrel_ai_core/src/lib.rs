//! UI-independent contracts shared by Pebrel's remote API transports and local agent hooks.
//!
//! This crate deliberately has no GPUI, process-management, or provider-credential
//! dependencies. It is the stable boundary between transport, session orchestration,
//! and presentation code.

use std::sync::Arc;

/// Stable identity for a configured AI provider.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProviderId(Arc<str>);

impl ProviderId {
    pub fn new(value: impl Into<Arc<str>>) -> Result<Self, AiError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(AiError::InvalidInput("provider id must not be empty".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Stable identity for one conversation, independent of a CLI session identifier.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SessionId(Arc<str>);

impl SessionId {
    pub fn new(value: impl Into<Arc<str>>) -> Result<Self, AiError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(AiError::InvalidInput("session id must not be empty".into()));
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Wire protocol selected for a provider endpoint.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ProviderProtocol {
    OpenAiChatCompletions,
    AnthropicMessages,
    GoogleGenerateContent,
    AzureOpenAiChatCompletions,
}

/// Semantic role of a conversation message.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AiRole {
    System,
    User,
    Assistant,
    Tool,
}

/// A protocol-neutral conversation message.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AiMessage {
    pub role: AiRole,
    pub content: Arc<str>,
    pub name: Option<Arc<str>>,
    pub tool_call_id: Option<Arc<str>>,
}

impl AiMessage {
    pub fn new(role: AiRole, content: impl Into<Arc<str>>) -> Self {
        Self { role, content: content.into(), name: None, tool_call_id: None }
    }

    pub fn tool_result(
        tool_call_id: impl Into<Arc<str>>,
        name: impl Into<Arc<str>>,
        content: impl Into<Arc<str>>,
    ) -> Self {
        Self {
            role: AiRole::Tool,
            content: content.into(),
            name: Some(name.into()),
            tool_call_id: Some(tool_call_id.into()),
        }
    }
}

/// Tool schema passed to providers that support tool calling.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolDefinition {
    pub name: Arc<str>,
    pub description: Arc<str>,
    /// JSON Schema document. Kept as source text to avoid coupling the core
    /// contract to a particular JSON implementation.
    pub input_schema: Arc<str>,
}

/// Tool invocation returned by a model.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ToolCall {
    pub id: Arc<str>,
    pub name: Arc<str>,
    /// Raw JSON arguments; validation belongs to the tool execution boundary.
    pub arguments: Arc<str>,
}

/// Token accounting reported by a provider, when available.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cached_input_tokens: Option<u64>,
}

/// Why a model stopped generating.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FinishReason {
    Stop,
    Length,
    ToolCalls,
    ContentFilter,
    Unknown,
}

/// Provider-neutral request used by API transports.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatRequest {
    pub session_id: SessionId,
    pub model: Arc<str>,
    pub messages: Vec<AiMessage>,
    pub tools: Vec<ToolDefinition>,
    pub temperature_milli: Option<u16>,
    pub max_output_tokens: Option<u32>,
}

impl ChatRequest {
    pub fn new(session_id: SessionId, model: impl Into<Arc<str>>, messages: Vec<AiMessage>) -> Self {
        Self {
            session_id,
            model: model.into(),
            messages,
            tools: Vec::new(),
            temperature_milli: None,
            max_output_tokens: None,
        }
    }
}

/// Provider-neutral completed response.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChatResponse {
    pub message: AiMessage,
    pub tool_calls: Vec<ToolCall>,
    pub usage: Option<TokenUsage>,
    pub finish_reason: FinishReason,
}

/// Events emitted by either an API runtime or a local-agent hook.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiEvent {
    SessionStarted { provider: ProviderId, session_id: SessionId },
    MessageStarted { provider: ProviderId, session_id: SessionId },
    MessageDelta { provider: ProviderId, session_id: SessionId, text: Arc<str> },
    ToolCallStarted { provider: ProviderId, session_id: SessionId, call: ToolCall },
    ToolCallFinished { provider: ProviderId, session_id: SessionId, call_id: Arc<str> },
    MessageCompleted {
        provider: ProviderId,
        session_id: SessionId,
        response: ChatResponse,
    },
    UsageUpdated { provider: ProviderId, session_id: SessionId, usage: TokenUsage },
    Failed { provider: ProviderId, session_id: Option<SessionId>, error: AiError },
    SessionEnded { provider: ProviderId, session_id: SessionId },
}

/// Errors crossing the transport/session boundary. Provider secrets and raw
/// request headers must never be included in these values.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AiError {
    InvalidInput(String),
    MissingCredential,
    UnsupportedProtocol,
    Transport(String),
    HttpStatus { status: u16, message: String },
    InvalidResponse(String),
    Cancelled,
}

impl std::fmt::Display for AiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(formatter, "Invalid AI request: {message}"),
            Self::MissingCredential => formatter.write_str("The provider API key is not configured"),
            Self::UnsupportedProtocol => formatter.write_str("The provider protocol is not supported"),
            Self::Transport(message) => write!(formatter, "AI transport failed: {message}"),
            Self::HttpStatus { status, message } => write!(formatter, "AI provider returned HTTP {status}: {message}"),
            Self::InvalidResponse(message) => write!(formatter, "Invalid AI provider response: {message}"),
            Self::Cancelled => formatter.write_str("AI request was cancelled"),
        }
    }
}

impl std::error::Error for AiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provider_and_session_ids_reject_blank_values() {
        assert!(ProviderId::new("  ").is_err());
        assert!(SessionId::new("").is_err());
        assert_eq!(ProviderId::new("deepseek").unwrap().as_str(), "deepseek");
        assert_eq!(SessionId::new("session-1").unwrap().as_str(), "session-1");
    }

    #[test]
    fn tool_results_keep_the_call_identity() {
        let message = AiMessage::tool_result("call-1", "read_file", "{\"ok\":true}");
        assert_eq!(message.role, AiRole::Tool);
        assert_eq!(message.tool_call_id.as_deref(), Some("call-1"));
        assert_eq!(message.name.as_deref(), Some("read_file"));
    }

    #[test]
    fn requests_start_with_no_implicit_tools_or_generation_limits() {
        let session = SessionId::new("session-1").unwrap();
        let request = ChatRequest::new(
            session,
            "deepseek-flash",
            vec![AiMessage::new(AiRole::User, "hello")],
        );
        assert!(request.tools.is_empty());
        assert_eq!(request.temperature_milli, None);
        assert_eq!(request.max_output_tokens, None);
    }
}
