import {
  Bot,
  Brain,
  ChevronDown,
  ChevronRight,
  FilePenLine,
  Loader2,
  Sparkles,
  Trash2,
  User,
  Wrench,
} from "lucide-react";
import { useState } from "react";

import { Badge } from "~/components/ui/badge";
import { Card, CardContent } from "~/components/ui/card";
import { ScrollArea } from "~/components/ui/scroll-area";
import { cn } from "~/lib/utils";
import type { ChatMessage } from "~/types/websocket";

interface MessageRendererProps {
  message: ChatMessage;
}

export function MessageRenderer({ message }: MessageRendererProps) {
  const { role, content, toolCall, fileChange, thinking, isStreaming } = message;

  if (thinking) {
    return <ThinkingBlock thinking={thinking} />;
  }

  if (toolCall) {
    return <ToolCallBlock toolCall={toolCall} />;
  }

  if (fileChange) {
    return <FileChangeBlock fileChange={fileChange} />;
  }

  const isUser = role === "user";
  const isSystem = role === "system";

  return (
    <div className={cn("flex w-full", isUser ? "justify-end" : "justify-start")}>
      <div
        className={cn(
          "w-full max-w-[85%] rounded-xl border px-4 py-3 shadow-sm",
          isUser && "border-primary/25 bg-primary/10",
          !isUser && !isSystem && "bg-card",
          isSystem && "border-amber-500/30 bg-amber-500/10"
        )}
      >
        <div className="flex items-center gap-2 text-xs">
          <span
            className={cn(
              "inline-flex size-6 items-center justify-center rounded-full border",
              isUser && "border-primary/30 text-primary",
              !isUser && !isSystem && "border-muted-foreground/30 text-muted-foreground",
              isSystem && "border-amber-600/30 text-amber-700"
            )}
          >
            {isUser ? (
              <User className="size-3.5" />
            ) : isSystem ? (
              <Sparkles className="size-3.5" />
            ) : (
              <Bot className="size-3.5" />
            )}
          </span>
          <span className="font-medium">
            {isUser ? "You" : isSystem ? "System" : "Assistant"}
          </span>
          {isStreaming && (
            <Loader2 className="size-3.5 animate-spin text-muted-foreground" />
          )}
        </div>
        <div className="mt-2 whitespace-pre-wrap break-words text-sm leading-relaxed">
          {content}
          {isStreaming && <span className="animate-pulse">▊</span>}
        </div>
      </div>
    </div>
  );
}

interface ThinkingBlockProps {
  thinking: string;
}

function ThinkingBlock({ thinking }: ThinkingBlockProps) {
  const [isExpanded, setIsExpanded] = useState(false);

  return (
    <CollapsibleCard
      icon={<Brain className="size-4 text-indigo-500" />}
      title="Thinking"
      expanded={isExpanded}
      onToggle={() => setIsExpanded((value) => !value)}
      variant="muted"
    >
      <ScrollArea className="max-h-64">
        <p className="whitespace-pre-wrap text-xs leading-relaxed text-muted-foreground">
          {thinking}
        </p>
      </ScrollArea>
    </CollapsibleCard>
  );
}

interface ToolCallBlockProps {
  toolCall: NonNullable<ChatMessage["toolCall"]>;
}

function ToolCallBlock({ toolCall }: ToolCallBlockProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const { tool_name, args, status } = toolCall;

  return (
    <CollapsibleCard
      icon={<Wrench className="size-4 text-blue-500" />}
      title={tool_name}
      expanded={isExpanded}
      onToggle={() => setIsExpanded((value) => !value)}
      trailing={
        <Badge
          variant="outline"
          className={cn(
            "text-[11px]",
            status === "running" && "border-blue-400/40 text-blue-600",
            status === "completed" && "border-emerald-500/40 text-emerald-600",
            status === "error" && "border-red-500/40 text-red-600"
          )}
        >
          {status ?? "pending"}
        </Badge>
      }
    >
      <pre className="overflow-x-auto rounded-md border bg-muted/40 p-2 text-xs">
        {JSON.stringify(args ?? {}, null, 2)}
      </pre>
    </CollapsibleCard>
  );
}

