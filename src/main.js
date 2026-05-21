const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// UI Elements
const tabDash = document.getElementById("tab-dash");
const tabSettings = document.getElementById("tab-settings");
const contentDash = document.getElementById("content-dash");
const contentSettings = document.getElementById("content-settings");

const subTabAudio = document.getElementById("sub-tab-audio");
const subTabAi = document.getElementById("sub-tab-ai");
const subContentAudio = document.getElementById("sub-content-audio");
const subContentAi = document.getElementById("sub-content-ai");

const selectMic = document.getElementById("mic-select");
const btnTestMic = document.getElementById("btn-test-mic");
const micTestBar = document.getElementById("mic-test-bar");

const btnRecord = document.getElementById("btn-record");
const statusBadge = document.getElementById("status-badge");
const statusLabel = document.getElementById("status-label");

const selectTranscriptionProvider = document.getElementById("transcription-provider-select");
const selectTranscriptionLanguage = document.getElementById("transcription-language-select");
const inputApiKey = document.getElementById("api-key");
const apiKeyGroup = document.getElementById("api-key-group");
const groupOpenAi = document.getElementById("openai-settings");
const inputOpenAiApiKey = document.getElementById("openai-api-key");
const btnToggleOpenAiKey = document.getElementById("btn-toggle-openai-key");
const inputOpenAiModel = document.getElementById("openai-model");
const groupLocalWhisper = document.getElementById("local-whisper-settings");
const selectLocalWhisperModel = document.getElementById("local-whisper-model");

const selectProvider = document.getElementById("provider-select");
const groupGemini = document.getElementById("gemini-settings");
const inputModel = document.getElementById("model-input");
const groupOllama = document.getElementById("ollama-settings");
const inputOllamaUrl = document.getElementById("ollama-url");
const inputOllamaModel = document.getElementById("ollama-model");
const groupRefinePrompt = document.getElementById("refine-prompt-group");
const inputPrompt = document.getElementById("refine-prompt");
const btnSaveSettings = document.getElementById("btn-save-settings");
const btnToggleKey = document.getElementById("btn-toggle-key");
const presetBadges = document.querySelectorAll(".preset-badge");

// New Refinement Provider Elements
const groupOpenAiRefine = document.getElementById("openai-refine-settings");
const inputOpenAiRefineApiKey = document.getElementById("openai-refine-api-key");
const btnToggleOpenAiRefineKey = document.getElementById("btn-toggle-openai-refine-key");
const inputOpenAiRefineModel = document.getElementById("openai-refine-model");

const groupOpenRouter = document.getElementById("openrouter-settings");
const inputOpenRouterApiKey = document.getElementById("openrouter-api-key");
const btnToggleOpenRouterKey = document.getElementById("btn-toggle-openrouter-key");
const inputOpenRouterModel = document.getElementById("openrouter-model");

const groupCustomApi = document.getElementById("custom-api-settings");
const inputCustomApiUrl = document.getElementById("custom-api-url");
const inputCustomApiKey = document.getElementById("custom-api-key");
const btnToggleCustomApiKey = document.getElementById("btn-toggle-custom-api-key");
const inputCustomApiModel = document.getElementById("custom-api-model");

const toastAlert = document.getElementById("toast-alert");
const toastText = document.getElementById("toast-text");

const canvas = document.getElementById("canvas-visualizer");
const ctx = canvas.getContext("2d");

// App State
let currentStatus = "Idle";
let isPasswordVisible = false;
let isOpenAiPasswordVisible = false;
let phase = 0;
let isTestingMic = false;
let micLevelUnlisten = null;
let recordingMicLevel = 0;
let currentAmpScale = 0.1;

// Tab Switcher
tabDash.addEventListener("click", () => {
  stopMicTesting(); // Always stop mic test when switching back to dashboard
  tabDash.classList.add("active");
  tabSettings.classList.remove("active");
  contentDash.classList.add("active");
  contentSettings.classList.remove("active");
});

