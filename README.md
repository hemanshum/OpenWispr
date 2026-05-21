# 🎙️ OpenWispr

OpenWispr is a state-of-the-art, global voice-dictation and transcription utility designed to supercharge your writing and programming workflows. It sits unobtrusively on your desktop, listening to your voice commands, transcribing them with maximum accuracy, and injecting them immediately into your active text editor.

With a beautiful dark-mode glassmorphic interface, real-time waveform visualizers, and multi-LLM refinement support, OpenWispr is a premium tool built with **Tauri**, **Rust**, and **Vanilla HTML/CSS/JavaScript**.

---

## ✨ Features

- **🌐 Global Trigger Key (`Ctrl`)**: Hold down the `Ctrl` key to record your voice instantly. Release the key to transcribe, polish, and paste the output straight into any active window or input field (with fallback clipboard copy for restricted applications).
- **☁️ Cloud & 🔌 Offline Transcription**:
  - **Google Gemini (Cloud)**: Blazing-fast transcription and refinement in a single optimized request (~1.5s).
  - **OpenAI Whisper (Cloud)**: Highly optimized Whisper API cloud transcription (~1.5s).
  - **Local Whisper (Offline)**: Run fully offline using local model weights (e.g., `base`, `tiny`, `small`) optimized for CPU execution.
- **🧠 Multi-Provider AI Refinement**: Refine and rewrite your voice drafts using:
  - **Google Gemini**
  - **OpenAI (GPT)**
  - **OpenRouter**
  - **Ollama** (Local models like Llama 3)
  - **Custom API** (Compatible OpenAI or custom endpoints)
- **⚡ Preset Badge Shortcuts**: Apply specific AI cleanup styles instantly using presets on the dashboard or settings page:
  - `Default` (Clean, remove filler words)
  - `Professional Email` (Draft formal emails)
  - `Developer Notes` (Format as markdown/documentation and code blocks)
  - `Verbatim Transcript` (Exact words, keeping filler sounds)
  - `Prompt Engineer` (Refine spoken instructions into structured LLM prompts)
- **💬 Hover Tooltips**: View exact prompt instructions by hovering over preset badges.
- **📜 Dictation History**: A dedicated tab to view past transcribes with localized date-and-time stamps. Copy previous transcriptions or delete records individually or in bulk.
- **🎨 Visual Waveforms**: High-fidelity dynamic canvas visualizer indicating `Idle`, `Recording` (pulsing level waves), and `Transcribing` (scanning laser pulse) states.
- **🎙️ Mic Tester**: Real-time microphone level bars in settings to test input levels before recording.
- **📱 Responsive & Resizable**: A sleek locked-width window (`620px`) with adjustable vertical height constraints to fit perfectly alongside code editors and browsers.

---

## 🛠️ Installation & Setup

### Prerequisites
Before setting up OpenWispr, ensure you have the following installed on your machine:
- **Rust & Cargo** (Version 1.70 or higher) -> [Install Rust](https://www.rust-lang.org/tools/install)
- **Node.js** (v18 or higher) & **npm** -> [Install Node.js](https://nodejs.org/)

### Get Started

1. **Clone the Repository**
   ```bash
   git clone https://github.com/your-username/OpenWispr.git
   cd OpenWispr
   ```

2. **Install Frontend Dependencies**
   ```bash
   npm install
   ```

3. **Run in Development Mode**
   Start the application in development mode with hot-reloading:
   ```bash
   npm run tauri dev
   ```

4. **Build Production Executable**
   Generate a standalone, optimized installer/executable:
   ```bash
   npm run tauri build
   ```

---

## 🔒 Security & Local Configs

Your API keys, preferences, and dictation history are **completely private and secure**:
- **Zero Cloud Storage**: All configurations are stored locally on your machine in your OS-specific configuration folder (e.g., `%APPDATA%/Roaming/OpenWispr/config.json` on Windows).
- **Safe Repository Push**: The `config.json` file is generated outside of the workspace project folder, meaning your API keys are **never pushed to git/GitHub** when sharing or committing code.
- **Local Database**: Dictation history is stored locally in an offline SQLite database (`history.db`) within your local app data folder and is never transmitted online.
