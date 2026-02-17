export type ClientMessage =
  | { type: "create_session"; agent_id: string; project_path: string }
  | { type: "send_prompt"; session_id: string; prompt: string }
  | { type: "close_session"; session_id: string }
  | { type: "list_agents" }
  | { type: "list_sessions" };

export type ServerMessage =
  | { type: "session_created"; session_id: string; agent_name: string }
  | { type: "prompt_accepted"; session_id: string }
  | { type: "content_delta"; session_id: string; content: string }
  | { type: "tool_call"; session_id: string; tool_name: string; args: unknown }
  | { type: "session_closed"; session_id: string }
  | { type: "agent_list"; agents: AgentInfo[] }
  | { type: "session_list"; sessions: SessionInfo[] }
  | { type: "error"; message: string };

export interface AgentInfo {
  id: string;
  display_name: string;
  agent_type: string;
  enabled: boolean;
}

export interface SessionInfo {
  session_id: string;
  agent_name: string;
  status: string;
}

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  timestamp: number;
  toolCall?: {
    tool_name: string;
    args: unknown;
  };
}

export type ConnectionStatus =
  | "disconnected"
  | "connecting"
  | "connected"
  | "error";
