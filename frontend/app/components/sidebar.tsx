import {
  Loader2,
  Plus,
  RefreshCw,
  Wifi,
  WifiOff,
  X,
} from "lucide-react";

import { useWsSend } from "~/components/websocket-provider";
import { Badge } from "~/components/ui/badge";
import { Button } from "~/components/ui/button";
import { ScrollArea } from "~/components/ui/scroll-area";
import { Separator } from "~/components/ui/separator";
import { cn } from "~/lib/utils";
import { useSessionStore } from "~/stores/session-store";
import type { ConnectionStatus } from "~/types/websocket";

export function Sidebar() {
  const send = useWsSend();
  const connectionStatus = useSessionStore((state) => state.connectionStatus);
  const agents = useSessionStore((state) => state.agents);
  const sessions = useSessionStore((state) => state.sessions);
  const activeSessionId = useSessionStore((state) => state.activeSessionId);
  const setActiveSession = useSessionStore((state) => state.setActiveSession);

  const handleCreateSession = (agentId: string) => {
    send({ type: "create_session", agent_id: agentId, project_path: "." });
  };

  const handleCloseSession = (sessionId: string) => {
    send({ type: "close_session", session_id: sessionId });
  };

  const handleRefresh = () => {
    send({ type: "list_agents" });
    send({ type: "list_sessions" });
  };

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

        <div className="space-y-1.5">
          {agents.map((agent) => (
            <Button
              key={agent.id}
              variant="outline"
              size="sm"
              className="w-full justify-start"
              onClick={() => handleCreateSession(agent.id)}
              disabled={connectionStatus !== "connected" || !agent.enabled}
            >
              <Plus className="size-3.5" />
              <span className="truncate">{agent.display_name}</span>
            </Button>
          ))}
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
                <p className="truncate font-medium">{session.agent_name}</p>
                <p className="truncate text-xs text-muted-foreground">
                  {session.status}
                </p>
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
                disabled={connectionStatus !== "connected"}
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
