import dayjs from "dayjs";
import { AlertCircle, Bot, Send, User, Wrench } from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { useWsSend } from "~/components/websocket-provider";
import { Button } from "~/components/ui/button";
import { ScrollArea } from "~/components/ui/scroll-area";
import { Textarea } from "~/components/ui/textarea";
import { cn } from "~/lib/utils";
import { useSessionStore } from "~/stores/session-store";
import type { ChatMessage } from "~/types/websocket";

export function ChatPanel() {
  const send = useWsSend();
  const connectionStatus = useSessionStore((state) => state.connectionStatus);
  const activeSessionId = useSessionStore((state) => state.activeSessionId);
  const messages = useSessionStore((state) =>
    state.activeSessionId ? state.messages[state.activeSessionId] ?? [] : []
  );
  const addUserMessage = useSessionStore((state) => state.addUserMessage);
  const sessions = useSessionStore((state) => state.sessions);

  const [input, setInput] = useState("");
  const [sending, setSending] = useState(false);
  const viewportHostRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    const viewport = viewportHostRef.current?.querySelector<HTMLElement>(
      "[data-slot='scroll-area-viewport']"
    );
    if (viewport) {
      viewport.scrollTop = viewport.scrollHeight;
    }
  }, [messages.length, activeSessionId]);

  const handleSend = () => {
    const text = input.trim();
    if (!text || !activeSessionId || connectionStatus !== "connected") {
      return;
    }

    setSending(true);
    addUserMessage(activeSessionId, text);
    send({ type: "send_prompt", session_id: activeSessionId, prompt: text });
    setInput("");
    setSending(false);
  };

  const handleKeyDown = (event: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      handleSend();
    }
  };

  const activeSessionName =
    sessions.find((session) => session.session_id === activeSessionId)?.agent_name ??
    "Session";

  if (!activeSessionId) {
    return (
      <section className="flex flex-1 items-center justify-center p-6">
        <div className="max-w-md rounded-lg border bg-card p-6 text-center">
          <p className="text-lg font-semibold">No active session</p>
          <p className="mt-1 text-sm text-muted-foreground">
            Create a new session from the sidebar to start chatting.
          </p>
        </div>
      </section>
    );
  }

  return (
    <section className="flex min-h-0 flex-1 flex-col">
      <header className="flex items-center justify-between border-b px-5 py-3">
        <div>
          <p className="text-sm font-semibold">{activeSessionName}</p>
          <p className="text-xs text-muted-foreground">{activeSessionId}</p>
        </div>
      </header>

      <div className="min-h-0 flex-1 p-4" ref={viewportHostRef}>
        <ScrollArea className="h-full rounded-lg border bg-background">
          <div className="mx-auto flex w-full max-w-3xl flex-col gap-4 p-4">
            {messages.map((message) => (
              <MessageBubble key={message.id} message={message} />
            ))}
            {messages.length === 0 ? (
              <p className="text-center text-sm text-muted-foreground">
                No messages yet
              </p>
            ) : null}
          </div>
        </ScrollArea>
      </div>

      <footer className="border-t p-4">
        <div className="mx-auto flex w-full max-w-3xl gap-2">
          <Textarea
            value={input}
            onChange={(event) => setInput(event.target.value)}
            onKeyDown={handleKeyDown}
            className="min-h-20 resize-none"
            placeholder="Type a message... (Enter to send, Shift+Enter for newline)"
            disabled={connectionStatus !== "connected"}
          />
          <Button
            type="button"
            size="icon"
            className="self-end"
            onClick={handleSend}
            disabled={
              !input.trim() || connectionStatus !== "connected" || sending
            }
          >
            <Send className="size-4" />
          </Button>
        </div>
      </footer>
    </section>
  );
}

function MessageBubble({ message }: { message: ChatMessage }) {
  const isUser = message.role === "user";
  const isSystem = message.role === "system";
  const isToolCall = Boolean(message.toolCall);

  return (
    <article className={cn("flex", isUser ? "justify-end" : "justify-start")}>
      <div
        className={cn(
          "max-w-[88%] rounded-lg border px-3 py-2",
          isUser && "border-primary/30 bg-primary text-primary-foreground",
          !isUser && !isSystem && "bg-muted",
          isSystem && "border-destructive/30 bg-destructive/10 text-destructive",
          isToolCall && "border-border bg-accent/70"
        )}
      >
        <div className="mb-1 flex items-center gap-1.5 text-xs opacity-75">
          {isSystem ? (
            <AlertCircle className="size-3.5" />
          ) : isUser ? (
            <User className="size-3.5" />
          ) : (
            <Bot className="size-3.5" />
          )}
          <span>{isSystem ? "System" : isUser ? "You" : "Assistant"}</span>
        </div>

        {isToolCall ? (
          <div className="mb-1 flex items-center gap-1.5 font-mono text-xs">
            <Wrench className="size-3.5" />
            <span>{message.toolCall?.tool_name}</span>
          </div>
        ) : null}

        <p className="whitespace-pre-wrap break-words text-sm">{message.content}</p>
        <p className="mt-1 text-right text-xs opacity-60">
          {dayjs(message.timestamp).format("HH:mm:ss")}
        </p>
      </div>
    </article>
  );
}