tabSettings.addEventListener("click", () => {
  tabSettings.classList.add("active");
  tabDash.classList.remove("active");
  contentSettings.classList.add("active");
  contentDash.classList.remove("active");
});

// Sub-Tab Switcher inside Settings
subTabAudio.addEventListener("click", () => {
  subTabAudio.classList.add("active");
  subTabAi.classList.remove("active");
  subContentAudio.classList.add("active");
  subContentAi.classList.remove("active");
});

subTabAi.addEventListener("click", () => {
  stopMicTesting(); // Stop mic test when switching away from audio tab
  subTabAi.classList.add("active");
  subTabAudio.classList.remove("active");
  subContentAi.classList.add("active");
  subContentAudio.classList.remove("active");
});

// Toggle API Key Visibility
btnToggleKey.addEventListener("click", () => {
  isPasswordVisible = !isPasswordVisible;
  inputApiKey.type = isPasswordVisible ? "text" : "password";
  btnToggleKey.textContent = isPasswordVisible ? "🙈" : "👁️";
});

// Toggle OpenAI API Key Visibility
btnToggleOpenAiKey.addEventListener("click", () => {
  isOpenAiPasswordVisible = !isOpenAiPasswordVisible;
  inputOpenAiApiKey.type = isOpenAiPasswordVisible ? "text" : "password";
  btnToggleOpenAiKey.textContent = isOpenAiPasswordVisible ? "🙈" : "👁️";
});

let isOpenAiRefinePasswordVisible = false;
let isOpenRouterPasswordVisible = false;
let isCustomApiPasswordVisible = false;

// Toggle OpenAI Refine API Key Visibility
btnToggleOpenAiRefineKey.addEventListener("click", () => {
  isOpenAiRefinePasswordVisible = !isOpenAiRefinePasswordVisible;
  inputOpenAiRefineApiKey.type = isOpenAiRefinePasswordVisible ? "text" : "password";
  btnToggleOpenAiRefineKey.textContent = isOpenAiRefinePasswordVisible ? "🙈" : "👁️";
});

// Toggle OpenRouter API Key Visibility
btnToggleOpenRouterKey.addEventListener("click", () => {
  isOpenRouterPasswordVisible = !isOpenRouterPasswordVisible;
  inputOpenRouterApiKey.type = isOpenRouterPasswordVisible ? "text" : "password";
  btnToggleOpenRouterKey.textContent = isOpenRouterPasswordVisible ? "🙈" : "👁️";
});

// Toggle Custom API Key Visibility
btnToggleCustomApiKey.addEventListener("click", () => {
  isCustomApiPasswordVisible = !isCustomApiPasswordVisible;
  inputCustomApiKey.type = isCustomApiPasswordVisible ? "text" : "password";
  btnToggleCustomApiKey.textContent = isCustomApiPasswordVisible ? "🙈" : "👁️";
});

// Sync OpenAI API Key inputs in real time
inputOpenAiApiKey.addEventListener("input", () => {
  inputOpenAiRefineApiKey.value = inputOpenAiApiKey.value;
});
inputOpenAiRefineApiKey.addEventListener("input", () => {
  inputOpenAiApiKey.value = inputOpenAiRefineApiKey.value;
});

// Model input event listener (removed selectModel dropdown listener)

