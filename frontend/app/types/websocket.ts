// Client messages
export type ClientMessage =
  | {
      type: "create_session";
      agent_id: string;
      project_path: string;
      model_config?: ModelConfig;
    }
  | { type: "send_prompt"; session_id: string; prompt: string }
  | { type: "close_session"; session_id: string }
  | { type: "list_agents" }
  | { type: "list_sessions" };

// Server messages
export type ServerMessage =
  | {
      type: "session_created";
      session_id: string;
      agent_name: string;
      model_config?: ModelConfig;
    }
  | { type: "prompt_accepted"; session_id: string }
  | { type: "content_delta"; session_id: string; content: string }
  | { type: "tool_call"; session_id: string; tool_name: string; args: unknown }
  | { type: "file_change"; session_id: string; path: string; action: string; content?: string; diff?: string }
  | { type: "token_usage"; session_id: string; usage: TokenUsage }
  | { type: "thinking"; session_id: string; content: string }
  | { type: "session_closed"; session_id: string }
  | { type: "agent_list"; agents: AgentInfo[] }
  | { type: "session_list"; sessions: SessionInfo[] }
  | { type: "error"; message: string };

// Agent types
export type AgentType = "claude_sdk" | "acp" | "codex" | "opencode";

export interface AgentInfo {
  id: string;
  display_name: string;
  agent_type: AgentType;
  enabled: boolean;
}

export interface SessionInfo {
  session_id: string;
  agent_name: string;
  status: string;
  model_config?: ModelConfig;
}

// Token usage information
export interface TokenUsage {
  input_tokens?: number;
  output_tokens?: number;
  total_tokens?: number;
  remaining_tokens?: number;
  context_window?: number;
  [key: string]: unknown;
}

// Chat message types
export type MessageRole = "user" | "assistant" | "system";

export interface ToolCallInfo {
  tool_name: string;
  args: unknown;
  status?: "pending" | "running" | "completed" | "error";
  output?: string;
}

export interface FileChangeInfo {
  path: string;
  action: "read" | "write" | "edit" | "delete";
  content?: string;
  diff?: string;
}

export interface ChatMessage {
  id: string;
  role: MessageRole;
  content: string;
  timestamp: number;
  toolCall?: ToolCallInfo;
  fileChange?: FileChangeInfo;
  thinking?: string;
  tokenUsage?: TokenUsage;
  isStreaming?: boolean;
}

// Model configuration
export interface ModelOption {
  id: string;
  display_name: string;
  description?: string;
}

export interface ReasoningEffortOption {
  id: "low" | "medium" | "high";
  display_name: string;
  description: string;
}

export interface ModelConfig {
  model?: string;
  reasoning_effort?: "low" | "medium" | "high";
}

// Session configuration for creation
export interface CreateSessionConfig {
  agent_id: string;
  project_path: string;
  model_config?: ModelConfig;
}

// Connection status
export type ConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "error";

// Helper functions
export function isFileChangeMessage(msg: ServerMessage): msg is Extract<ServerMessage, { type: "file_change" }> {
  return msg.type === "file_change";
}

export function isTokenUsageMessage(msg: ServerMessage): msg is Extract<ServerMessage, { type: "token_usage" }> {
  return msg.type === "token_usage";
}

export function isThinkingMessage(msg: ServerMessage): msg is Extract<ServerMessage, { type: "thinking" }> {
  return msg.type === "thinking";
}

export function isToolCallMessage(msg: ServerMessage): msg is Extract<ServerMessage, { type: "tool_call" }> {
  return msg.type === "tool_call";
}

export function isContentDeltaMessage(msg: ServerMessage): msg is Extract<ServerMessage, { type: "content_delta" }> {
  return msg.type === "content_delta";
}

// Agent capabilities
export const AGENT_CAPABILITIES: Record<AgentType, { supportsReasoningEffort: boolean; defaultModels: ModelOption[] }> = {
  claude_sdk: {
    supportsReasoningEffort: false,
    defaultModels: [
      { id: "claude-sonnet-4-20250514", display_name: "Claude Sonnet 4" },
      { id: "claude-3-5-sonnet-20241022", display_name: "Claude 3.5 Sonnet" },
      { id: "claude-3-5-haiku-20241022", display_name: "Claude 3.5 Haiku" },
    ],
  },
  codex: {
    supportsReasoningEffort: true,
    defaultModels: [
      { id: "o4-mini", display_name: "o4-mini" },
      { id: "gpt-4o", display_name: "GPT-4o" },
      { id: "gpt-4o-mini", display_name: "GPT-4o Mini" },
    ],
  },
  opencode: {
    supportsReasoningEffort: false,
    defaultModels: [
      { id: "claude-sonnet-4-20250514", display_name: "Claude Sonnet 4" },
      { id: "gpt-4o", display_name: "GPT-4o" },
      { id: "gemini-2.5-pro", display_name: "Gemini 2.5 Pro" },
    ],
  },
  acp: {
    supportsReasoningEffort: false,
    defaultModels: [],
  },
};

export const REASONING_EFFORT_OPTIONS: ReasoningEffortOption[] = [
  { id: "low", display_name: "Low", description: "Quick responses, minimal reasoning" },
  { id: "medium", display_name: "Medium", description: "Balanced reasoning and speed" },
  { id: "high", display_name: "High", description: "Deep reasoning, thorough analysis" },
];
