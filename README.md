<p align="center">
  <h1 align="center">🧠 CYMOS</h1>
  <h3 align="center">Control Your Memory Operating System</h3>
  <p align="center">
    <strong>Capture · Connect · Understand · Reason · Act · Remember Forever</strong>
  </p>
  <p align="center">
    <img src="https://img.shields.io/badge/version-1.0.0-a78bfa?style=for-the-badge&labelColor=1e1b2e" alt="Version">
    <img src="https://img.shields.io/badge/tauri-v2-24C8DB?style=for-the-badge&logo=tauri&logoColor=white&labelColor=1e1b2e" alt="Tauri">
    <img src="https://img.shields.io/badge/rust-stable-F97316?style=for-the-badge&logo=rust&logoColor=white&labelColor=1e1b2e" alt="Rust">
    <img src="https://img.shields.io/badge/react-19-61DAFB?style=for-the-badge&logo=react&logoColor=white&labelColor=1e1b2e" alt="React">
    <img src="https://img.shields.io/badge/license-MIT-34d399?style=for-the-badge&labelColor=1e1b2e" alt="MIT License">
  </p>
  <p align="center">
    <em>The Personal Cognitive Operating System for the AI Era.</em>
  </p>
</p>

---

# 🚀 v1.0 — Personal Cognitive Operating System (PCOS)

> **Remember Everything. Understand Everything. Accomplish Anything.**
>
> **The world's first open-source AI-powered Personal Memory Operating System.**

---

## 👁️ Vision

To build a secure, privacy-first cognitive operating system that augments human intelligence by transforming everyday digital interactions into structured, searchable, connected, and actionable knowledge. 

CYMOS becomes the permanent memory layer between humans and artificial intelligence.

## 🎯 Mission

Empower every individual, developer, researcher, student, engineer, and organization with a lifelong AI memory that learns, grows, reasons, and collaborates while keeping complete ownership of personal knowledge.

---

## 🔍 Overview

CYMOS is no longer just a clipboard manager or an AI assistant. It is now a **Personal Cognitive Operating System (PCOS)**.

Every interaction becomes part of your personal knowledge ecosystem. Everything is connected. Everything is searchable. Everything remains under your control.

### What Makes CYMOS Different?

| Traditional Applications | CYMOS |
| :--- | :--- |
| Store Files / Notes | ✓ Captures Knowledge |
| Store Clipboard Items | ✓ Understands Knowledge |
| Siloed Apps | ✓ Connects Knowledge |
| Static Data | ✓ Learns Continuously |
| Simple Search | ✓ Reasons Contextually |
| Reactive UI | ✓ Assists Intelligently |
| Manual Organization | ✓ Automates Repetitive Work |

---

## 🧬 Core Platform Modules

### 1. Memory Engine
Captures and formats everyday digital objects: Clipboard, Images, Files, PDFs, HTML, Rich Text, Code, and URLs.

### 2. AI Intelligence Engine
Performs local LLM analysis to generate AI summaries, auto-categorizations, metadata extraction, keyword generation, and topic detection.

### 3. Semantic Memory Engine
Uses local vector embeddings (`nomic-embed-text`) to enable semantic search, hybrid retrieval, and similar memory identification.

### 4. Knowledge Graph Engine
Extracts entities and relationships automatically to map your thoughts in an interactive visual Graph Explorer.

### 5. Memory Assistant
A RAG-powered chat workspace where you can ask questions naturally:
* *"Show everything I learned about Kubernetes."*
* *"Summarize my BSNL project."*
* *"What Docker commands do I use most?"*

### 6. Agent Intelligence
Specialized AI agents working independently or collaborating on complex tasks: Memory, Research, Planning, Coding, Learning, and Documentation agents.

### 7. Autonomous Intelligence
A background tokio-based cron loop that automatically embeds items, extracts graph relationships, processes duplicates, and records knowledge health metrics.

### 8. Universal Platform
- **Supported OS**: Windows, macOS, Linux.
- **Integrations**: Chrome Extension, VS Code/JetBrains (via local REST API), Terminal, and a developer Plugin SDK.

---

## 🏗️ Architecture