// Update speed & performance estimation message dynamically
function updatePerformanceAdvisor() {
  const transProvider = selectTranscriptionProvider.value;
  const refProvider = selectProvider.value;

  const card = document.getElementById("performance-advisor");
  const icon = document.getElementById("advisor-icon");
  const badge = document.getElementById("advisor-badge");
  const desc = document.getElementById("advisor-description");

  if (!card || !icon || !badge || !desc) return;

  // Reset classes
  card.className = "performance-advisor";

  if (transProvider === "gemini" && refProvider === "gemini") {
    card.classList.add("speed-blazing");
    icon.textContent = "🚀";
    badge.textContent = "Blazing Fast (~1.5s)";
    desc.innerHTML = `Using <strong>Google Gemini (Cloud)</strong> for both transcription and refinement executes in a single optimized API request. Highly recommended for near-instant responses.`;
  } else if (transProvider === "gemini" && refProvider === "none") {
    card.classList.add("speed-blazing");
    icon.textContent = "🚀";
    badge.textContent = "Blazing Fast (~1.5s)";
    desc.innerHTML = `Using <strong>Google Gemini (Cloud)</strong> transcription with refinement disabled runs extremely fast. Perfect for quick and accurate dictation.`;
  } else if (transProvider === "openai" && refProvider === "none") {
    card.classList.add("speed-blazing");
    icon.textContent = "🚀";
    badge.textContent = "Blazing Fast (~1.5s)";
    desc.innerHTML = `Using <strong>OpenAI Whisper (Cloud)</strong> transcription with refinement disabled is highly optimized and returns in under 2 seconds.`;
  } else if ((transProvider === "gemini" || transProvider === "openai") && (refProvider === "gemini" || refProvider === "openai" || refProvider === "openrouter" || refProvider === "custom")) {
    card.classList.add("speed-fast");
    icon.textContent = "⚡";
    badge.textContent = "Fast (~3.0s)";
    desc.innerHTML = `Uses cloud-based transcription with <strong>${refProvider === "openai" ? "OpenAI GPT" : refProvider === "openrouter" ? "OpenRouter" : refProvider === "custom" ? "Custom API" : "Google Gemini"}</strong> refinement. Fast and highly accurate refinement with minimal network overhead.`;
  } else if ((transProvider === "gemini" || transProvider === "openai") && refProvider === "ollama") {
    card.classList.add("speed-slow");
    icon.textContent = "🐢";
    badge.textContent = "Slow (~10-30s)";
    desc.innerHTML = `Using a cloud transcription engine but refining with a local <strong>Ollama LLM</strong> is slow due to local generation latency. Consider switching Refinement Provider to Gemini Cloud or None for a significant speedup.`;
  } else if (transProvider === "local_whisper" && refProvider === "none") {
    card.classList.add("speed-moderate");
    icon.textContent = "📊";
    badge.textContent = "Moderate (~8-15s)";
    desc.innerHTML = `Local offline transcription requires running Python and PyTorch model weights on your CPU. We added <strong>--fp16 False</strong> to speed it up on CPU, but switching to a Cloud provider is recommended.`;
  } else if (transProvider === "local_whisper" && (refProvider === "gemini" || refProvider === "openai" || refProvider === "openrouter" || refProvider === "custom")) {
    card.classList.add("speed-slow");
    icon.textContent = "🐢";
    badge.textContent = "Slow (~10-20s)";
    desc.innerHTML = `Local Whisper offline transcription takes time to initialize on your CPU. To speed this up, use a smaller Whisper model or switch Transcription Provider to Gemini/OpenAI Cloud.`;
  } else if (transProvider === "local_whisper" && refProvider === "ollama") {
    card.classList.add("speed-slow");
    icon.textContent = "🐢";
    badge.textContent = "Very Slow (~20-40s+)";
    desc.innerHTML = `Fully offline execution (Local Whisper + Local Ollama) is extremely resource-intensive and runs slowly on CPU. Use cloud-based options for the fastest experience.`;
  }
}

