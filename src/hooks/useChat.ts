import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen, UnlistenFn } from "@tauri-apps/api/event";
import type { AvatarState, AvatarEmotion } from "../components/AvatarCanvas";

export interface ChatMessage {
    role: string;
    content: string;
}

export interface ChatStatus {
    status: string;
    isThinking: boolean;
}

// Simple emotion detection from response content
function detectEmotion(content: string): AvatarEmotion {
    const lower = content.toLowerCase();
    if (
        lower.includes("!") && lower.includes("?") ||
        lower.includes("뭐") && lower.includes("!") ||
        lower.includes("え") ||
        lower.includes("놀") ||
        lower.includes("대박")
    ) {
        return "surprised";
    }
    if (
        lower.includes("바보") ||
        lower.includes("흥") ||
        lower.includes("짜증") ||
        lower.includes("하아") ||
        lower.includes("변태")
    ) {
        return "angry";
    }
    if (
        lower.includes("ㅎㅎ") ||
        lower.includes("ㅋㅋ") ||
        lower.includes("좋") ||
        lower.includes("감사") ||
        lower.includes("기뻐") ||
        lower.includes("^^")
    ) {
        return "happy";
    }
    if (
        lower.includes("슬프") ||
        lower.includes("아쉽") ||
        lower.includes("미안") ||
        lower.includes("걱정")
    ) {
        return "sad";
    }
    return "neutral";
}

export function useChat() {
    const [messages, setMessages] = useState<ChatMessage[]>([]);
    const [status, setStatus] = useState<ChatStatus>({
        status: "Connecting...",
        isThinking: false,
    });
    const [avatarState, setAvatarState] = useState<AvatarState>("idle");
    const [emotion, setEmotion] = useState<AvatarEmotion>("neutral");
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

                    if (event.payload.role === "assistant") {
                        // Detect emotion from response
                        setEmotion(detectEmotion(event.payload.content));

                        // Speaking animation for a duration based on content length
                        setAvatarState("speaking");
                        const speakDuration = Math.min(
                            Math.max(event.payload.content.length * 50, 2000),
                            15000
                        );
                        setTimeout(() => {
                            setAvatarState("idle");
                            // Reset emotion after a delay
                            setTimeout(() => setEmotion("neutral"), 3000);
                        }, speakDuration);
                    }

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

                // Map status to avatar state
                if (event.payload.is_thinking) {
                    setAvatarState("thinking");
                }
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

            // Don't add user message locally — backend emits it via chat-message event
            // This keeps backend as single source of truth
            setStatus({ status: "Sending", isThinking: true });

            try {
                await invoke("send_message", { message: text });
            } catch (e) {
                console.error("Failed to send message:", e);
                setMessages((prev) => [
                    ...prev,
                    { role: "system", content: `Error: ${e}` },
                ]);
                setStatus({ status: "Error", isThinking: false });
                setAvatarState("idle");
            }
        },
        []
    );

    const clearChat = useCallback(async () => {
        setMessages([]);
        setAvatarState("idle");
        setEmotion("neutral");
        try {
            await invoke("clear_chat");
        } catch (e) {
            console.error("Failed to clear chat:", e);
        }
    }, []);

    return { messages, status, avatarState, emotion, sendMessage, clearChat };
}
