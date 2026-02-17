import { Outlet } from "react-router";

import { Sidebar } from "~/components/sidebar";
import { WebSocketProvider } from "~/components/websocket-provider";

export default function AppLayout() {
  return (
    <WebSocketProvider>
      <div className="flex h-screen bg-background text-foreground">
        <Sidebar />
        <main className="flex min-w-0 flex-1 flex-col overflow-hidden">
          <Outlet />
        </main>
      </div>
    </WebSocketProvider>
  );
}
