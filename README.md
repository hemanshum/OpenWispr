# 🎙️ Murmur

Murmur is a state-of-the-art, global voice-dictation and transcription utility designed to supercharge your writing and programming workflows. It sits unobtrusively on your desktop, listening to your voice commands, transcribing them with maximum accuracy, and injecting them immediately into your active text editor.

With a beautiful dark-mode glassmorphic interface, real-time waveform visualizers, and multi-LLM refinement support, Murmur is a premium tool built with **Tauri**, **Rust**, and **Vanilla HTML/CSS/JavaScript**.

---

## ✨ Features

- **🌐 Global Customizable Trigger Keys**: Fully customize global hotkey actions for Push to talk, Hands-free mode, and Cancel directly in settings.
- **⚙️ Interactive Keyboard Shortcut Recorder**:
  - **Push to talk**: Triggers speech-to-text dictation (Default: `Ctrl`).
  - **Hands-free mode**: Dictate hands-free by pressing to start and stop (Default: `Ctrl + Win`).
  - **Cancel Shortcut**: Instantly dismisses active recordings or transcriptions.
  - **Recording Engine**: Click any shortcut display box to enter a pulsing "Recording..." state. Just press your desired shortcut to capture it!
    - Supports rich **multi-modifier combinations** (`Control`, `Win`, `Shift`, `Alt`) for Push to talk and Hands-free mode, saving automatically when all keys are released.
    - Supports **multi-key combinations & chords** (e.g. `Control + Shift + C`, `Alt + Escape`, `F12`) for Cancel, saving instantly on keydown.
  - **Tactile Keycaps UI**: Displays beautiful 3D off-white keycap pills representing your shortcut keys in real time.
  - **Duplication Safeguard**: Automatically checks if a newly recorded shortcut is already assigned to another action. If a duplicate is detected, it blocks the assignment and shows a clear warning toast to keep your hotkeys unique.
- **☁️ Cloud & 🔌 Offline Transcription**:
  - **Google Gemini (Cloud)**: Blazing-fast transcription and refinement in a single optimized request (~1.5s).
  - **OpenAI Whisper (Cloud)**: Highly optimized Whisper API cloud transcription (~1.5s).
  - **Nvidia Parakeet V3 (Offline)**: High-accuracy offline transcription powered by Nvidia's NeMo ASR model. English-only, ~380 MB download.
  - **Local Whisper (Offline)**: Run fully offline using local model weights (e.g., `base`, `tiny`, `small`) optimized for CPU execution. Supports multiple languages.
  - **LM Studio (Local)**: Hook into a local LM Studio server running a Whisper-compatible endpoint (default `http://localhost:1234`) for high-fidelity offline transcription.
- **🧠 Multi-Provider AI Refinement**: Refine and rewrite your voice drafts using:
  - **Google Gemini**
  - **OpenAI (GPT)**
  - **OpenRouter**
  - **Ollama** (Local models like Llama 3)
  - **LM Studio** (Local chat/completions compatible models)
  - **Local LLM (Qwen3 0.6B)** — Fully offline, downloads ~230 MB, no API keys needed
  - **Custom API** (Compatible OpenAI or custom endpoints)
- **🌍 Multilingual Support**: Transcribe in 15+ Indian and international languages including Hindi, Tamil, Telugu, Bengali, Gujarati, Kannada, Malayalam, Marathi, Punjabi, and more. Language selection is provider-aware — offline models are English-only, while cloud models support the full language set.
- **⚡ Performance & Speed Optimizations**:
  - **Persistent Connection Reuse**: Utilizes a shared static HTTP client connection pool to bypass TLS handshakes on every call, saving ~100-300ms per request.
  - **Silence Trimming**: Intelligently crops leading/trailing silence from recordings prior to upload, shrinking WAV payloads by up to 40% for faster uploads.
  - **Upgraded Noise Gate**: Optimized noise gate (threshold at 0.08) applied dynamically, with full support integrated into the settings tab's live audio level indicator.
  - **Performance Advisor**: Real-time speed/quality estimation panel showing expected latency for your chosen ASR × Refinement combination.
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
- **📱 Responsive & Resizable**: A sleek locked-width window (`1080px`) with adjustable vertical height.
- **🎯 One-Click Onboarding**: First-launch wizard with "Plug & Play" (auto-downloads Parakeet V3 + Qwen3 0.6B) or "Custom Setup" paths. Shown only once.
- **🔧 Active Configuration Display**: Dashboard shows your current transcription and refinement providers at a glance.
- **🌗 Dynamic Theme Toggle**: Light mode (☀️ sun icon) and Dark mode (🌙 moon icon) with adaptive UI.

---

## 🛠️ Installation & Setup

### Prerequisites
Before setting up Murmur, ensure you have the following installed on your machine:
- **Rust & Cargo** (Version 1.70 or higher) -> [Install Rust](https://www.rust-lang.org/tools/install)
- **Node.js** (v18 or higher) & **npm** -> [Install Node.js](https://nodejs.org/)

### Get Started

1. **Clone the Repository**
   ```bash
   git clone https://github.com/hemanshum/Murmur.git
   cd Murmur
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

### Releases & Downloads

You can download the latest pre-built installer for version **0.3.0** here:
- **[Download v0.3.0 Release](https://github.com/hemanshum/Murmur/releases/tag/v0.3.0)**

---

## 📋 Release Notes

### v0.3.0
- **Multilingual transcription**: Added support for 15+ languages (Hindi, Tamil, Telugu, Bengali, etc.) with provider-aware language selection
- **Local LLM refinement**: Added Qwen3 0.6B as a fully offline refinement option (~230 MB)
- **One-click onboarding**: First-launch wizard for instant Plug & Play setup — shown only on first install
- **Active Configuration card**: Dashboard now displays current transcription + refinement providers at a glance
- **Performance Advisor**: Comprehensive speed/quality estimations for all 12 ASR × Refinement combinations
- **Dynamic theme toggle**: Light mode shows ☀️ sun icon + "Light", Dark mode shows 🌙 moon icon + "Dark"
- **Improved dropdown styling**: Premium hover/focus glow with accent-orange ring and smooth transitions
- **Debug logs gated for dev only**: `[DEBUG]` messages no longer appear in production builds
- **Window UX fixes**: Header clicks now bring window to front; increased default window height to prevent scrollbar
- **Removed Urdu** from language options

### v0.2.0
- Initial public release with cloud + offline transcription, multi-provider refinement, hotkey system, and history

---

## 🔒 Security & Local Configs

Your API keys, preferences, and dictation history are **completely private and secure**:
- **Zero Cloud Storage**: All configurations are stored locally on your machine in your OS-specific configuration folder (e.g., `%APPDATA%/Roaming/Murmur/config.json` on Windows).
- **Safe Repository Push**: The `config.json` file is generated outside of the workspace project folder, meaning your API keys are **never pushed to git/GitHub** when sharing or committing code.
- **Local Database**: Dictation history is stored locally in an offline SQLite database (`history.db`) within your local app data folder and is never transmitted online.
