import React, { useState, useRef, useEffect } from "react";
import { ChatMessage, ChatStatus } from "../hooks/useChat";

interface ChatPanelProps {
    messages: ChatMessage[];
    status: ChatStatus;
    onSend: (text: string) => void;
    onClear: () => void;
}

export const ChatPanel: React.FC<ChatPanelProps> = ({
    messages,
    status,
    onSend,
    onClear,
}) => {
    const [input, setInput] = useState("");
    const [showSettings, setShowSettings] = useState(false);
    const [ttsEnabled, setTtsEnabled] = useState(true);
    const messagesEndRef = useRef<HTMLDivElement>(null);
    const textareaRef = useRef<HTMLTextAreaElement>(null);

    // Auto-scroll to bottom
    useEffect(() => {
        messagesEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }, [messages, status.isThinking]);

    // Auto-resize textarea
    useEffect(() => {
        if (textareaRef.current) {
            textareaRef.current.style.height = "auto";
            textareaRef.current.style.height =
                Math.min(textareaRef.current.scrollHeight, 120) + "px";
        }
    }, [input]);

    const handleSend = () => {
        const text = input.trim();
        if (text && !status.isThinking) {
            onSend(text);
            setInput("");
        }
    };

    const handleKeyDown = (e: React.KeyboardEvent) => {
        if (e.key === "Enter" && !e.shiftKey) {
            e.preventDefault();
            handleSend();
        }
    };

    const renderMarkdown = (text: string) => {
        // Simple markdown: **bold**, `code`, ```code blocks```, bullet lists
        const lines = text.split("\n");
        const elements: React.ReactNode[] = [];
        let inCodeBlock = false;
        let codeContent: string[] = [];
        let codeLang = "";

        lines.forEach((line, i) => {
            if (line.startsWith("```")) {
                if (inCodeBlock) {
                    elements.push(
                        <pre key={`code-${i}`} className="code-block">
                            <code>{codeContent.join("\n")}</code>
                        </pre>
                    );
                    codeContent = [];
                    inCodeBlock = false;
                } else {
                    codeLang = line.slice(3).trim();
                    inCodeBlock = true;
                }
                return;
            }

            if (inCodeBlock) {
                codeContent.push(line);
                return;
            }

            if (line.startsWith("- ") || line.startsWith("* ")) {
                elements.push(
                    <div key={i} className="bullet-item">
                        <span className="bullet">•</span>
                        <span
                            dangerouslySetInnerHTML={{
                                __html: renderInline(line.slice(2)),
                            }}
                        />
                    </div>
                );
            } else if (line.trim() === "") {
                elements.push(<div key={i} className="spacer" />);
            } else {
                elements.push(
                    <p
                        key={i}
                        className="text-line"
                        dangerouslySetInnerHTML={{ __html: renderInline(line) }}
                    />
                );
            }
        });

        // Unclosed code block
        if (inCodeBlock && codeContent.length > 0) {
            elements.push(
                <pre key="code-end" className="code-block">
                    <code>{codeContent.join("\n")}</code>
                </pre>
            );
        }

        return elements;
    };

    const renderInline = (text: string): string => {
        return text
            .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
            .replace(/`(.*?)`/g, '<code class="inline-code">$1</code>');
    };

    return (
        <div className="chat-panel">
            {/* Toolbar */}
            <div className="toolbar">
                <div className="toolbar-left">
                    <button className="tool-btn" onClick={onClear} title="Clear chat">
                        🗑 Clear
                    </button>
                    <div className="toolbar-divider" />
                    <button
                        className="tool-btn"
                        onClick={() => setShowSettings(!showSettings)}
                    >
                        ⚙ {showSettings ? "▼" : "▶"}
                    </button>
                </div>
                <div className="toolbar-right">
                    <span
                        className="status-dot"
                        style={{
                            backgroundColor: status.isThinking ? "#ffc832" : "#50c850",
                        }}
                    />
                    <span className="status-text">{status.status}</span>
                </div>
            </div>

            {/* Settings */}
            {showSettings && (
                <div className="settings-panel">
                    <h4>Settings</h4>
                    <label className="setting-item">
                        <input
                            type="checkbox"
                            checked={ttsEnabled}
                            onChange={() => setTtsEnabled(!ttsEnabled)}
                        />
                        🔊 Voice Output (TTS)
                    </label>
                </div>
            )}

            {/* Messages */}
            <div className="messages-container">
                {messages.map((msg, i) => (
                    <div key={i} className={`message message-${msg.role}`}>
                        <div className="message-header">
                            {msg.role === "user" && (
                                <span className="sender user-sender">Guest ❯</span>
                            )}
                            {msg.role === "assistant" && (
                                <span className="sender assistant-sender">Amadeus ❯</span>
                            )}
                            {msg.role === "system" && (
                                <span className="sender system-sender">⚙ System</span>
                            )}
                        </div>
                        <div className="message-content">
                            {msg.role === "assistant"
                                ? renderMarkdown(msg.content)
                                : msg.content}
                        </div>
                    </div>
                ))}

                {/* Typing indicator */}
                {status.isThinking && (
                    <div className="message message-thinking">
                        <div className="typing-indicator">
                            <div className="dot" />
                            <div className="dot" />
                            <div className="dot" />
                        </div>
                        <span className="thinking-text">Amadeus가 생각 중...</span>
                    </div>
                )}

                <div ref={messagesEndRef} />
            </div>

            {/* Input */}
            <div className="input-container">
                <textarea
                    ref={textareaRef}
                    value={input}
                    onChange={(e) => setInput(e.target.value)}
                    onKeyDown={handleKeyDown}
                    placeholder="메시지를 입력하세요... (Shift+Enter로 줄바꿈)"
                    rows={1}
                    disabled={status.isThinking}
                />
                <button
                    className="send-btn"
                    onClick={handleSend}
                    disabled={!input.trim() || status.isThinking}
                >
                    Send
                </button>
            </div>
        </div>
    );
};
