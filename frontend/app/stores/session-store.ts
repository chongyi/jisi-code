import { create } from "zustand";
import { immer } from "zustand/middleware/immer";

import type {
  AgentInfo,
  ChatMessage,
  ConnectionStatus,
  ModelConfig,
  ServerMessage,
  SessionInfo,
  TokenUsage,
} from "~/types/websocket";

interface SessionMetadata {
  tokenUsage?: TokenUsage;
  modelConfig?: ModelConfig;
}

interface SessionState {
  connectionStatus: ConnectionStatus;
  agents: AgentInfo[];
  sessions: SessionInfo[];
  activeSessionId: string | null;
  creatingSessionAgentId: string | null;
  lastError: string | null;
  messages: Record<string, ChatMessage[]>;
  sessionMetadata: Record<string, SessionMetadata>;
  agentModelConfigs: Record<string, ModelConfig>;
  projectPath: string;

  setConnectionStatus: (status: ConnectionStatus) => void;
  setAgents: (agents: AgentInfo[]) => void;
  setSessions: (sessions: SessionInfo[]) => void;
  setActiveSession: (sessionId: string | null) => void;
  startCreatingSession: (agentId: string) => void;
  finishCreatingSession: () => void;
  setLastError: (message: string | null) => void;
  addUserMessage: (sessionId: string, content: string) => void;
  handleServerMessage: (message: ServerMessage) => void;
  removeSession: (sessionId: string) => void;
  setAgentModelConfig: (agentId: string, config: ModelConfig | null) => void;
  setSessionModelConfig: (sessionId: string, config: ModelConfig | null) => void;
  setProjectPath: (path: string) => void;
  getSessionTokenUsage: (sessionId: string) => TokenUsage | undefined;
  getSessionModelConfig: (sessionId: string) => ModelConfig | undefined;
  getAgentModelConfig: (agentId: string) => ModelConfig | undefined;
}

let messageIdCounter = 0;

function nextMessageId(): string {
  messageIdCounter += 1;
  return `msg-${Date.now()}-${messageIdCounter}`;
}

function normalizeModelConfig(
  config: ModelConfig | null | undefined
): ModelConfig | undefined {
  if (!config) {
    return undefined;
  }

  const normalized: ModelConfig = {};
  if (config.model) {
    normalized.model = config.model;
  }
  if (config.reasoning_effort) {
    normalized.reasoning_effort = config.reasoning_effort;
  }

  return Object.keys(normalized).length > 0 ? normalized : undefined;
}