interface FileChangeBlockProps {
  fileChange: NonNullable<ChatMessage["fileChange"]>;
}

function FileChangeBlock({ fileChange }: FileChangeBlockProps) {
  const [isExpanded, setIsExpanded] = useState(false);
  const { path, action, content, diff } = fileChange;

  const ActionIcon =
    action === "delete"
      ? Trash2
      : action === "edit"
        ? FilePenLine
        : FilePenLine;

  const actionColor =
    action === "delete"
      ? "text-red-500"
      : action === "edit"
        ? "text-amber-500"
        : action === "write"
          ? "text-emerald-500"
          : "text-blue-500";

  return (
    <CollapsibleCard
      icon={<ActionIcon className={cn("size-4", actionColor)} />}
      title={action.toUpperCase()}
      expanded={isExpanded}
      onToggle={() => setIsExpanded((value) => !value)}
      trailing={
        <code className="max-w-60 truncate rounded bg-muted px-1.5 py-0.5 text-[11px]">
          {path}
        </code>
      }
    >
      <div className="space-y-2">
        {content ? (
          <div>
            <p className="mb-1 text-xs font-medium text-muted-foreground">Content</p>
            <pre className="overflow-x-auto rounded-md border bg-muted/40 p-2 text-xs">
              {content}
            </pre>
          </div>
        ) : null}
        {diff ? (
          <div>
            <p className="mb-1 text-xs font-medium text-muted-foreground">Diff</p>
            <pre className="overflow-x-auto rounded-md border bg-muted/40 p-2 text-xs font-mono">
              <DiffDisplay diff={diff} />
            </pre>
          </div>
        ) : null}
      </div>
    </CollapsibleCard>
  );
}

interface CollapsibleCardProps {
  icon: React.ReactNode;
  title: string;
  expanded: boolean;
  onToggle: () => void;
  trailing?: React.ReactNode;
  variant?: "default" | "muted";
  children: React.ReactNode;
}

function CollapsibleCard({
  icon,
  title,
  expanded,
  onToggle,
  trailing,
  variant = "default",
  children,
}: CollapsibleCardProps) {
  return (
    <Card
      className={cn(
        "overflow-hidden border shadow-sm",
        variant === "muted" && "bg-muted/20"
      )}
    >
      <button
        type="button"
        onClick={onToggle}
        className="flex w-full items-center gap-2 px-3 py-2 text-left hover:bg-muted/30"
      >
        {expanded ? (
          <ChevronDown className="size-4 text-muted-foreground" />
        ) : (
          <ChevronRight className="size-4 text-muted-foreground" />
        )}
        {icon}
        <span className="truncate text-sm font-medium">{title}</span>
        <div className="ml-auto">{trailing}</div>
      </button>
      {expanded ? <CardContent className="pt-0">{children}</CardContent> : null}
    </Card>
  );
}

interface DiffDisplayProps {
  diff: string;
}

function DiffDisplay({ diff }: DiffDisplayProps) {
  const lines = diff.split("\n");

  return (
    <div className="space-y-0">
      {lines.map((line, index) => {
        let lineClass = "";
        if (line.startsWith("+") && !line.startsWith("+++")) {
          lineClass = "bg-emerald-500/10 text-emerald-700 dark:text-emerald-300";
        } else if (line.startsWith("-") && !line.startsWith("---")) {
          lineClass = "bg-red-500/10 text-red-700 dark:text-red-300";
        } else if (line.startsWith("@@")) {
          lineClass = "bg-blue-500/10 text-blue-700 dark:text-blue-300";
        }

        return (
          <div key={index} className={cn("px-1", lineClass)}>
            {line || " "}
          </div>
        );
      })}
    </div>
  );
}
