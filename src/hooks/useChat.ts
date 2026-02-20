import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";

export interface ChatMessage {
  role: string;
  content: string;
}

export interface ChatStatus {
  status: string;
  isThinking: boolean;
}

export function useChat() {
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [status, setStatus] = useState<ChatStatus>({
    status: "Connecting...",
    isThinking: false,
  });
  const unlistenRefs = useRef<UnlistenFn[]>([]);

  useEffect(() => {
    const setupListeners = async () => {
      const unlistenMsg = await listen<{ role: string; content: string }>(
        "chat-message",
        (event) => {
          setMessages((prev) => [
            ...prev,
            { role: event.payload.role, content: event.payload.content },
          ]);
          if (event.payload.role === "assistant" || event.payload.role === "system") {
            setStatus((prev) => ({ ...prev, isThinking: false }));
          }
        }
      );

      const unlistenStatus = await listen<{
        status: string;
        is_thinking: boolean;
      }>("chat-status", (event) => {
        setStatus({
          status: event.payload.status,
          isThinking: event.payload.is_thinking,
        });
      });

      unlistenRefs.current = [unlistenMsg, unlistenStatus];
    };

    setupListeners();

    return () => {
      unlistenRefs.current.forEach((unlisten) => unlisten());
    };
  }, []);

  const sendMessage = useCallback(
    async (text: string) => {
      if (!text.trim()) return;

      // Add user message to local state immediately
      setMessages((prev) => [...prev, { role: "user", content: text }]);
      setStatus({ status: "Thinking", isThinking: true });

      try {
        await invoke("send_message", { message: text });
      } catch (e) {
        console.error("Failed to send message:", e);
        setMessages((prev) => [
          ...prev,
          { role: "system", content: `Error: ${e}` },
        ]);
        setStatus({ status: "Error", isThinking: false });
      }
    },
    []
  );

  const clearChat = useCallback(async () => {
    setMessages([]);
    try {
      await invoke("clear_chat");
    } catch (e) {
      console.error("Failed to clear chat:", e);
    }
  }, []);

  return { messages, status, sendMessage, clearChat };
}
