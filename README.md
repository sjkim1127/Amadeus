<p align="center">
  <img src="https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white" />
  <img src="https://img.shields.io/badge/Tauri_2-FFC131?style=for-the-badge&logo=tauri&logoColor=white" />
  <img src="https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB" />
  <img src="https://img.shields.io/badge/TypeScript-007ACC?style=for-the-badge&logo=typescript&logoColor=white" />
</p>

# Amadeus 🧠

**Amadeus** is a local-first AI assistant inspired by *Steins;Gate*'s Makise Kurisu.  
Runs entirely on your Mac with a local LLM (GGUF), system tools, and a premium desktop UI.

> *"El Psy Kongroo."*

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| 🤖 **Local LLM** | Runs Qwen 2.5 7B locally via `llama.cpp` with Metal GPU acceleration |
| 💬 **Chat UI** | Premium dark-themed React interface with markdown rendering |
| 🔧 **System Tools** | Screenshot, file management, keyboard/mouse input, browser automation |
| 🔊 **Voice (TTS)** | Text-to-speech via macOS `say` command |
| 🎤 **Voice (STT)** | Speech-to-text via Whisper (CoreML) |
| 🧠 **Memory** | Persistent conversation history with SQLite |
| 🎭 **Persona** | Tsundere neuroscientist personality with tool-use capability |

---

## 🏗️ Architecture

```
┌─────────────────────────────────────┐
│           Tauri 2 App               │
│  ┌──────────────────────────────┐   │
│  │   Frontend (React + Vite)    │   │
│  │  ┌────────┐  ┌────────────┐  │   │
│  │  │ Chat UI│  │ (VRM WIP)  │  │   │
│  │  │(React) │  │(three-vrm) │  │   │
│  │  └────────┘  └────────────┘  │   │
│  └──────────────────────────────┘   │
│  ┌──────────────────────────────┐   │
│  │   Rust Backend (Commands)    │   │
│  │  • LLM (llama.cpp / Metal)  │   │
│  │  • Tools (screenshot, etc)  │   │
│  │  • Memory (SQLite)          │   │
│  │  • TTS (say command)        │   │
│  └──────────────────────────────┘   │
└─────────────────────────────────────┘
```

---

## 🚀 Getting Started

### Prerequisites

- **macOS** (Apple Silicon recommended)
- **Rust** (1.75+)
- **Node.js** (18+)
- **Xcode Command Line Tools** (`xcode-select --install`)

### Setup

```bash
# Clone
git clone https://github.com/sjkim1127/Amadeus.git
cd Amadeus

# Install frontend dependencies
npm install

# Download a GGUF model (e.g., Qwen 2.5 7B)
mkdir -p model/localllm
# Place your .gguf file at: model/localllm/qwen2.5-7b-instruct-q4_k_m.gguf

# Run in development mode
npm run tauri dev
```

### Build for Production

```bash
npm run tauri build
```

---

## 📁 Project Structure

```
Amadeus/
├── src/                    # React frontend
│   ├── App.tsx             # Main app layout
│   ├── App.css             # Premium dark theme
│   ├── components/
│   │   └── ChatPanel.tsx   # Chat UI with markdown
│   └── hooks/
│       └── useChat.ts      # Tauri IPC hook
├── src-tauri/              # Rust backend
│   ├── src/
│   │   ├── lib.rs          # Tauri entry + agent loop
│   │   ├── agent/          # Persona, memory, tool dispatch
│   │   ├── llm/            # Local GGUF + Ollama clients
│   │   ├── system/         # Screenshot, files, input, browser
│   │   └── voice/          # TTS (say), STT (whisper)
│   ├── Cargo.toml
│   └── tauri.conf.json
└── index.html
```

---

## 🛠️ Tech Stack

- **Desktop Framework**: [Tauri 2](https://tauri.app/)
- **Frontend**: React 19 + TypeScript + Vite
- **Backend**: Rust (tokio async runtime)
- **LLM**: [llama.cpp](https://github.com/ggml-org/llama.cpp) via `llama-cpp-2` crate (Metal GPU)
- **Database**: SQLite via `sqlx`
- **Voice**: Whisper (STT), macOS `say` (TTS)

---

## 📜 License

This project is licensed under the [Open Software License 3.0](LICENSE).
