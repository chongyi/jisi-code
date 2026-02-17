import { create } from "zustand";
import { immer } from "zustand/middleware/immer";

import type {
  AgentInfo,
  ChatMessage,
  ConnectionStatus,
  ServerMessage,
  SessionInfo,
} from "~/types/websocket";

interface SessionState {
  connectionStatus: ConnectionStatus;
  agents: AgentInfo[];
  sessions: SessionInfo[];
  activeSessionId: string | null;
  messages: Record<string, ChatMessage[]>;
  setConnectionStatus: (status: ConnectionStatus) => void;
  setAgents: (agents: AgentInfo[]) => void;
  setSessions: (sessions: SessionInfo[]) => void;
  setActiveSession: (sessionId: string | null) => void;
  addUserMessage: (sessionId: string, content: string) => void;
  handleServerMessage: (message: ServerMessage) => void;
  removeSession: (sessionId: string) => void;
}

let messageIdCounter = 0;

function nextMessageId(): string {
  messageIdCounter += 1;
  return `msg-${Date.now()}-${messageIdCounter}`;
}

export const useSessionStore = create<SessionState>()(
  immer((set) => ({
    connectionStatus: "disconnected",
    agents: [],
    sessions: [],
    activeSessionId: null,
    messages: {},

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
        }
      }),

    setActiveSession: (sessionId) =>
      set((state) => {
        state.activeSessionId = sessionId;
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

    handleServerMessage: (message) =>
      set((state) => {
        switch (message.type) {
          case "session_created": {
            state.messages[message.session_id] ??= [];
            state.sessions.push({
              session_id: message.session_id,
              agent_name: message.agent_name,
              status: "Ready",
            });
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
              !lastMessage.toolCall
            ) {
              lastMessage.content += message.content;
            } else {
              messageList.push({
                id: nextMessageId(),
                role: "assistant",
                content: message.content,
                timestamp: Date.now(),
              });
            }
            break;
          }
          case "tool_call": {
            const messageList = state.messages[message.session_id];
            if (!messageList) {
              break;
            }

            messageList.push({
              id: nextMessageId(),
              role: "assistant",
              content: `Tool: ${message.tool_name}`,
              timestamp: Date.now(),
              toolCall: {
                tool_name: message.tool_name,
                args: message.args,
              },
            });
            break;
          }
          case "session_closed": {
            const index = state.sessions.findIndex(
              (session) => session.session_id === message.session_id
            );
            if (index !== -1) {
              state.sessions.splice(index, 1);
            }

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
            }
            break;
          }
          case "error": {
            if (!state.activeSessionId) {
              break;
            }

            state.messages[state.activeSessionId] ??= [];
            state.messages[state.activeSessionId].push({
              id: nextMessageId(),
              role: "system",
              content: message.message,
              timestamp: Date.now(),
            });
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

        if (state.activeSessionId === sessionId) {
          state.activeSessionId = state.sessions[0]?.session_id ?? null;
        }
      }),
  }))
);