// Update visibility of setting blocks based on selected providers
function updateSettingsVisibility() {
  const transProvider = selectTranscriptionProvider.value;
  const refProvider = selectProvider.value;

  // 1. Transcription Provider Inputs visibility
  if (transProvider === "openai") {
    groupOpenAi.style.display = "block";
    groupLocalWhisper.style.display = "none";
  } else if (transProvider === "local_whisper") {
    groupOpenAi.style.display = "none";
    groupLocalWhisper.style.display = "block";
  } else {
    groupOpenAi.style.display = "none";
    groupLocalWhisper.style.display = "none";
  }

  // 2. Gemini API Key visibility
  if (transProvider === "gemini" || refProvider === "gemini") {
    apiKeyGroup.style.display = "flex";
    
    const label = apiKeyGroup.querySelector(".form-label");
    if (transProvider === "gemini" && refProvider === "gemini") {
      label.textContent = "Gemini API Key (Used for transcription & refinement)";
    } else if (transProvider === "gemini") {
      label.textContent = "Gemini API Key (Used for transcription)";
    } else {
      label.textContent = "Gemini API Key (Used for refinement)";
    }
  } else {
    apiKeyGroup.style.display = "none";
  }

  // 3. Refinement Provider visibility
  if (refProvider === "gemini") {
    groupGemini.style.display = "block";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "none";
    groupRefinePrompt.style.display = "block";
  } else if (refProvider === "openai") {
    groupGemini.style.display = "none";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "block";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "none";
    groupRefinePrompt.style.display = "block";
  } else if (refProvider === "openrouter") {
    groupGemini.style.display = "none";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "block";
    groupCustomApi.style.display = "none";
    groupRefinePrompt.style.display = "block";
  } else if (refProvider === "custom") {
    groupGemini.style.display = "none";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "block";
    groupRefinePrompt.style.display = "block";
  } else if (refProvider === "ollama") {
    groupGemini.style.display = "none";
    groupOllama.style.display = "block";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "none";
    groupRefinePrompt.style.display = "block";
  } else {
    // "none"
    groupGemini.style.display = "none";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "none";
    groupRefinePrompt.style.display = "none";
  }

  // Update dynamic performance advisor
  updatePerformanceAdvisor();
}

selectTranscriptionProvider.addEventListener("change", updateSettingsVisibility);
selectProvider.addEventListener("change", updateSettingsVisibility);

// Toast Manager
function showToast(message, isError = false) {
  toastText.textContent = message;
  toastAlert.className = "toast";
  if (isError) {
    toastAlert.classList.add("error");
  }
  toastAlert.classList.add("show");
  setTimeout(() => {
    toastAlert.classList.remove("show");
  }, 3000);
}

// Preset Badges Click Handler
presetBadges.forEach((badge) => {
  badge.addEventListener("click", () => {
    inputPrompt.value = badge.getAttribute("data-prompt");
    showToast("Selected preset prompt");
  });
});

// Save Settings
btnSaveSettings.addEventListener("click", async () => {
  const config = {
    api_key: inputApiKey.value,
    prompt: inputPrompt.value,
    model: inputModel.value.trim(),
    provider: selectProvider.value,
    ollama_url: inputOllamaUrl.value.trim(),
    ollama_model: inputOllamaModel.value.trim(),
    audio_device: selectMic.value,
    transcription_provider: selectTranscriptionProvider.value,
    openai_api_key: inputOpenAiApiKey.value.trim(),
    openai_model: inputOpenAiModel.value.trim(),
    local_whisper_model: selectLocalWhisperModel.value,
    transcription_language: selectTranscriptionLanguage.value,
    openai_refine_model: inputOpenAiRefineModel.value.trim(),
    openrouter_api_key: inputOpenRouterApiKey.value.trim(),
    openrouter_model: inputOpenRouterModel.value.trim(),
    custom_api_url: inputCustomApiUrl.value.trim(),
    custom_api_key: inputCustomApiKey.value.trim(),
    custom_api_model: inputCustomApiModel.value.trim(),
  };

  try {
    await invoke("save_config", { config });
    showToast("Configurations saved successfully!");
  } catch (err) {
    showToast(`Failed to save settings: ${err}`, true);
  }
});

// Manual Trigger Start/Stop
btnRecord.addEventListener("click", async () => {
  if (currentStatus === "Idle") {
    try {
      await invoke("manual_trigger_start");
    } catch (err) {
      showToast(`Error starting: ${err}`, true);
    }
  } else if (currentStatus === "Recording") {
    try {
      await invoke("manual_trigger_stop");
    } catch (err) {
      showToast(`Error stopping: ${err}`, true);
    }
  }
});

