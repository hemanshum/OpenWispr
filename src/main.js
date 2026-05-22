const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// UI Elements
const tabDash = document.getElementById("tab-dash");
const tabHistory = document.getElementById("tab-history");
const tabSettings = document.getElementById("tab-settings");
const contentDash = document.getElementById("content-dash");
const contentHistory = document.getElementById("content-history");
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

// Offline Model Manager Elements
const cardOfflineModel = document.getElementById("offline-model-card");
const labelOfflineModelName = document.getElementById("model-name-display");
const badgeOfflineModelStatus = document.getElementById("model-status-badge");
const btnDownloadModel = document.getElementById("btn-download-model");
const containerDownloadProgress = document.getElementById("model-download-progress-container");
const textDownloadStatus = document.getElementById("download-status-text");
const textDownloadPercent = document.getElementById("download-percent-text");
const barDownloadProgress = document.getElementById("download-progress-bar");

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
const dashboardLanguageSelect = document.getElementById("dashboard-language-select");
const dashboardPresetBadges = document.querySelectorAll(".dashboard-preset-badge");

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

const lastPreparedContainer = document.getElementById("last-prepared-container");
const lastPreparedText = document.getElementById("last-prepared-text");
const btnCopyLast = document.getElementById("btn-copy-last");

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
  tabHistory.classList.remove("active");
  tabSettings.classList.remove("active");
  contentDash.classList.add("active");
  contentHistory.classList.remove("active");
  contentSettings.classList.remove("active");
  invoke("set_window_focusable", { focusable: false }).catch((err) => console.error(err));
});

tabHistory.addEventListener("click", () => {
  stopMicTesting();
  tabHistory.classList.add("active");
  tabDash.classList.remove("active");
  tabSettings.classList.remove("active");
  contentHistory.classList.add("active");
  contentDash.classList.remove("active");
  contentSettings.classList.remove("active");
  invoke("set_window_focusable", { focusable: true }).catch((err) => console.error(err));
  loadHistory();
});

