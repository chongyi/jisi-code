#!/usr/bin/env node

const readline = require("node:readline");

const rl = readline.createInterface({
  input: process.stdin,
  crlfDelay: Infinity,
});

function send(payload) {
  process.stdout.write(`${JSON.stringify(payload)}\n`);
}

function chunkText(text, size) {
  const chunks = [];
  for (let i = 0; i < text.length; i += size) {
    chunks.push(text.slice(i, i + size));
  }
  return chunks;
}

function handleInitialize(id) {
  send({
    jsonrpc: "2.0",
    id,
    result: {
      protocol_version: "0.1.0",
      capabilities: {
        content_delta: true,
      },
    },
  });

  send({
    jsonrpc: "2.0",
    method: "acp/statusUpdate",
    params: {
      status: "ready",
    },
  });
}

function handleSendMessage(id, params) {
  const prompt =
    typeof params?.message === "string" ? params.message : String(params?.message ?? "");

  send({
    jsonrpc: "2.0",
    id,
    result: {
      accepted: true,
    },
  });

  const reply = `Mock ACP reply: ${prompt}`;
  const chunks = chunkText(reply, 20);

  chunks.forEach((content, index) => {
    setTimeout(() => {
      send({
        jsonrpc: "2.0",
        method: "acp/contentDelta",
        params: {
          content,
        },
      });
    }, 70 * (index + 1));
  });
}

rl.on("line", (line) => {
  let message;
  try {
    message = JSON.parse(line);
  } catch {
    return;
  }

  if (!message || typeof message !== "object") {
    return;
  }

  const id = message.id;
  const method = message.method;
  const params = message.params;

  if (typeof id !== "number" || typeof method !== "string") {
    return;
  }

  if (method === "acp/initialize") {
    handleInitialize(id);
    return;
  }

  if (method === "acp/sendMessage") {
    handleSendMessage(id, params);
    return;
  }

  if (method === "acp/cancelRequest") {
    send({
      jsonrpc: "2.0",
      id,
      result: {
        cancelled: true,
      },
    });
    return;
  }

  send({
    jsonrpc: "2.0",
    id,
    error: {
      code: -32601,
      message: `Method not found: ${method}`,
    },
  });
});

rl.on("close", () => {
  process.exit(0);
});