```text
                    User
                      │
                      ▼
          Desktop + Browser + IDE
                      │
                      ▼
            Memory Capture Engine
                      │
                      ▼
          AI Understanding Pipeline
                      │
        ┌─────────────┼─────────────┐
        ▼             ▼             ▼
 Semantic DB     Knowledge Graph   Metadata DB
   (SQLite)       (Local Nodes)     (SQLite)
      │                │               │
      └─────────────┬─────────────────┘
                    ▼
          Agent Intelligence Layer
                    │
                    ▼
      Autonomous Workflow Engine
                    │
                    ▼
          CYMOS Cognitive Dashboard
```

---

## 🛠️ Technology Stack

| Layer | Technology | Purpose |
|---|---|---|
| **Desktop** | [Tauri v2](https://tauri.app) | Native desktop shell, system tray, IPC |
| **Core Language** | Rust | Core logic, clipboard monitoring, AI orchestration |
| **Frontend** | React 19 + TypeScript | UI components, routing, state management |
| **Database** | SQLite (via sqlx) | Relational storage for clips, metrics, and tasks |
| **Vector Search**| Sqlite-vss / Local Hybrid | Vector-based semantic similarity search |
| **REST API** | Axum | Local REST server on `localhost:11435` |
| **AI Runtime** | [Ollama](https://ollama.ai) | Local LLM inference (Qwen 3 / Gemma / Llama) |
| **Embeddings** | nomic-embed-text | Local text vector embeddings |
| **Styling** | CSS (custom design system) | Dark theme, glassmorphism, micro-animations |
| **Icons** | Lucide React | Consistent icon set |

---

## 🔒 Enterprise Features & Security
- **End-to-End Encryption**: AES-256 local database encryption.
- **Offline-First Mode**: Run 100% locally with zero cloud dependencies.
- **Self-Hosted Workspaces**: P2P database synchronization (Roadmap).
- **Zero-Trust Security**: Safe API execution endpoints.
- **Audit Logs**: Transparent logs of background maintenance runs.

---

## ⚡ Quick Start

### Prerequisites

* **Node.js** (>= 18)
* **Rust** (stable toolchain)
* **Ollama** (latest version running locally)

### 1. Clone & Install

```bash
git clone https://github.com/your-username/CtrlCove.git
cd CtrlCove/CtrlCove
npm install
```

### 2. Start Ollama & Pull Models

Ensure the Ollama server is running (`ollama serve`), then pull the required models:

```bash
# Pull model for summaries and agent reasoning (choose one)
ollama pull gemma3:1b

# Pull the embedding model for semantic search
ollama pull nomic-embed-text
```

### 3. Run in Development Mode

```bash
npm run tauri dev
```

### 4. Install the Chrome Extension

1. Navigate to `chrome://extensions/` in Google Chrome or Brave.
2. Toggle on **Developer mode** in the top right.
3. Click **Load unpacked** and select the `CtrlCove/extensions/browser` directory.
4. Highlight text on any page, right-click, and select **Save to CYMOS**!

### 5. Build Production Binaries

```bash
npm run tauri build
```
Generates a native `.dmg` (macOS), `.msi` (Windows), or `.AppImage` (Linux) installer in `src-tauri/target/release/bundle/`.

---

## 🗺️ Roadmap

| Version | Milestone | Status |
|---|---|---|
| `v0.0.1` | Clipboard Vault | ✅ Complete |
| `v0.0.2` | Rich Memory Engine | ✅ Complete |
| `v0.0.3` | AI Memory Engine | ✅ Complete |
| `v0.0.4` | Semantic Memory Engine | ✅ Complete |
| `v0.0.5` | Knowledge Graph Engine | ✅ Complete |
| `v0.0.6` | Memory Assistant Engine | ✅ Complete |
| `v0.0.7` | Agent Intelligence Engine | ✅ Complete |
| `v0.0.8` | Autonomous Intelligence Engine | ✅ Complete |
| `v0.0.9` | Universal Memory Platform (API & Extension) | ✅ Complete |
| `v1.0.0` | Personal Cognitive OS | 🎉 Released |
| `v2.0.0` | CYMOS Cognitive Cloud | 🔜 Planned |

---

## 📜 License

[MIT License](LICENSE) — use it, modify it, ship it.

---

<p align="center">
  <strong>CYMOS</strong><br>
  <em>The Personal Cognitive Operating System for the AI Era.</em><br>
  <strong>Remember Forever.</strong>
</p>
