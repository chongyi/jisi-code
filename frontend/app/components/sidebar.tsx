import {
  AlertCircle,
  FolderOpen,
  Loader2,
  Plus,
  RefreshCw,
  Wifi,
  WifiOff,
  X,
} from "lucide-react";
import { useEffect, useRef, useState } from "react";

import { DirectoryInput } from "~/components/directory-picker";
import { ModelSelector } from "~/components/model-selector";
import { useWsSend } from "~/components/websocket-provider";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { ScrollArea } from "~/components/ui/scroll-area";
import { Separator } from "~/components/ui/separator";
import { cn } from "~/lib/utils";
import { useSessionStore } from "~/stores/session-store";
import type { ConnectionStatus, ModelConfig } from "~/types/websocket";

export function Sidebar() {
  const send = useWsSend();
  const connectionStatus = useSessionStore((state) => state.connectionStatus);
  const agents = useSessionStore((state) => state.agents);
  const sessions = useSessionStore((state) => state.sessions);
  const activeSessionId = useSessionStore((state) => state.activeSessionId);
  const creatingSessionAgentId = useSessionStore(
    (state) => state.creatingSessionAgentId
  );
  const lastError = useSessionStore((state) => state.lastError);
  const projectPath = useSessionStore((state) => state.projectPath);
  const setActiveSession = useSessionStore((state) => state.setActiveSession);
  const startCreatingSession = useSessionStore(
    (state) => state.startCreatingSession
  );
  const finishCreatingSession = useSessionStore(
    (state) => state.finishCreatingSession
  );
  const setLastError = useSessionStore((state) => state.setLastError);
  const setProjectPath = useSessionStore((state) => state.setProjectPath);
  const agentModelConfigs = useSessionStore((state) => state.agentModelConfigs);
  const setAgentModelConfig = useSessionStore((state) => state.setAgentModelConfig);
  const createTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  const [localProjectPath, setLocalProjectPath] = useState(projectPath);

  useEffect(() => {
    if (createTimerRef.current) {
      clearTimeout(createTimerRef.current);
      createTimerRef.current = null;
    }

    if (!creatingSessionAgentId) {
      return;
    }

    createTimerRef.current = setTimeout(() => {
      const store = useSessionStore.getState();
      if (store.creatingSessionAgentId) {
        store.finishCreatingSession();
        store.setLastError("Create session timed out. Please retry.");
      }
    }, 32000);

    return () => {
      if (createTimerRef.current) {
        clearTimeout(createTimerRef.current);
        createTimerRef.current = null;
      }
    };
  }, [creatingSessionAgentId]);

  const handleCreateSession = (agentId: string) => {
    setLastError(null);
    setProjectPath(localProjectPath);
    startCreatingSession(agentId);
    const modelConfig = normalizeModelConfig(agentModelConfigs[agentId]);
    const sent = send({
      type: "create_session",
      agent_id: agentId,
      project_path: localProjectPath || ".",
      model_config: modelConfig,
    });

    if (!sent) {
      finishCreatingSession();
      setLastError("WebSocket is disconnected. Reconnect and try again.");
    }
  };

  const handleCloseSession = (sessionId: string) => {
    const sent = send({ type: "close_session", session_id: sessionId });
    if (!sent) {
      setLastError("WebSocket is disconnected. Unable to close session.");
    }
  };

  const handleRefresh = () => {
    setLastError(null);
    const sentAgents = send({ type: "list_agents" });
    const sentSessions = send({ type: "list_sessions" });
    if (!sentAgents || !sentSessions) {
      setLastError("WebSocket is disconnected. Unable to refresh.");
    }
  };
  const agentDisplayMap = new Map(
    agents.map((agent) => [agent.id, agent.display_name] as const)
  );

  return (
    <aside className="flex w-72 shrink-0 flex-col border-r bg-card">
      <div className="flex items-center justify-between gap-2 p-4">
        <div className="min-w-0">
          <p className="truncate text-sm font-semibold">Jisi Code</p>
          <p className="text-xs text-muted-foreground">Session Console</p>
        </div>
        <ConnectionIndicator status={connectionStatus} />
      </div>
      <Separator />

      <div className="space-y-3 p-3">
        {lastError ? (
          <div className="rounded-md border border-destructive/40 bg-destructive/10 px-2.5 py-2 text-xs text-destructive">
            <div className="flex items-start gap-2">
              <AlertCircle className="mt-0.5 size-3.5 shrink-0" />
              <p className="min-w-0 flex-1 break-words">{lastError}</p>
            </div>
          </div>
        ) : null}

        {/* Project Path Input */}
        <div className="space-y-1.5">
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <FolderOpen className="size-3.5" />
            <span>Working Directory</span>
          </div>
          <DirectoryInput
            value={localProjectPath}
            onChange={setLocalProjectPath}
            disabled={connectionStatus !== "connected" || creatingSessionAgentId !== null}
          />
        </div>

        <div className="flex items-center justify-between">
          <p className="text-xs text-muted-foreground">Available Agents</p>
          <Button
            aria-label="refresh list"
            variant="ghost"
            size="icon-xs"
            onClick={handleRefresh}
            disabled={connectionStatus !== "connected"}
          >
            <RefreshCw className="size-3.5" />
          </Button>
        </div>
        <p className="text-[11px] text-muted-foreground">
          Model settings are applied when creating a new session.
        </p>

        <div className="space-y-1.5">
          {agents.map((agent) => {
            const modelConfig = agentModelConfigs[agent.id] ?? null;

            return (
              <div
                key={agent.id}
                className="rounded-md border bg-background/70 p-2 shadow-sm"
              >
                <Button
                  variant="outline"
                  size="sm"
                  className="w-full justify-start"
                  onClick={() => handleCreateSession(agent.id)}
                  disabled={
                    connectionStatus !== "connected" ||
                    !agent.enabled ||
                    creatingSessionAgentId !== null
                  }
                >
                  {creatingSessionAgentId === agent.id ? (
                    <Loader2 className="size-3.5 animate-spin" />
                  ) : (
                    <Plus className="size-3.5" />
                  )}
                  <span className="truncate">
                    {creatingSessionAgentId === agent.id
                      ? "Creating session..."
                      : agent.display_name}
                  </span>
                </Button>
                <div className="mt-2 flex justify-end">
                  <ModelSelector
                    agent={agent}
                    config={modelConfig}
                    onChange={(nextConfig) =>
                      setAgentModelConfig(agent.id, normalizeModelConfig(nextConfig) ?? null)
                    }
                    disabled={
                      connectionStatus !== "connected" ||
                      creatingSessionAgentId !== null
                    }
                  />
                </div>
              </div>
            );
          })}
          {agents.length === 0 ? (
            <div className="rounded-md border border-dashed px-3 py-2 text-xs text-muted-foreground">
              No agents loaded
            </div>
          ) : null}
        </div>
      </div>

      <Separator />

      <ScrollArea className="min-h-0 flex-1">
        <div className="space-y-1.5 p-3">
          <p className="px-1 text-xs text-muted-foreground">Sessions</p>
          {sessions.map((session) => (
            <div
              key={session.session_id}
              role="button"
              tabIndex={0}
              onClick={() => setActiveSession(session.session_id)}
              onKeyDown={(event) => {
                if (event.key === "Enter" || event.key === " ") {
                  event.preventDefault();
                  setActiveSession(session.session_id);
                }
              }}
              className={cn(
                "flex cursor-pointer items-center gap-2 rounded-md px-2 py-2 text-sm transition-colors",
                session.session_id === activeSessionId
                  ? "bg-accent text-accent-foreground"
                  : "hover:bg-accent/50"
              )}
            >
              <div className="min-w-0 flex-1">
                <p className="truncate font-medium">
                  {agentDisplayMap.get(session.agent_name) ?? session.agent_name}
                </p>
                <p className="truncate text-xs text-muted-foreground">
                  {session.status}
                </p>
                {session.model_config?.model ? (
                  <p className="truncate text-[11px] text-muted-foreground/80">
                    {session.model_config.model}
                  </p>
                ) : null}
              </div>
              <Button
                type="button"
                variant="ghost"
                size="icon-xs"
                className="shrink-0"
                onClick={(event) => {
                  event.stopPropagation();
                  handleCloseSession(session.session_id);
                }}
                disabled={
                  connectionStatus !== "connected" ||
                  creatingSessionAgentId !== null
                }
              >
                <X className="size-3.5" />
              </Button>
            </div>
          ))}
          {sessions.length === 0 ? (
            <div className="rounded-md border border-dashed px-3 py-5 text-center text-xs text-muted-foreground">
              No active sessions
            </div>
          ) : null}
        </div>
      </ScrollArea>
    </aside>
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

function ConnectionIndicator({ status }: { status: ConnectionStatus }) {
  if (status === "connected") {
    return (
      <Badge variant="default" className="text-xs">
        <Wifi className="size-3" />
        Connected
      </Badge>
    );
  }

  if (status === "connecting") {
    return (
      <Badge variant="secondary" className="text-xs">
        <Loader2 className="size-3 animate-spin" />
        Connecting
      </Badge>
    );
  }

  if (status === "error") {
    return (
      <Badge variant="destructive" className="text-xs">
        <WifiOff className="size-3" />
        Error
      </Badge>
    );
  }

  return (
    <Badge variant="outline" className="text-xs">
      <WifiOff className="size-3" />
      Disconnected
    </Badge>
  );
}