// Get audio devices and populate dropdown
async function loadAudioDevices(selectedDevice) {
  try {
    const devices = await invoke("get_audio_devices");
    selectMic.innerHTML = "";
    devices.forEach((device) => {
      const option = document.createElement("option");
      option.value = device;
      option.textContent = device === "Default" ? "Default System Device" : device;
      selectMic.appendChild(option);
    });
    if (selectedDevice && devices.includes(selectedDevice)) {
      selectMic.value = selectedDevice;
    } else {
      selectMic.value = "Default";
    }
  } catch (err) {
    showToast(`Error listing audio devices: ${err}`, true);
  }
}

// Mic Testing Functionality
async function startMicTesting() {
  if (isTestingMic) return;
  try {
    isTestingMic = true;
    btnTestMic.classList.add("testing");
    btnTestMic.querySelector(".btn-text").textContent = "Stop Test";
    
    // Subscribe to level events from Rust
    micLevelUnlisten = await listen("mic-test-level", (event) => {
      const level = event.payload; // 0 to 100
      micTestBar.style.width = `${level}%`;
    });

    const chosenMic = selectMic.value;
    await invoke("start_mic_test", { deviceName: chosenMic === "Default" ? null : chosenMic });
  } catch (err) {
    showToast(`Error starting mic test: ${err}`, true);
    stopMicTesting();
  }
}

async function stopMicTesting() {
  if (!isTestingMic) return;
  isTestingMic = false;
  btnTestMic.classList.remove("testing");
  btnTestMic.querySelector(".btn-text").textContent = "Test Mic";
  
  if (micLevelUnlisten) {
    micLevelUnlisten();
    micLevelUnlisten = null;
  }
  
  micTestBar.style.width = "0%";
  
  try {
    await invoke("stop_mic_test");
  } catch (err) {
    console.error("Error stopping mic test:", err);
  }
}

btnTestMic.addEventListener("click", () => {
  if (isTestingMic) {
    stopMicTesting();
  } else {
    startMicTesting();
  }
});

// Load Config on Startup
async function initConfig() {
  try {
    const config = await invoke("load_config");
    inputApiKey.value = config.api_key || "";
    inputPrompt.value = config.prompt || "";
    
    selectTranscriptionProvider.value = config.transcription_provider || "gemini";
    selectTranscriptionLanguage.value = config.transcription_language || "auto";
    inputOpenAiApiKey.value = config.openai_api_key || "";
    inputOpenAiModel.value = config.openai_model || "whisper-1";
    selectLocalWhisperModel.value = config.local_whisper_model || "base";

    const savedProvider = config.provider || "gemini";
    selectProvider.value = savedProvider;
    
    // Set the refinement OpenAI key from the unified key on load
    inputOpenAiRefineApiKey.value = config.openai_api_key || "";
    inputOpenAiRefineModel.value = config.openai_refine_model || "gpt-4o-mini";

    inputOpenRouterApiKey.value = config.openrouter_api_key || "";
    inputOpenRouterModel.value = config.openrouter_model || "google/gemini-2.5-flash";

    inputCustomApiUrl.value = config.custom_api_url || "";
    inputCustomApiKey.value = config.custom_api_key || "";
    inputCustomApiModel.value = config.custom_api_model || "";

    updateSettingsVisibility();

    inputOllamaUrl.value = config.ollama_url || "http://localhost:11434";
    inputOllamaModel.value = config.ollama_model || "llama3";
    let savedModel = config.model || "gemini-2.0-flash";
    if (savedModel === "gemini-1.5-flash") {
      savedModel = "gemini-2.0-flash";
      // Save healed config back
      config.model = "gemini-2.0-flash";
      invoke("save_config", { config }).catch((err) => console.error("Failed to auto-migrate config model:", err));
    }
    inputModel.value = savedModel;

    // Load available audio devices and highlight saved device
    await loadAudioDevices(config.audio_device);
  } catch (err) {
    showToast("Error loading saved configurations", true);
  }
}