export const useSessionStore = create<SessionState>()(
  immer((set, get) => ({
    connectionStatus: "disconnected",
    agents: [],
    sessions: [],
    activeSessionId: null,
    creatingSessionAgentId: null,
    lastError: null,
    messages: {},
    sessionMetadata: {},
    agentModelConfigs: {},
    projectPath: ".",

    setConnectionStatus: (status) =>
      set((state) => {
        state.connectionStatus = status;
      }),

    setAgents: (agents) =>
      set((state) => {
        state.agents = agents;
      }),

    setSessions: (sessions) =>
      set((state) => {
        state.sessions = sessions;

        for (const session of sessions) {
          if (!state.messages[session.session_id]) {
            state.messages[session.session_id] = [];
          }
          if (!state.sessionMetadata[session.session_id]) {
            state.sessionMetadata[session.session_id] = {};
          }
          const modelConfig = normalizeModelConfig(session.model_config);
          if (modelConfig) {
            state.sessionMetadata[session.session_id].modelConfig = modelConfig;
            state.agentModelConfigs[session.agent_name] = modelConfig;
          } else {
            delete state.sessionMetadata[session.session_id].modelConfig;
          }
        }
      }),

    setActiveSession: (sessionId) =>
      set((state) => {
        state.activeSessionId = sessionId;
      }),

    startCreatingSession: (agentId) =>
      set((state) => {
        state.creatingSessionAgentId = agentId;
      }),

    finishCreatingSession: () =>
      set((state) => {
        state.creatingSessionAgentId = null;
      }),

    setLastError: (message) =>
      set((state) => {
        state.lastError = message;
      }),

    addUserMessage: (sessionId, content) =>
      set((state) => {
        if (!state.messages[sessionId]) {
          state.messages[sessionId] = [];
        }

        state.messages[sessionId].push({
          id: nextMessageId(),
          role: "user",
          content,
          timestamp: Date.now(),
        });
      }),

    setAgentModelConfig: (agentId, config) =>
      set((state) => {
        const normalized = normalizeModelConfig(config);
        if (normalized) {
          state.agentModelConfigs[agentId] = normalized;
        } else {
          delete state.agentModelConfigs[agentId];
        }
      }),

    setSessionModelConfig: (sessionId, config) =>
      set((state) => {
        const normalized = normalizeModelConfig(config);
        state.sessionMetadata[sessionId] ??= {};
        if (normalized) {
          state.sessionMetadata[sessionId].modelConfig = normalized;
        } else {
          delete state.sessionMetadata[sessionId].modelConfig;
        }

        const session = state.sessions.find((item) => item.session_id === sessionId);
        if (session) {
          session.model_config = normalized;
        }
      }),

    setProjectPath: (path) =>
      set((state) => {
        state.projectPath = path;
      }),

    getSessionTokenUsage: (sessionId) => {
      return get().sessionMetadata[sessionId]?.tokenUsage;
    },

    getSessionModelConfig: (sessionId) => {
      return get().sessionMetadata[sessionId]?.modelConfig;
    },

    getAgentModelConfig: (agentId) => {
      return get().agentModelConfigs[agentId];
    },

    handleServerMessage: (message) =>
      set((state) => {
        switch (message.type) {
          case "session_created": {
            state.creatingSessionAgentId = null;
            state.lastError = null;
            state.messages[message.session_id] ??= [];
            const createdModelConfig = normalizeModelConfig(message.model_config);
            state.sessionMetadata[message.session_id] = {
              modelConfig: createdModelConfig,
            };
            state.sessions.push({
              session_id: message.session_id,
              agent_name: message.agent_name,
              status: "Ready",
              model_config: createdModelConfig,
            });
            if (createdModelConfig) {
              state.agentModelConfigs[message.agent_name] = createdModelConfig;
            }
            state.activeSessionId = message.session_id;
            break;
          }
          case "prompt_accepted": {
            const session = state.sessions.find(
              (item) => item.session_id === message.session_id
            );
            if (session) {
              session.status = "Processing";
            }
            break;
          }
          case "content_delta": {
            const messageList = state.messages[message.session_id];
            if (!messageList) {
              break;
            }

            const lastMessage = messageList[messageList.length - 1];
            if (
              lastMessage &&
              lastMessage.role === "assistant" &&
              !lastMessage.toolCall &&
              !lastMessage.fileChange
            ) {
              lastMessage.content += message.content;
              lastMessage.isStreaming = true;
            } else {
              messageList.push({
                id: nextMessageId(),
                role: "assistant",
                content: message.content,
                timestamp: Date.now(),
                isStreaming: true,
              });
            }
            break;
          }
          case "tool_call": {
            const messageList = state.messages[message.session_id];
            if (!messageList) {
              break;
            }

            // Mark the previous streaming message as complete
            const lastMessage = messageList[messageList.length - 1];
            if (lastMessage?.isStreaming) {
              lastMessage.isStreaming = false;
            }

            messageList.push({
              id: nextMessageId(),
              role: "assistant",
              content: "",
              timestamp: Date.now(),
              toolCall: {
                tool_name: message.tool_name,
                args: message.args,
                status: "running",
              },
            });
            break;
          }
          case "file_change": {
            const messageList = state.messages[message.session_id];
            if (!messageList) {
              break;
            }

            messageList.push({
              id: nextMessageId(),
              role: "assistant",
              content: "",
              timestamp: Date.now(),
              fileChange: {
                path: message.path,
                action: message.action as "read" | "write" | "edit" | "delete",
                content: message.content,
                diff: message.diff,
              },
            });
            break;
          }
          case "token_usage": {
            state.sessionMetadata[message.session_id] ??= {};
            state.sessionMetadata[message.session_id].tokenUsage = message.usage;
            break;
          }
          case "thinking": {
            const messageList = state.messages[message.session_id];
            if (!messageList) {
              break;
            }

            const lastMessage = messageList[messageList.length - 1];
            if (lastMessage?.thinking) {
              lastMessage.thinking += message.content;
            } else {
              messageList.push({
                id: nextMessageId(),
                role: "assistant",
                content: "",
                timestamp: Date.now(),
                thinking: message.content,
              });
            }
            break;
          }
          case "session_closed": {
            const index = state.sessions.findIndex(
              (session) => session.session_id === message.session_id
            );
            if (index !== -1) {
              state.sessions.splice(index, 1);
            }

            delete state.messages[message.session_id];
            delete state.sessionMetadata[message.session_id];

            if (state.activeSessionId === message.session_id) {
              state.activeSessionId = state.sessions[0]?.session_id ?? null;
            }
            break;
          }
          case "agent_list": {
            state.agents = message.agents;
            break;
          }
          case "session_list": {
            state.sessions = message.sessions;
            for (const session of message.sessions) {
              state.messages[session.session_id] ??= [];
              state.sessionMetadata[session.session_id] ??= {};
              const modelConfig = normalizeModelConfig(session.model_config);
              if (modelConfig) {
                state.sessionMetadata[session.session_id].modelConfig = modelConfig;
                state.agentModelConfigs[session.agent_name] = modelConfig;
              } else {
                delete state.sessionMetadata[session.session_id].modelConfig;
              }
            }
            break;
          }
          case "error": {
            state.creatingSessionAgentId = null;
            state.lastError = message.message;

            // Mark streaming as complete on error
            if (state.activeSessionId) {
              const messageList = state.messages[state.activeSessionId];
              if (messageList && messageList.length > 0) {
                const lastMessage = messageList[messageList.length - 1];
                if (lastMessage?.isStreaming) {
                  lastMessage.isStreaming = false;
                }
              }

              state.messages[state.activeSessionId] ??= [];
              state.messages[state.activeSessionId].push({
                id: nextMessageId(),
                role: "system",
                content: message.message,
                timestamp: Date.now(),
              });
            }
            break;
          }
        }
      }),

    removeSession: (sessionId) =>
      set((state) => {
        const index = state.sessions.findIndex(
          (session) => session.session_id === sessionId
        );
        if (index !== -1) {
          state.sessions.splice(index, 1);
        }

        delete state.messages[sessionId];
        delete state.sessionMetadata[sessionId];

        if (state.activeSessionId === sessionId) {
          state.activeSessionId = state.sessions[0]?.session_id ?? null;
        }
      }),
  }))
);
