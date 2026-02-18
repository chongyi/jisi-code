import { Bot, Send, Sparkles } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { MessageRenderer } from "~/components/message-renderer";
import { ModelSelector } from "~/components/model-selector";
import { TokenUsageDisplay } from "~/components/token-usage-display";
import { Button } from "~/components/ui/button";
import { Badge } from "~/components/ui/badge";
import { ScrollArea } from "~/components/ui/scroll-area";
import { Textarea } from "~/components/ui/textarea";
import { useWsSend } from "~/components/websocket-provider";
import { useSessionStore } from "~/stores/session-store";
import type { ChatMessage, ModelConfig } from "~/types/websocket";

const EMPTY_MESSAGES: ChatMessage[] = [];

export function ChatPanel() {
  const send = useWsSend();
  const connectionStatus = useSessionStore((state) => state.connectionStatus);
  const agents = useSessionStore((state) => state.agents);
  const sessions = useSessionStore((state) => state.sessions);
  const activeSessionId = useSessionStore((state) => state.activeSessionId);
  const messages = useSessionStore((state) =>
    state.activeSessionId
      ? state.messages[state.activeSessionId] ?? EMPTY_MESSAGES
      : EMPTY_MESSAGES
  );
  const activeSessionMetadata = useSessionStore((state) =>
    state.activeSessionId ? state.sessionMetadata[state.activeSessionId] : undefined
  );
  const agentModelConfigs = useSessionStore((state) => state.agentModelConfigs);
  const lastError = useSessionStore((state) => state.lastError);
  const setLastError = useSessionStore((state) => state.setLastError);
  const addUserMessage = useSessionStore((state) => state.addUserMessage);
  const setSessionModelConfig = useSessionStore((state) => state.setSessionModelConfig);
  const setAgentModelConfig = useSessionStore((state) => state.setAgentModelConfig);

  const [input, setInput] = useState("");
  const viewportHostRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const viewport = viewportHostRef.current?.querySelector<HTMLElement>(
      "[data-slot='scroll-area-viewport']"
    );
    if (!viewport) {
      return;
    }

    requestAnimationFrame(() => {
      viewport.scrollTop = viewport.scrollHeight;
    });
  }, [messages.length, messages[messages.length - 1]?.content]);

  const handleSend = () => {
    const text = input.trim();
    if (!text || !activeSessionId || connectionStatus !== "connected") {
      return;
    }

    setLastError(null);
    addUserMessage(activeSessionId, text);
    const sent = send({
      type: "send_prompt",
      session_id: activeSessionId,
      prompt: text,
    });

    if (sent) {
      setInput("");
      return;
    }

    setLastError("WebSocket is disconnected. Unable to send prompt.");
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      handleSend();
    }
  };

  const activeSession = sessions.find(
    (session) => session.session_id === activeSessionId
  );
  const activeAgent = agents.find((agent) => agent.id === activeSession?.agent_name);
  const activeSessionName = activeAgent?.display_name ?? activeSession?.agent_name ?? "Session";
  const tokenUsage = activeSessionMetadata?.tokenUsage;
  const sessionModelConfig = activeSessionMetadata?.modelConfig;
  const fallbackAgentModelConfig = activeAgent ? agentModelConfigs[activeAgent.id] : undefined;

  const modelConfig = sessionModelConfig ?? fallbackAgentModelConfig ?? null;
  const handleModelConfigChange = (nextConfig: ModelConfig) => {
    const normalizedConfig = normalizeModelConfig(nextConfig);
    if (activeSessionId) {
      setSessionModelConfig(activeSessionId, normalizedConfig ?? null);
    }
    if (activeAgent) {
      setAgentModelConfig(activeAgent.id, normalizedConfig ?? null);
    }
  };

  if (!activeSessionId) {
    return (
      <section className="flex flex-1 items-center justify-center p-6">
        <div className="max-w-md rounded-xl border bg-card/90 p-6 text-center shadow-sm">
          <div className="mx-auto mb-3 flex size-10 items-center justify-center rounded-full bg-muted">
            <Sparkles className="size-5 text-muted-foreground" />
          </div>
          <p className="text-lg font-semibold">No active session</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Create a new session from the sidebar to start chatting.
          </p>
          {lastError ? (
            <p className="mt-3 rounded-md border border-destructive/40 bg-destructive/10 px-3 py-2 text-left text-xs text-destructive">
              {lastError}
            </p>
          ) : null}
        </div>
      </section>
    );
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <header className="flex flex-wrap items-center justify-between gap-3 border-b bg-background/80 px-5 py-3 backdrop-blur-sm">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <p className="truncate text-sm font-semibold">{activeSessionName}</p>
            {activeSession && (
              <Badge variant="outline" className="text-[11px]">
                {activeSession.status}
              </Badge>
            )}
          </div>
          <p className="truncate text-xs text-muted-foreground">{activeSessionId}</p>
        </div>
        <div className="flex items-center gap-3">
          <ModelSelector
            agent={activeAgent}
            config={modelConfig}
            onChange={handleModelConfigChange}
            disabled={connectionStatus !== "connected"}
          />
          <TokenUsageDisplay usage={tokenUsage} />
        </div>
      </header>

      <div
        className="min-h-0 flex-1 bg-[radial-gradient(circle_at_top,_hsl(var(--muted)/0.5),_transparent_55%)]"
        ref={viewportHostRef}
      >
        <ScrollArea className="h-full">
          <div className="mx-auto flex w-full max-w-4xl flex-col gap-2 px-4 py-5 md:px-6">
            {messages.map((message) => (
              <MessageRenderer key={message.id} message={message} />
            ))}
            {messages.length === 0 ? (
              <div className="flex flex-col items-center justify-center gap-2 rounded-xl border border-dashed bg-card/50 py-20 text-center">
                <Bot className="size-12 text-muted-foreground/40" />
                <p className="text-sm text-muted-foreground">
                  No messages yet. Start the conversation.
                </p>
              </div>
            ) : null}
          </div>
        </ScrollArea>
      </div>

      <footer className="border-t bg-background/80 p-4 backdrop-blur-sm">
        <div className="mx-auto w-full max-w-4xl rounded-xl border bg-card/90 p-3 shadow-sm">
          <div className="flex gap-2">
            <Textarea
              value={input}
              onChange={(event) => setInput(event.target.value)}
              onKeyDown={handleKeyDown}
              className="min-h-24 resize-none"
              placeholder="Type a message... (Enter to send, Shift+Enter for newline)"
              disabled={connectionStatus !== "connected"}
            />
            <Button
              type="button"
              size="icon"
              className="self-end"
              onClick={handleSend}
              disabled={!input.trim() || connectionStatus !== "connected"}
            >
              <Send className="size-4" />
            </Button>
          </div>
        </div>
      </footer>
    </section>
  );
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
