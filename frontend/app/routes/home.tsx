import type { Route } from "./+types/home";
import { ChatPanel } from "~/components/chat-panel";

export function meta({}: Route.MetaArgs) {
  return [
    { title: "Jisi Code" },
    { name: "description", content: "AI Coding Tool Orchestrator" },
  ];
}

export default function Home() {
  return <ChatPanel />;
}
