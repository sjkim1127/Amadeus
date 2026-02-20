import { ChatPanel } from "./components/ChatPanel";
import { AvatarCanvas } from "./components/AvatarCanvas";
import { useChat } from "./hooks/useChat";
import "./App.css";

function App() {
    const { messages, status, sendMessage, clearChat } = useChat();

    return (
        <div className="app">
            <div className="app-layout">
                {/* Left: Avatar */}
                <div className="avatar-wrapper">
                    <AvatarCanvas />
                </div>

                {/* Right: Chat */}
                <div className="chat-section">
                    <ChatPanel
                        messages={messages}
                        status={status}
                        onSend={sendMessage}
                        onClear={clearChat}
                    />
                </div>
            </div>
        </div>
    );
}

export default App;
