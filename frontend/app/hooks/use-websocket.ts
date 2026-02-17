import { useCallback, useEffect, useRef } from "react";

import type {
  ClientMessage,
  ConnectionStatus,
  ServerMessage,
} from "~/types/websocket";

const WS_URL = "ws://127.0.0.1:3001/ws";
const RECONNECT_DELAY = 3000;
const MAX_RECONNECT_ATTEMPTS = 5;

interface UseWebSocketOptions {
  onMessage: (message: ServerMessage) => void;
  onStatusChange: (status: ConnectionStatus) => void;
  autoConnect?: boolean;
}

export function useWebSocket({
  onMessage,
  onStatusChange,
  autoConnect = true,
}: UseWebSocketOptions) {
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectAttemptsRef = useRef(0);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const connect = useCallback(() => {
    if (
      wsRef.current &&
      (wsRef.current.readyState === WebSocket.OPEN ||
        wsRef.current.readyState === WebSocket.CONNECTING)
    ) {
      return;
    }

    onStatusChange("connecting");
    const ws = new WebSocket(WS_URL);

    ws.onopen = () => {
      reconnectAttemptsRef.current = 0;
      onStatusChange("connected");
    };

    ws.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data) as ServerMessage;
        onMessage(message);
      } catch (error) {
        console.error("Failed to parse WebSocket message:", error);
      }
    };

    ws.onclose = () => {
      onStatusChange("disconnected");
      wsRef.current = null;

      if (reconnectAttemptsRef.current < MAX_RECONNECT_ATTEMPTS) {
        reconnectAttemptsRef.current += 1;
        reconnectTimerRef.current = setTimeout(connect, RECONNECT_DELAY);
      }
    };

    ws.onerror = () => {
      onStatusChange("error");
    };

    wsRef.current = ws;
  }, [onMessage, onStatusChange]);

  const disconnect = useCallback(() => {
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }

    reconnectAttemptsRef.current = MAX_RECONNECT_ATTEMPTS;
    wsRef.current?.close();
    wsRef.current = null;
  }, []);

  const send = useCallback((message: ClientMessage) => {
    if (wsRef.current?.readyState === WebSocket.OPEN) {
      wsRef.current.send(JSON.stringify(message));
      return;
    }

    console.warn("WebSocket is not connected");
  }, []);

  useEffect(() => {
    if (autoConnect) {
      connect();
    }

    return () => {
      disconnect();
    };
  }, [autoConnect, connect, disconnect]);

  return {
    send,
    connect,
    disconnect,
  };
}