// Update Status UI
function updateStatusUI(status) {
  currentStatus = status;
  statusLabel.textContent = status;
  
  // Reset all classes
  statusBadge.className = "status-pill";
  btnRecord.className = "mic-trigger";

  if (status === "Recording") {
    statusBadge.classList.add("recording");
    btnRecord.classList.add("recording");
  } else if (status === "Transcribing") {
    statusBadge.classList.add("transcribing");
  } else if (status === "Pasting") {
    statusBadge.classList.add("pasting");
  } else if (status.startsWith("Error")) {
    statusBadge.classList.add("error-status");
  }
}

// Listen for status events from Rust
listen("status-changed", (event) => {
  updateStatusUI(event.payload);
  if (event.payload !== "Recording") {
    recordingMicLevel = 0;
  }
});

// Listen for mic levels during recording
listen("recording-mic-level", (event) => {
  recordingMicLevel = event.payload; // 0 to 100
});

// Canvas resizing
function resizeCanvas() {
  canvas.width = canvas.parentElement.clientWidth;
  canvas.height = canvas.parentElement.clientHeight;
}
window.addEventListener("resize", resizeCanvas);
resizeCanvas();

// Draw Waveform Loop
function draw() {
  ctx.clearRect(0, 0, canvas.width, canvas.height);
  const width = canvas.width;
  const height = canvas.height;

  phase += 0.05;

  if (currentStatus === "Recording") {
    const targetScale = recordingMicLevel / 100.0;
    // Smooth transition: 85% of current scale + 15% of target scale
    currentAmpScale += (targetScale - currentAmpScale) * 0.15;
    
    // 3 layered moving gradient waves
    drawWave(3, 28 * currentAmpScale, 0.015, "#a855f7", 0.4);
    drawWave(2, 18 * currentAmpScale, 0.025, "#6366f1", 0.3);
    drawWave(1.5, 10 * currentAmpScale, 0.035, "#06b6d4", 0.2);
  } else {
    currentAmpScale = 0.1; // reset
    if (currentStatus === "Transcribing" || currentStatus === "Pasting") {
      // Draw scanning laser pulse wave
      drawWave(1.0, 8, 0.01, "#a855f7", 0.25);
      const scanX = (Math.sin(phase * 0.5) + 1) * 0.5 * width;
      ctx.shadowBlur = 15;
      ctx.shadowColor = "#06b6d4";
      ctx.strokeStyle = "rgba(6, 182, 212, 0.8)";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(scanX, 10);
      ctx.lineTo(scanX, height - 10);
      ctx.stroke();
      ctx.shadowBlur = 0;
    } else {
      // Idle calm wave
      drawWave(0.3, 3, 0.008, "#94a3b8", 0.15);
    }
  }

  requestAnimationFrame(draw);
}

function drawWave(amplitudeMult, baseAmp, frequency, color, opacity) {
  ctx.beginPath();
  ctx.strokeStyle = color;
  ctx.globalAlpha = opacity;
  ctx.lineWidth = 2;
  ctx.shadowBlur = 8;
  ctx.shadowColor = color;

  const width = canvas.width;
  const centerY = canvas.height / 2;

  for (let x = 0; x < width; x++) {
    const taper = Math.sin((x / width) * Math.PI);
    const y = centerY + Math.sin(x * frequency - phase * amplitudeMult) * baseAmp * taper;

    if (x === 0) {
      ctx.moveTo(x, y);
    } else {
      ctx.lineTo(x, y);
    }
  }
  ctx.stroke();
  ctx.shadowBlur = 0;
  ctx.globalAlpha = 1.0;
}

// Initializations
async function init() {
  await initConfig();
  
  // Get initial status
  try {
    const status = await invoke("get_status");
    updateStatusUI(status);
  } catch (_) {}
  
  draw();
}

init();
