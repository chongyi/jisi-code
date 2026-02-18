import { createContext, useCallback, useContext, useEffect, useRef } from "react";

import { useWebSocket } from "~/hooks/use-websocket";
import { useSessionStore } from "~/stores/session-store";
import type {
  ClientMessage,
  ConnectionStatus,
  ServerMessage,
} from "~/types/websocket";

interface WebSocketContextValue {
  send: (message: ClientMessage) => boolean;
}

const WebSocketContext = createContext<WebSocketContextValue | null>(null);

export function WebSocketProvider({ children }: { children: React.ReactNode }) {
  const handleServerMessage = useSessionStore((state) => state.handleServerMessage);
  const setConnectionStatus = useSessionStore((state) => state.setConnectionStatus);
  const sendRef = useRef<(message: ClientMessage) => boolean>(() => false);

  const onMessage = useCallback(
    (message: ServerMessage) => {
      handleServerMessage(message);
    },
    [handleServerMessage]
  );

  const onStatusChange = useCallback(
    (status: ConnectionStatus) => {
      setConnectionStatus(status);

      if (status === "connected") {
        sendRef.current({ type: "list_agents" });
        sendRef.current({ type: "list_sessions" });
      }
    },
    [setConnectionStatus]
  );

  const { send } = useWebSocket({ onMessage, onStatusChange });

  const sendWithLog = useCallback(
    (message: ClientMessage) => {
      return send(message);
    },
    [send]
  );

  useEffect(() => {
    sendRef.current = sendWithLog;
  }, [sendWithLog]);

  return (
    <WebSocketContext.Provider value={{ send: sendWithLog }}>
      {children}
    </WebSocketContext.Provider>
  );
}

export function useWsSend() {
  const context = useContext(WebSocketContext);
  if (!context) {
    throw new Error("useWsSend must be used within WebSocketProvider");
  }

  return context.send;
}