tabSettings.addEventListener("click", () => {
  tabSettings.classList.add("active");
  tabDash.classList.remove("active");
  tabHistory.classList.remove("active");
  contentSettings.classList.add("active");
  contentDash.classList.remove("active");
  contentHistory.classList.remove("active");
  invoke("set_window_focusable", { focusable: true }).catch((err) => console.error(err));
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

  // 1. Cloud-to-Cloud (Blazing Fast)
  if ((transProvider === "gemini" || transProvider === "openai") && refProvider === "none") {
    card.classList.add("speed-blazing");
    icon.textContent = "🚀";
    badge.textContent = "Blazing Fast (~1.5s)";
    desc.innerHTML = `Using <strong>${transProvider === "gemini" ? "Google Gemini" : "OpenAI Whisper"} (Cloud)</strong> transcription with refinement disabled runs extremely fast. Perfect for quick, near-instant dictation.`;
  } else if (transProvider === "gemini" && refProvider === "gemini") {
    card.classList.add("speed-blazing");
    icon.textContent = "🚀";
    badge.textContent = "Blazing Fast (~1.5s)";
    desc.innerHTML = `Using <strong>Google Gemini (Cloud)</strong> for both transcription and refinement executes in a single optimized API request. Highly recommended for near-instant responses.`;
  }
  // 2. Cloud transcription + Cloud refinement (Fast)
  else if ((transProvider === "gemini" || transProvider === "openai") && 
             (refProvider === "gemini" || refProvider === "openai" || refProvider === "openrouter" || refProvider === "custom")) {
    card.classList.add("speed-fast");
    icon.textContent = "⚡";
    badge.textContent = "Fast (~3.0s)";
    desc.innerHTML = `Uses cloud-based transcription with <strong>${refProvider === "openai" ? "OpenAI GPT" : refProvider === "openrouter" ? "OpenRouter" : refProvider === "custom" ? "Custom API" : "Google Gemini"}</strong> refinement. Fast and highly accurate refinement with minimal network overhead.`;
  }
  // 3. Local Parakeet / Local Whisper + No refinement (Moderate)
  else if ((transProvider === "local_parakeet" || transProvider === "local_whisper") && refProvider === "none") {
    card.classList.add("speed-moderate");
    icon.textContent = "📊";
    badge.textContent = "Moderate (~2.0-4.0s)";
    desc.innerHTML = `Using <strong>${transProvider === "local_parakeet" ? "Nvidia Parakeet V3" : "Local Whisper"} (Offline)</strong> transcription via the optimized sherpa-onnx engine. Run completely offline and privately on your CPU.`;
  }
  // 4. Local Parakeet / Local Whisper + Cloud refinement (Moderate)
  else if ((transProvider === "local_parakeet" || transProvider === "local_whisper") && 
             (refProvider === "gemini" || refProvider === "openai" || refProvider === "openrouter" || refProvider === "custom")) {
    card.classList.add("speed-moderate");
    icon.textContent = "📊";
    badge.textContent = "Moderate (~3.5-6.0s)";
    desc.innerHTML = `Local offline transcription with <strong>${transProvider === "local_parakeet" ? "Nvidia Parakeet V3" : "Local Whisper"}</strong> combined with cloud refinement. A hybrid setup giving local privacy/speed for audio and high-quality LLM cleanup.`;
  }
  // 5. Cloud transcription + Local Ollama refinement (Slow)
  else if ((transProvider === "gemini" || transProvider === "openai") && refProvider === "ollama") {
    card.classList.add("speed-slow");
    icon.textContent = "🐢";
    badge.textContent = "Slow (~10-25s)";
    desc.innerHTML = `Using a cloud transcription engine but refining with a local <strong>Ollama LLM</strong> is slow due to local generation latency. Consider switching Refinement Provider to Gemini Cloud or None for a significant speedup.`;
  }
  // 6. Local transcription + Local Ollama refinement (Very Slow)
  else if ((transProvider === "local_parakeet" || transProvider === "local_whisper") && refProvider === "ollama") {
    card.classList.add("speed-slow");
    icon.textContent = "🐢";
    badge.textContent = "Very Slow (~20-40s+)";
    desc.innerHTML = `Fully offline execution (Local ASR + Local Ollama) is secure and private but extremely resource-intensive on CPU. Expect longer processing times.`;
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

  // Check and update Offline Model status
  checkOfflineModelStatus();
}

selectTranscriptionProvider.addEventListener("change", updateSettingsVisibility);
selectProvider.addEventListener("change", updateSettingsVisibility);
selectLocalWhisperModel.addEventListener("change", checkOfflineModelStatus);

// Offline Model Manager status and download logic
let isDownloading = false;

async function checkOfflineModelStatus() {
  const transProvider = selectTranscriptionProvider.value;
  if (transProvider !== "local_whisper" && transProvider !== "local_parakeet") {
    cardOfflineModel.style.display = "none";
    return;
  }

  cardOfflineModel.style.display = "block";

  let modelId = "";
  let modelName = "";

  if (transProvider === "local_parakeet") {
    modelId = "parakeet_v3";
    modelName = "Nvidia Parakeet V3";
  } else {
    const val = selectLocalWhisperModel.value;
    modelId = "whisper_" + val;
    modelName = "Whisper " + val.charAt(0).toUpperCase() + val.slice(1);
  }

  labelOfflineModelName.textContent = modelName;

  try {
    const isDownloaded = await invoke("check_model_downloaded", { modelId });
    if (isDownloaded) {
      badgeOfflineModelStatus.textContent = "Ready";
      badgeOfflineModelStatus.className = "model-status-badge ready";
      btnDownloadModel.textContent = "Download Again";
      btnDownloadModel.disabled = isDownloading;
    } else {
      badgeOfflineModelStatus.textContent = "Not Downloaded";
      badgeOfflineModelStatus.className = "model-status-badge not-downloaded";
      btnDownloadModel.textContent = "Download Model";
      btnDownloadModel.disabled = isDownloading;
    }
  } catch (err) {
    console.error("Failed to check model status:", err);
  }
}

btnDownloadModel.addEventListener("click", async () => {
  const transProvider = selectTranscriptionProvider.value;
  let modelId = "";
  if (transProvider === "local_parakeet") {
    modelId = "parakeet_v3";
  } else if (transProvider === "local_whisper") {
    modelId = "whisper_" + selectLocalWhisperModel.value;
  } else {
    return;
  }

  isDownloading = true;
  btnDownloadModel.disabled = true;
  badgeOfflineModelStatus.textContent = "Downloading";
  badgeOfflineModelStatus.className = "model-status-badge downloading";
  containerDownloadProgress.style.display = "block";
  textDownloadStatus.textContent = "Starting download...";
  textDownloadPercent.textContent = "0%";
  barDownloadProgress.style.width = "0%";

  try {
    await invoke("download_model_files", { modelId });
  } catch (err) {
    isDownloading = false;
    btnDownloadModel.disabled = false;
    showToast(`Download failed: ${err}`, true);
    checkOfflineModelStatus();
  }
});

// Subscribe to progress events from the backend downloader
listen("model-download-progress", (event) => {
  const payload = event.payload;
  const currentTransProvider = selectTranscriptionProvider.value;

  let activeModelId = "";
  if (currentTransProvider === "local_parakeet") {
    activeModelId = "parakeet_v3";
  } else if (currentTransProvider === "local_whisper") {
    activeModelId = "whisper_" + selectLocalWhisperModel.value;
  }

  if (payload.model_id === activeModelId) {
    containerDownloadProgress.style.display = "block";
    badgeOfflineModelStatus.textContent = "Downloading";
    badgeOfflineModelStatus.className = "model-status-badge downloading";

    textDownloadStatus.textContent = payload.status;
    textDownloadPercent.textContent = `${Math.round(payload.progress)}%`;
    barDownloadProgress.style.width = `${payload.progress}%`;

    if (payload.progress >= 100.0 && payload.file_name === "") {
      isDownloading = false;
      btnDownloadModel.disabled = false;
      btnDownloadModel.textContent = "Download Again";
      badgeOfflineModelStatus.textContent = "Ready";
      badgeOfflineModelStatus.className = "model-status-badge ready";
      setTimeout(() => {
        if (!isDownloading) {
          containerDownloadProgress.style.display = "none";
        }
      }, 3000);
      showToast("Model downloaded successfully!");
    } else {
      isDownloading = true;
      btnDownloadModel.disabled = true;
    }
  }
});

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

// Function to update visual active classes on preset badges
function updateActivePresetBadges(promptText) {
  presetBadges.forEach((badge) => {
    if (badge.getAttribute("data-prompt") === promptText) {
      badge.classList.add("active");
    } else {
      badge.classList.remove("active");
    }
  });

  dashboardPresetBadges.forEach((badge) => {
    if (badge.getAttribute("data-prompt") === promptText) {
      badge.classList.add("active");
    } else {
      badge.classList.remove("active");
    }
  });
}

// Preset Badges Hover Tooltip Logic
const tooltip = document.getElementById("app-tooltip");

function showPresetTooltip(e) {
  const badge = e.currentTarget;
  const text = badge.getAttribute("data-prompt");
  if (!text || !tooltip) return;

  tooltip.textContent = text;
  tooltip.classList.add("show");

  const rect = badge.getBoundingClientRect();
  const tooltipRect = tooltip.getBoundingClientRect();

  // Position above the badge, centered horizontally
  let left = rect.left + (rect.width - tooltipRect.width) / 2;
  let top = rect.top - tooltipRect.height - 8;

  // Prevent going off-screen horizontally
  const padding = 12;
  if (left < padding) {
    left = padding;
  } else if (left + tooltipRect.width > window.innerWidth - padding) {
    left = window.innerWidth - tooltipRect.width - padding;
  }

  // Prevent going off-screen vertically (if there's no space above, show below)
  if (top < padding) {
    top = rect.bottom + 8;
  }

  tooltip.style.left = `${left}px`;
  tooltip.style.top = `${top}px`;
}

function hidePresetTooltip() {
  if (tooltip) {
    tooltip.classList.remove("show");
  }
}

// Bind hover and click events to all badges
document.querySelectorAll(".preset-badge, .dashboard-preset-badge").forEach((badge) => {
  badge.addEventListener("mouseenter", showPresetTooltip);
  badge.addEventListener("mouseleave", hidePresetTooltip);
  badge.addEventListener("click", hidePresetTooltip);
});

// Preset Badges Click Handler (Settings page)
presetBadges.forEach((badge) => {
  badge.addEventListener("click", () => {
    const prompt = badge.getAttribute("data-prompt");
    inputPrompt.value = prompt;
    updateActivePresetBadges(prompt);
    showToast("Selected preset prompt");
  });
});

// Preset Badges Click Handler (Dashboard page)
dashboardPresetBadges.forEach((badge) => {
  badge.addEventListener("click", async () => {
    const prompt = badge.getAttribute("data-prompt");
    inputPrompt.value = prompt;
    updateActivePresetBadges(prompt);
    await autoSaveConfig();
    showToast("Selected preset prompt");
  });
});

// Watch input in settings textarea to dynamically update badge styling
inputPrompt.addEventListener("input", () => {
  updateActivePresetBadges(inputPrompt.value);
});

// Sync Spoken Language selectors
dashboardLanguageSelect.addEventListener("change", async () => {
  selectTranscriptionLanguage.value = dashboardLanguageSelect.value;
  await autoSaveConfig();
  showToast("Language updated");
});

selectTranscriptionLanguage.addEventListener("change", () => {
  dashboardLanguageSelect.value = selectTranscriptionLanguage.value;
});

// Auto Save Settings
async function autoSaveConfig() {
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
  } catch (err) {
    console.error("Auto-save failed:", err);
  }
}

// Save Settings Button Handler
btnSaveSettings.addEventListener("click", async () => {
  await autoSaveConfig();
  showToast("Configurations saved successfully!");
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
    updateActivePresetBadges(config.prompt || "");
    
    selectTranscriptionProvider.value = config.transcription_provider || "gemini";
    selectTranscriptionLanguage.value = config.transcription_language || "auto";
    if (dashboardLanguageSelect) {
      dashboardLanguageSelect.value = config.transcription_language || "auto";
    }
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

// Listen for text-prepared events from Rust backend
listen("text-prepared", (event) => {
  const preparedText = event.payload;
  if (preparedText && preparedText.trim()) {
    lastPreparedText.value = preparedText;
    lastPreparedContainer.style.display = "flex";
  }
});

// Copy Last Prepared Text handler
btnCopyLast.addEventListener("click", async () => {
  const text = lastPreparedText.value;
  if (text) {
    try {
      await navigator.clipboard.writeText(text);
      showToast("Copied to clipboard!");
    } catch (err) {
      showToast("Failed to copy text: " + err, true);
    }
  }
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

async function updateLastPreparedFromHistory() {
  try {
    const entries = await invoke("get_history");
    if (entries && entries.length > 0) {
      lastPreparedText.value = entries[0].text;
      lastPreparedContainer.style.display = "flex";
    } else {
      lastPreparedText.value = "";
      lastPreparedContainer.style.display = "none";
    }
  } catch (err) {
    console.error("Failed to load history for last prepared text:", err);
  }
}

// Initializations
async function init() {
  await initConfig();
  
  // Get initial status
  try {
    const status = await invoke("get_status");
    updateStatusUI(status);
  } catch (_) {}
  
  // Set window non-focusable on start (since dashboard is active)
  invoke("set_window_focusable", { focusable: false }).catch((err) => console.error(err));
  
  // Bind History specific non-tab UI elements
  window.historyCount = document.getElementById("history-count");
  window.btnClearHistory = document.getElementById("btn-clear-history");
  window.historyEmpty = document.getElementById("history-empty");
  window.historyList = document.getElementById("history-list");

  // Clear History Listener
  window.btnClearHistory.addEventListener("click", async () => {
    if (confirm("Are you sure you want to clear all dictation history? This action cannot be undone.")) {
      try {
        await invoke("clear_all_history");
        showToast("Cleared all history");
        await loadHistory();
        await updateLastPreparedFromHistory();
      } catch (err) {
        showToast(`Failed to clear history: ${err}`, true);
      }
    }
  });
  
  draw();
  await updateLastPreparedFromHistory();
}

async function loadHistory() {
  try {
    const entries = await invoke("get_history");
    renderHistory(entries);
  } catch (err) {
    showToast(`Failed to load history: ${err}`, true);
  }
}

function renderHistory(entries) {
  const historyList = document.getElementById("history-list");
  const historyCount = document.getElementById("history-count");
  const historyEmpty = document.getElementById("history-empty");

  if (!historyList || !historyCount || !historyEmpty) return;

  historyList.innerHTML = "";
  historyCount.textContent = `${entries.length} ${entries.length === 1 ? 'entry' : 'entries'}`;
  
  if (entries.length === 0) {
    historyEmpty.style.display = "flex";
    return;
  }
  
  historyEmpty.style.display = "none";
  
  entries.forEach((entry) => {
    const item = document.createElement("div");
    item.className = "history-item";
    
    // Header
    const header = document.createElement("div");
    header.className = "history-item-header";
    
    const timeSpan = document.createElement("span");
    timeSpan.className = "history-item-time";
    timeSpan.textContent = formatTimestamp(entry.timestamp);
    header.appendChild(timeSpan);
    
    const actions = document.createElement("div");
    actions.className = "history-item-actions";
    
    // Copy button
    const copyBtn = document.createElement("button");
    copyBtn.className = "history-item-btn copy-btn-item";
    copyBtn.title = "Copy to clipboard";
    copyBtn.innerHTML = `
      <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
        <path d="M16 1H4c-1.1 0-2 .9-2 2v14h2V3h12V1zm3 4H8c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h11c1.1 0 2-.9 2-2V7c0-1.1-.9-2-2-2zm0 16H8V7h11v14z"/>
      </svg>
    `;
    copyBtn.addEventListener("click", async () => {
      try {
        await navigator.clipboard.writeText(entry.text);
        showToast("Copied to clipboard!");
      } catch (err) {
        showToast(`Failed to copy: ${err}`, true);
      }
    });
    actions.appendChild(copyBtn);
    
    // Delete button
    const deleteBtn = document.createElement("button");
    deleteBtn.className = "history-item-btn delete-btn";
    deleteBtn.title = "Delete entry";
    deleteBtn.innerHTML = `
      <svg viewBox="0 0 24 24" width="14" height="14" fill="currentColor">
        <path d="M6 19c0 1.1.9 2 2 2h8c1.1 0 2-.9 2-2V7H6v12zM19 4h-3.5l-1-1h-5l-1 1H5v2h14V4z"/>
      </svg>
    `;
    deleteBtn.addEventListener("click", async () => {
      try {
        await invoke("delete_history_entry", { id: entry.id });
        showToast("Deleted entry");
        await loadHistory();
        await updateLastPreparedFromHistory();
      } catch (err) {
        showToast(`Failed to delete: ${err}`, true);
      }
    });
    actions.appendChild(deleteBtn);
    
    header.appendChild(actions);
    item.appendChild(header);
    
    // Body
    const body = document.createElement("div");
    body.className = "history-item-body";
    body.textContent = entry.text;
    item.appendChild(body);
    
    historyList.appendChild(item);
  });
}

function formatTimestamp(tsString) {
  // SQLite format: "2026-05-21 22:34:56"
  try {
    const parts = tsString.split(" ");
    if (parts.length < 2) return tsString;
    const dateParts = parts[0].split("-");
    const timeParts = parts[1].split(":");
    if (dateParts.length !== 3 || timeParts.length < 2) return tsString;
    
    const year = parseInt(dateParts[0], 10);
    const month = parseInt(dateParts[1], 10) - 1;
    const day = parseInt(dateParts[2], 10);
    const hour = parseInt(timeParts[0], 10);
    const minute = parseInt(timeParts[1], 10);
    
    const date = new Date(year, month, day, hour, minute);
    
    return date.toLocaleDateString(undefined, {
      month: 'short',
      day: 'numeric',
      year: 'numeric'
    }) + ' at ' + date.toLocaleTimeString(undefined, {
      hour: 'numeric',
      minute: '2-digit',
      hour12: true
    });
  } catch (e) {
    return tsString;
  }
}

init();
