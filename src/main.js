const { invoke, convertFileSrc } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const { getCurrentWindow } = window.__TAURI__.window;

const appWindow = getCurrentWindow();

// UI Elements
const tabDash = document.getElementById("tab-dash");
const tabHistory = document.getElementById("tab-history");
const tabSettings = document.getElementById("tab-settings");
const contentDash = document.getElementById("content-dash");
const contentHistory = document.getElementById("content-history");
const contentSettings = document.getElementById("content-settings");

const btnMinimize = document.getElementById("btn-minimize");
const btnMaximize = document.getElementById("btn-maximize");
const btnClose = document.getElementById("btn-close");

const subTabAudio = document.getElementById("sub-tab-audio");
const subTabAi = document.getElementById("sub-tab-ai");
const subTabHotkeys = document.getElementById("sub-tab-hotkeys");
const subTabModels = document.getElementById("sub-tab-models");
const subContentAudio = document.getElementById("sub-content-audio");
const subContentAi = document.getElementById("sub-content-ai");
const subContentHotkeys = document.getElementById("sub-content-hotkeys");
const subContentModels = document.getElementById("sub-content-models");

const transcribeHotkeyDisplay = document.getElementById("transcribe-hotkey-display");
const cancelHotkeyDisplay = document.getElementById("cancel-hotkey-display");
const btnResetHotkeys = document.getElementById("btn-reset-hotkeys");

// Hotkey Recording State Variables
let activeRecordingType = null; // 'transcribe' or 'cancel'
let currentTranscribeKey = "Control";
let currentCancelKey = "Escape";
let tempModifiers = []; // stores temporary modifier keys during recording
let recordedBaseKey = null; // regular non-modifier key pressed during recording
let accumulatedModifiers = []; // tracks the maximum modifier combination pressed during recording

const selectMic = document.getElementById("mic-select");
const btnTestMic = document.getElementById("btn-test-mic");
const micTestBar = document.getElementById("mic-test-bar");
const checkboxNoiseGate = document.getElementById("noise-gate-checkbox");

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
const inputGeminiRefineApiKey = document.getElementById("gemini-refine-api-key");
const btnToggleGeminiRefineKey = document.getElementById("btn-toggle-gemini-refine-key");

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

const groupLmStudio = document.getElementById("lm-studio-settings");
const inputLmStudioUrl = document.getElementById("lm-studio-url");
const inputLmStudioModel = document.getElementById("lm-studio-model");
const groupLmStudioTranscription = document.getElementById("lm-studio-transcription-settings");
const inputLmStudioTranscriptionUrl = document.getElementById("lm-studio-transcription-url");

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

// Dynamic Stats Tracking State
let totalWords = parseInt(localStorage.getItem("murmur_total_words") || "755", 10);
let wpm = parseInt(localStorage.getItem("murmur_wpm") || "127", 10);
let streak = parseInt(localStorage.getItem("murmur_streak") || "1", 10);

function updateStatsUI() {
  const totalWordsEl = document.getElementById("stat-total-words");
  const wpmEl = document.getElementById("stat-wpm");
  const streakEl = document.getElementById("stat-streak");

  if (totalWordsEl) totalWordsEl.textContent = totalWords;
  if (wpmEl) wpmEl.textContent = wpm;
  if (streakEl) streakEl.textContent = streak;

  localStorage.setItem("murmur_total_words", totalWords);
  localStorage.setItem("murmur_wpm", wpm);
  localStorage.setItem("murmur_streak", streak);
}

// Search and filter state
let allHistoryEntries = [];
let searchQuery = "";

const historySearchInput = document.getElementById("history-search");
if (historySearchInput) {
  historySearchInput.addEventListener("input", (e) => {
    searchQuery = e.target.value.toLowerCase();
    filterAndRenderHistory();
  });
}

function filterAndRenderHistory() {
  const filtered = allHistoryEntries.filter(entry => 
    entry.text.toLowerCase().includes(searchQuery)
  );
  renderHistory(filtered);
}

// Window Controls
if (btnMinimize) {
  btnMinimize.addEventListener("click", () => appWindow.minimize());
}
if (btnMaximize) {
  btnMaximize.addEventListener("click", () => appWindow.toggleMaximize());
}
if (btnClose) {
  btnClose.addEventListener("click", () => appWindow.close());
}

// Bring window forward and make focusable on click/mousedown
document.addEventListener("mousedown", () => {
  invoke("set_window_focusable", { focusable: true }).catch((err) => console.error(err));
});

// Auto-disable focusability on blur so hotkeys work for other apps
window.addEventListener("blur", () => {
  invoke("set_window_focusable", { focusable: false }).catch((err) => console.error(err));
});

// Tab Switcher
tabDash.addEventListener("click", () => {
  stopMicTesting(); // Always stop mic test when switching back to dashboard
  tabDash.classList.add("active");
  tabHistory.classList.remove("active");
  tabSettings.classList.remove("active");
  contentDash.classList.add("active");
  contentHistory.classList.remove("active");
  contentSettings.classList.remove("active");
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
function setActiveSubTab(tabName) {
  stopMicTesting(); // Always stop mic test when switching sub-tabs inside settings

  const tabs = {
    audio: { tab: subTabAudio, panel: subContentAudio },
    ai: { tab: subTabAi, panel: subContentAi },
    hotkeys: { tab: subTabHotkeys, panel: subContentHotkeys },
    models: { tab: subTabModels, panel: subContentModels }
  };

  Object.keys(tabs).forEach((key) => {
    const item = tabs[key];
    if (key === tabName) {
      if (item.tab) item.tab.classList.add("active");
      if (item.panel) item.panel.classList.add("active");
    } else {
      if (item.tab) item.tab.classList.remove("active");
      if (item.panel) item.panel.classList.remove("active");
    }
  });

  if (tabName === "models") {
    loadModelsManager();
  }
}

if (subTabAudio) subTabAudio.addEventListener("click", () => setActiveSubTab("audio"));
if (subTabAi) subTabAi.addEventListener("click", () => setActiveSubTab("ai"));
if (subTabHotkeys) subTabHotkeys.addEventListener("click", () => setActiveSubTab("hotkeys"));
if (subTabModels) subTabModels.addEventListener("click", () => setActiveSubTab("models"));

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

// Sync Gemini API Key inputs in real time
inputApiKey.addEventListener("input", () => {
  inputGeminiRefineApiKey.value = inputApiKey.value;
});
inputGeminiRefineApiKey.addEventListener("input", () => {
  inputApiKey.value = inputGeminiRefineApiKey.value;
});

// Sync LM Studio URL inputs in real time
inputLmStudioUrl.addEventListener("input", () => {
  inputLmStudioTranscriptionUrl.value = inputLmStudioUrl.value;
});
inputLmStudioTranscriptionUrl.addEventListener("input", () => {
  inputLmStudioUrl.value = inputLmStudioTranscriptionUrl.value;
});

let isGeminiRefinePasswordVisible = false;
btnToggleGeminiRefineKey.addEventListener("click", () => {
  isGeminiRefinePasswordVisible = !isGeminiRefinePasswordVisible;
  inputGeminiRefineApiKey.type = isGeminiRefinePasswordVisible ? "text" : "password";
  btnToggleGeminiRefineKey.textContent = isGeminiRefinePasswordVisible ? "🙈" : "👁️";
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
  // 7. LM Studio transcription + Cloud refinement or none
  else if (transProvider === "lm_studio" && refProvider === "none") {
    card.classList.add("speed-moderate");
    icon.textContent = "📊";
    badge.textContent = "Moderate (~2.0-4.0s)";
    desc.innerHTML = `Using <strong>LM Studio (Local Whisper)</strong> for transcription with refinement disabled. Runs locally via your LM Studio server.`;
  }
  else if (transProvider === "lm_studio" && (refProvider === "gemini" || refProvider === "openai" || refProvider === "openrouter" || refProvider === "custom")) {
    card.classList.add("speed-moderate");
    icon.textContent = "📊";
    badge.textContent = "Moderate (~3.5-6.0s)";
    desc.innerHTML = `Local <strong>LM Studio</strong> transcription combined with cloud refinement. A hybrid setup for local audio privacy with cloud cleanup quality.`;
  }
  else if (transProvider === "lm_studio" && (refProvider === "ollama" || refProvider === "lm_studio")) {
    card.classList.add("speed-slow");
    icon.textContent = "🐢";
    badge.textContent = "Slow (~10-30s+)";
    desc.innerHTML = `Fully local execution with <strong>LM Studio</strong> for both transcription and refinement. Private but may be slow depending on hardware.`;
  }
  // 8. Any transcription + LM Studio refinement (not already covered)
  else if ((transProvider === "gemini" || transProvider === "openai") && refProvider === "lm_studio") {
    card.classList.add("speed-slow");
    icon.textContent = "🐢";
    badge.textContent = "Slow (~10-25s)";
    desc.innerHTML = `Using a cloud transcription engine but refining with a local <strong>LM Studio LLM</strong> is slower due to local generation latency. Consider switching Refinement Provider to Gemini Cloud or None for a significant speedup.`;
  }
  else if ((transProvider === "local_parakeet" || transProvider === "local_whisper") && refProvider === "lm_studio") {
    card.classList.add("speed-slow");
    icon.textContent = "🐢";
    badge.textContent = "Very Slow (~20-40s+)";
    desc.innerHTML = `Fully offline execution (Local ASR + Local LM Studio) is secure and private but extremely resource-intensive. Expect longer processing times.`;
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
    groupLmStudioTranscription.style.display = "none";
  } else if (transProvider === "local_whisper") {
    groupOpenAi.style.display = "none";
    groupLocalWhisper.style.display = "block";
    groupLmStudioTranscription.style.display = "none";
  } else if (transProvider === "lm_studio") {
    groupOpenAi.style.display = "none";
    groupLocalWhisper.style.display = "none";
    groupLmStudioTranscription.style.display = "block";
  } else {
    groupOpenAi.style.display = "none";
    groupLocalWhisper.style.display = "none";
    groupLmStudioTranscription.style.display = "none";
  }

  // 2. Gemini API Key visibility (Transcription section)
  if (transProvider === "gemini") {
    apiKeyGroup.style.display = "flex";
    const label = apiKeyGroup.querySelector(".form-label");
    label.textContent = "Gemini API Key (Used for transcription)";
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
    groupLmStudio.style.display = "none";
    groupRefinePrompt.style.display = "block";
  } else if (refProvider === "openai") {
    groupGemini.style.display = "none";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "block";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "none";
    groupLmStudio.style.display = "none";
    groupRefinePrompt.style.display = "block";
  } else if (refProvider === "openrouter") {
    groupGemini.style.display = "none";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "block";
    groupCustomApi.style.display = "none";
    groupLmStudio.style.display = "none";
    groupRefinePrompt.style.display = "block";
  } else if (refProvider === "custom") {
    groupGemini.style.display = "none";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "block";
    groupLmStudio.style.display = "none";
    groupRefinePrompt.style.display = "block";
  } else if (refProvider === "ollama") {
    groupGemini.style.display = "none";
    groupOllama.style.display = "block";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "none";
    groupLmStudio.style.display = "none";
    groupRefinePrompt.style.display = "block";
  } else if (refProvider === "lm_studio") {
    groupGemini.style.display = "none";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "none";
    groupLmStudio.style.display = "block";
    groupRefinePrompt.style.display = "block";
  } else {
    // "none"
    groupGemini.style.display = "none";
    groupOllama.style.display = "none";
    groupOpenAiRefine.style.display = "none";
    groupOpenRouter.style.display = "none";
    groupCustomApi.style.display = "none";
    groupLmStudio.style.display = "none";
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
      btnDownloadModel.style.display = "none"; // Hide if already downloaded
    } else {
      badgeOfflineModelStatus.textContent = "Not Downloaded";
      badgeOfflineModelStatus.className = "model-status-badge not-downloaded";
      btnDownloadModel.style.display = "block"; // Show to download
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
  btnDownloadModel.style.display = "none";
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
    btnDownloadModel.style.display = "block";
    showToast(`Download failed: ${err}`, true);
    checkOfflineModelStatus();
  }
});

// Models Manager tab functions
async function loadModelsManager() {
  const models = ["parakeet_v3", "whisper_tiny", "whisper_base", "whisper_small"];
  for (const modelId of models) {
    try {
      const isDownloaded = await invoke("check_model_downloaded", { modelId });
      updateModelCardUI(modelId, isDownloaded);
    } catch (err) {
      console.error(`Failed to check model ${modelId} status:`, err);
    }
  }
}

function updateModelCardUI(modelId, isDownloaded, isDownloading = false) {
  const badge = document.getElementById(`status-badge-${modelId}`);
  const btnDownload = document.getElementById(`btn-download-${modelId}`);
  const btnDelete = document.getElementById(`btn-delete-${modelId}`);
  const progressContainer = document.getElementById(`progress-container-${modelId}`);

  if (!badge || !btnDownload || !btnDelete) return;

  if (isDownloading) {
    badge.textContent = "Downloading";
    badge.className = "model-status-badge downloading";
    btnDownload.style.display = "none";
    btnDelete.style.display = "none";
  } else if (isDownloaded) {
    badge.textContent = "Ready";
    badge.className = "model-status-badge ready";
    btnDownload.style.display = "none";
    btnDelete.style.display = "block";
    btnDelete.disabled = false;
    if (progressContainer) progressContainer.style.display = "none";
  } else {
    badge.textContent = "Not Downloaded";
    badge.className = "model-status-badge not-downloaded";
    btnDownload.style.display = "block";
    btnDownload.disabled = false;
    btnDelete.style.display = "none";
    if (progressContainer) progressContainer.style.display = "none";
  }
}

async function downloadModelFromManager(modelId) {
  const btnDownload = document.getElementById(`btn-download-${modelId}`);
  const badge = document.getElementById(`status-badge-${modelId}`);
  const progressContainer = document.getElementById(`progress-container-${modelId}`);
  const progressStatus = document.getElementById(`progress-status-${modelId}`);
  const progressPercent = document.getElementById(`progress-percent-${modelId}`);
  const progressBar = document.getElementById(`progress-bar-${modelId}`);

  if (btnDownload) btnDownload.disabled = true;
  if (badge) {
    badge.textContent = "Downloading";
    badge.className = "model-status-badge downloading";
  }
  if (progressContainer) {
    progressContainer.style.display = "block";
    if (progressStatus) progressStatus.textContent = "Starting download...";
    if (progressPercent) progressPercent.textContent = "0%";
    if (progressBar) progressBar.style.width = "0%";
  }

  isDownloading = true;

  try {
    await invoke("download_model_files", { modelId });
  } catch (err) {
    isDownloading = false;
    showToast(`Download failed: ${err}`, true);
    loadModelsManager();
  }
}

async function deleteModelFromManager(modelId) {
  if (confirm(`Are you sure you want to delete the offline files for this model? This action cannot be undone.`)) {
    const btnDelete = document.getElementById(`btn-delete-${modelId}`);
    if (btnDelete) btnDelete.disabled = true;
    try {
      await invoke("delete_model_files", { modelId });
      showToast("Model files deleted successfully");
      loadModelsManager();
      // Also update the active model status card in AI settings if applicable
      checkOfflineModelStatus();
    } catch (err) {
      showToast(`Failed to delete model: ${err}`, true);
      loadModelsManager();
    }
  }
}

function bindModelManagerEvents() {
  const models = ["parakeet_v3", "whisper_tiny", "whisper_base", "whisper_small"];
  models.forEach(modelId => {
    const btnDownload = document.getElementById(`btn-download-${modelId}`);
    const btnDelete = document.getElementById(`btn-delete-${modelId}`);

    if (btnDownload) {
      btnDownload.addEventListener("click", () => downloadModelFromManager(modelId));
    }
    if (btnDelete) {
      btnDelete.addEventListener("click", () => deleteModelFromManager(modelId));
    }
  });
}

// Subscribe to progress events from the backend downloader
listen("model-download-progress", (event) => {
  const payload = event.payload; // { model_id, file_index, total_files, file_name, progress, status }
  const modelId = payload.model_id;

  // 1. Update Models Manager view elements
  const managerContainer = document.getElementById(`progress-container-${modelId}`);
  const managerStatus = document.getElementById(`progress-status-${modelId}`);
  const managerPercent = document.getElementById(`progress-percent-${modelId}`);
  const managerBar = document.getElementById(`progress-bar-${modelId}`);
  const managerBadge = document.getElementById(`status-badge-${modelId}`);
  const managerDownloadBtn = document.getElementById(`btn-download-${modelId}`);
  const managerDeleteBtn = document.getElementById(`btn-delete-${modelId}`);

  if (managerBadge) {
    managerBadge.textContent = "Downloading";
    managerBadge.className = "model-status-badge downloading";
  }
  if (managerDownloadBtn) managerDownloadBtn.style.display = "none";
  if (managerDeleteBtn) managerDeleteBtn.style.display = "none";

  if (managerContainer) {
    managerContainer.style.display = "block";
    if (managerStatus) managerStatus.textContent = payload.status;
    if (managerPercent) managerPercent.textContent = `${Math.round(payload.progress)}%`;
    if (managerBar) managerBar.style.width = `${payload.progress}%`;
  }

  // Handle completion for Models Manager view
  if (payload.progress >= 100.0 && payload.file_name === "") {
    isDownloading = false;
    showToast("Model downloaded successfully!");
    loadModelsManager();
    checkOfflineModelStatus();
  }

  // 2. Update AI Settings view elements if it's the active selected model
  const currentTransProvider = selectTranscriptionProvider.value;
  let activeModelId = "";
  if (currentTransProvider === "local_parakeet") {
    activeModelId = "parakeet_v3";
  } else if (currentTransProvider === "local_whisper") {
    activeModelId = "whisper_" + selectLocalWhisperModel.value;
  }

  if (modelId === activeModelId) {
    containerDownloadProgress.style.display = "block";
    badgeOfflineModelStatus.textContent = "Downloading";
    badgeOfflineModelStatus.className = "model-status-badge downloading";
    btnDownloadModel.style.display = "none";

    textDownloadStatus.textContent = payload.status;
    textDownloadPercent.textContent = `${Math.round(payload.progress)}%`;
    barDownloadProgress.style.width = `${payload.progress}%`;

    if (payload.progress >= 100.0 && payload.file_name === "") {
      badgeOfflineModelStatus.textContent = "Ready";
      badgeOfflineModelStatus.className = "model-status-badge ready";
      btnDownloadModel.style.display = "none"; // Hide since downloaded
      setTimeout(() => {
        if (!isDownloading) {
          containerDownloadProgress.style.display = "none";
        }
      }, 3000);
    } else {
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

selectTranscriptionLanguage.addEventListener("change", async () => {
  dashboardLanguageSelect.value = selectTranscriptionLanguage.value;
  await autoSaveConfig();
});

checkboxNoiseGate.addEventListener("change", async () => {
  await autoSaveConfig();
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
    noise_gate: checkboxNoiseGate.checked,
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
    lm_studio_url: inputLmStudioUrl.value.trim() || inputLmStudioTranscriptionUrl.value.trim(),
    lm_studio_model: inputLmStudioModel.value.trim(),
    transcribe_key: currentTranscribeKey,
    notes_key: currentNotesKey,
    cancel_key: currentCancelKey,
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
    inputGeminiRefineApiKey.value = config.api_key || "";
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
    checkboxNoiseGate.checked = config.noise_gate || false;

    inputLmStudioUrl.value = config.lm_studio_url || "http://localhost:1234";
    inputLmStudioTranscriptionUrl.value = config.lm_studio_url || "http://localhost:1234";
    inputLmStudioModel.value = config.lm_studio_model || "";

    currentTranscribeKey = config.transcribe_key || "Control";
    currentCancelKey = config.cancel_key || "Escape";

    renderVisualKeycapsFromKeyName(currentTranscribeKey, transcribeHotkeyDisplay);
    renderVisualKeycapsFromKeyName(currentCancelKey, cancelHotkeyDisplay);
    updateDashboardKeycaps(currentTranscribeKey);
  } catch (err) {
    showToast("Error loading saved configurations", true);
  }
}

function renderVisualKeycapsFromKeyName(val, displayEl) {
  if (!displayEl) return;
  displayEl.innerHTML = "";
  
  if (!val) return;
  const parts = val.split(" + ");
  parts.forEach(part => {
    let displayPart = part;
    if (displayPart === "Control") displayPart = "Ctrl";
    if (displayPart === "Escape") displayPart = "esc";
    
    const span = document.createElement("span");
    span.className = "visual-keycap";
    span.textContent = displayPart;
    displayEl.appendChild(span);
  });
  
  const pencil = document.createElement("span");
  pencil.className = "edit-pencil-indicator";
  pencil.textContent = "✏️";
  displayEl.appendChild(pencil);
}

function updateDashboardKeycaps(keyName) {
  const keycapRow = document.querySelector(".keycap-row");
  if (!keycapRow) return;

  keycapRow.innerHTML = "";

  let displayKey = keyName || "Control";
  if (displayKey === "Control") {
    displayKey = "Ctrl";
  }

  const span = document.createElement("span");
  span.className = "keycap";
  span.textContent = displayKey;
  keycapRow.appendChild(span);
}

// Add event listeners for Hotkey triggers click to enter recording
if (transcribeHotkeyDisplay) {
  transcribeHotkeyDisplay.addEventListener("click", (e) => {
    e.stopPropagation();
    enterRecordingMode("transcribe");
  });
}
if (cancelHotkeyDisplay) {
  cancelHotkeyDisplay.addEventListener("click", (e) => {
    e.stopPropagation();
    enterRecordingMode("cancel");
  });
}

function getDisplayElement(type) {
  if (type === "transcribe") return transcribeHotkeyDisplay;
  if (type === "cancel") return cancelHotkeyDisplay;
  return null;
}

function getActiveDisplayElement() {
  return getDisplayElement(activeRecordingType);
}

function enterRecordingMode(type) {
  if (activeRecordingType) {
    exitRecordingMode(false);
  }

  activeRecordingType = type;
  tempModifiers = [];
  recordedBaseKey = null;
  accumulatedModifiers = [];

  const displayEl = getDisplayElement(type);
  if (!displayEl) return;

  displayEl.classList.add("recording");
  displayEl.innerHTML = '<span class="recording-dot"></span><span>Recording...</span>';
  displayEl.focus();
}

async function exitRecordingMode(saveChanges) {
  if (!activeRecordingType) return;

  const type = activeRecordingType;
  const displayEl = getDisplayElement(type);
  
  activeRecordingType = null;
  tempModifiers = [];
  recordedBaseKey = null;
  accumulatedModifiers = [];

  if (displayEl) {
    displayEl.classList.remove("recording");
    displayEl.blur();
  }

  let currentKey = "Control";
  if (type === "transcribe") {
    currentKey = currentTranscribeKey;
  } else if (type === "cancel") {
    currentKey = currentCancelKey;
  }

  renderVisualKeycapsFromKeyName(currentKey, displayEl);

  if (saveChanges) {
    if (type === "transcribe") {
      updateDashboardKeycaps(currentTranscribeKey);
    }
    await autoSaveConfig();
  }
}

function isShortcutDuplicate(combination, type) {
  if (type !== "transcribe" && combination === currentTranscribeKey) {
    return "Push to talk";
  }
  if (type !== "cancel" && combination === currentCancelKey) {
    return "Cancel";
  }
  return null;
}

function renderTemporaryKeycaps(keys, baseKey) {
  const displayEl = getActiveDisplayElement();
  if (!displayEl) return;

  displayEl.innerHTML = "";
  keys.forEach(part => {
    let displayPart = part;
    if (displayPart === "Control") displayPart = "Ctrl";
    if (displayPart === "Escape") displayPart = "esc";
    
    const span = document.createElement("span");
    span.className = "visual-keycap";
    span.textContent = displayPart;
    displayEl.appendChild(span);
  });

  if (baseKey) {
    let displayPart = baseKey;
    if (displayPart === "Escape") displayPart = "esc";
    
    const span = document.createElement("span");
    span.className = "visual-keycap";
    span.textContent = displayPart;
    displayEl.appendChild(span);
  }

  const pencil = document.createElement("span");
  pencil.className = "edit-pencil-indicator";
  pencil.textContent = "✏️";
  displayEl.appendChild(pencil);
}

function mapModifierKey(key) {
  if (key === "Control" || key === "Ctrl") return "Control";
  if (key === "Meta" || key === "OS" || key === "Win") return "Win";
  if (key === "Shift") return "Shift";
  if (key === "Alt") return "Alt";
  return null;
}

function mapKeyToCanonical(e) {
  if (["Control", "Shift", "Alt", "Meta", "OS"].includes(e.key)) {
    return null;
  }

  if (e.key === " ") {
    return "Space";
  }

  if (e.key.length === 1) {
    return e.key.toUpperCase();
  }

  return e.key;
}

// Global Keyboard Recording Listeners
window.addEventListener("keydown", (e) => {
  if (!activeRecordingType) return;

  e.preventDefault();
  e.stopPropagation();

  // Escape key cancels active recording only if not recording 'cancel' itself (unless modifiers are held)
  if (e.key === "Escape" && activeRecordingType !== "cancel" && !e.ctrlKey && !e.shiftKey && !e.altKey && !e.metaKey) {
    exitRecordingMode(false);
    return;
  }

  // Determine held modifiers
  let heldModifiers = [];
  if (e.ctrlKey) heldModifiers.push("Control");
  if (e.metaKey) heldModifiers.push("Win");
  if (e.shiftKey) heldModifiers.push("Shift");
  if (e.altKey) heldModifiers.push("Alt");

  // If e.key is a modifier itself, we can make sure it is included
  const keyCanonical = mapModifierKey(e.key);
  if (keyCanonical && !heldModifiers.includes(keyCanonical)) {
    heldModifiers.push(keyCanonical);
  }

  let orderedModifiers = [];
  if (heldModifiers.includes("Control")) orderedModifiers.push("Control");
  if (heldModifiers.includes("Win")) orderedModifiers.push("Win");
  if (heldModifiers.includes("Shift")) orderedModifiers.push("Shift");
  if (heldModifiers.includes("Alt")) orderedModifiers.push("Alt");

  const isModifier = ["Control", "Shift", "Alt", "Meta", "OS"].includes(e.key);

  if (!isModifier) {
    // It's a regular key, which means this key combination is complete!
    let canonicalKey = mapKeyToCanonical(e);
    if (canonicalKey) {
      recordedBaseKey = canonicalKey;
      tempModifiers = orderedModifiers;
      
      // Construct final combination string
      let combination = "";
      if (tempModifiers.length > 0) {
        combination = tempModifiers.join(" + ") + " + " + recordedBaseKey;
      } else {
        combination = recordedBaseKey;
      }
      
      // Perform duplicate check
      const duplicateType = isShortcutDuplicate(combination, activeRecordingType);
      if (duplicateType) {
        showToast(`Shortcut '${combination}' is already in use by ${duplicateType}!`, true);
        exitRecordingMode(false);
        return;
      }
      
      // Uniquely set and save
      if (activeRecordingType === "transcribe") {
        currentTranscribeKey = combination;
      } else if (activeRecordingType === "notes") {
        currentNotesKey = combination;
      } else if (activeRecordingType === "cancel") {
        currentCancelKey = combination;
      }
      
      exitRecordingMode(true);
    }
  } else {
    // It is a modifier key. Update held and accumulated modifier states
    tempModifiers = orderedModifiers;
    if (orderedModifiers.length > accumulatedModifiers.length) {
      accumulatedModifiers = orderedModifiers;
    }
    renderTemporaryKeycaps(orderedModifiers, null);
  }
}, { capture: true });

window.addEventListener("keyup", (e) => {
  if (!activeRecordingType) return;

  e.preventDefault();
  e.stopPropagation();

  // If a regular key was already pressed and recorded, keydown handled it.
  // We only handle keyup to detect modifier-only sequences when all modifiers are released.
  const isModifier = ["Control", "Shift", "Alt", "Meta", "OS"].includes(e.key);
  if (!isModifier) return;

  let heldModifiers = [];
  if (e.ctrlKey) heldModifiers.push("Control");
  if (e.metaKey) heldModifiers.push("Win");
  if (e.shiftKey) heldModifiers.push("Shift");
  if (e.altKey) heldModifiers.push("Alt");

  let orderedModifiers = [];
  if (heldModifiers.includes("Control")) orderedModifiers.push("Control");
  if (heldModifiers.includes("Win")) orderedModifiers.push("Win");
  if (heldModifiers.includes("Shift")) orderedModifiers.push("Shift");
  if (heldModifiers.includes("Alt")) orderedModifiers.push("Alt");

  if (orderedModifiers.length === 0) {
    // All modifier keys were fully released!
    // Since no regular key was pressed, it was a modifier-only sequence.
    if (accumulatedModifiers.length > 0) {
      const combination = accumulatedModifiers.join(" + ");
      
      // Duplicate check
      const duplicateType = isShortcutDuplicate(combination, activeRecordingType);
      if (duplicateType) {
        showToast(`Shortcut '${combination}' is already in use by ${duplicateType}!`, true);
        exitRecordingMode(false);
        return;
      }
      
      if (activeRecordingType === "transcribe") {
        currentTranscribeKey = combination;
      } else if (activeRecordingType === "cancel") {
        currentCancelKey = combination;
      }
      
      exitRecordingMode(true);
    } else {
      // Just clicked and released without pressing keys, ignore
      exitRecordingMode(false);
    }
  } else {
    // Update active modifiers visually
    tempModifiers = orderedModifiers;
    renderTemporaryKeycaps(orderedModifiers, null);
  }
}, { capture: true });

// Clicking outside exits recording mode without saving
document.addEventListener("mousedown", (e) => {
  if (activeRecordingType) {
    const activeDisplay = getActiveDisplayElement();
    if (activeDisplay && !activeDisplay.contains(e.target)) {
      exitRecordingMode(false);
    }
  }
});

if (btnResetHotkeys) {
  btnResetHotkeys.addEventListener("click", async () => {
    currentTranscribeKey = "Control";
    currentCancelKey = "Escape";

    renderVisualKeycapsFromKeyName(currentTranscribeKey, transcribeHotkeyDisplay);
    renderVisualKeycapsFromKeyName(currentCancelKey, cancelHotkeyDisplay);
    updateDashboardKeycaps("Control");

    await autoSaveConfig();
    showToast("Shortcuts reset to default");
  });
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
    
    // Dynamically increment words and update profile
    const wordsCount = preparedText.split(/\s+/).filter(Boolean).length;
    totalWords += wordsCount;
    // Subtle wpm fluctuation around 125-135
    wpm = Math.round(122 + Math.random() * 10 + (wordsCount % 6));
    updateStatsUI();
  }
});

// Listen for tab switching requests from the system tray
listen("show-tab", (event) => {
  const tabName = event.payload;
  if (tabName === "dashboard" && tabDash) {
    tabDash.click();
  } else if (tabName === "settings" && tabSettings) {
    tabSettings.click();
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
    
    // 3 layered moving gradient waves in premium oranges
    drawWave(3, 28 * currentAmpScale, 0.015, "#eb5e28", 0.4);
    drawWave(2, 18 * currentAmpScale, 0.025, "#f07f54", 0.3);
    drawWave(1.5, 10 * currentAmpScale, 0.035, "#f5a582", 0.2);
  } else {
    currentAmpScale = 0.1; // reset
    if (currentStatus === "Transcribing" || currentStatus === "Pasting") {
      // Draw scanning laser pulse wave in orange
      drawWave(1.0, 8, 0.01, "#eb5e28", 0.25);
      const scanX = (Math.sin(phase * 0.5) + 1) * 0.5 * width;
      ctx.shadowBlur = 15;
      ctx.shadowColor = "#eb5e28";
      ctx.strokeStyle = "rgba(235, 94, 40, 0.8)";
      ctx.lineWidth = 2;
      ctx.beginPath();
      ctx.moveTo(scanX, 10);
      ctx.lineTo(scanX, height - 10);
      ctx.stroke();
      ctx.shadowBlur = 0;
    } else {
      // Idle calm wave in soft warm grey-brown
      drawWave(0.3, 3, 0.008, "#74726b", 0.15);
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
  
  // Get app version
  try {
    const version = await invoke("get_app_version");
    const versionEl = document.getElementById("brand-version");
    if (versionEl) {
      versionEl.textContent = `v${version}`;
    }
  } catch (err) {
    console.error("Failed to load app version:", err);
  }

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
  
  bindModelManagerEvents();
  await loadModelsManager();

  draw();
  await loadHistory();
  await updateLastPreparedFromHistory();

  // Initialize dark mode from saved preference
  initDarkMode();
}

// =====================
// DARK MODE LOGIC
// =====================
function initDarkMode() {
  const darkModeCheckbox = document.getElementById("dark-mode-checkbox");
  const savedTheme = localStorage.getItem("murmur_theme") || "light";

  if (savedTheme === "dark") {
    document.documentElement.setAttribute("data-theme", "dark");
    if (darkModeCheckbox) darkModeCheckbox.checked = true;
  } else {
    document.documentElement.removeAttribute("data-theme");
    if (darkModeCheckbox) darkModeCheckbox.checked = false;
  }

  if (darkModeCheckbox) {
    darkModeCheckbox.addEventListener("change", () => {
      if (darkModeCheckbox.checked) {
        document.documentElement.setAttribute("data-theme", "dark");
        localStorage.setItem("murmur_theme", "dark");
      } else {
        document.documentElement.removeAttribute("data-theme");
        localStorage.setItem("murmur_theme", "light");
      }
    });
  }
}

function calculateStreakFromHistory(entries) {
  if (!entries || entries.length === 0) return 0;
  
  // Extract unique dates as YYYY-MM-DD
  const dates = new Set();
  entries.forEach(entry => {
    if (entry.timestamp) {
      const datePart = entry.timestamp.split(" ")[0]; // Get YYYY-MM-DD
      dates.add(datePart);
    }
  });
  
  // Sort dates in descending order (most recent first)
  const sortedDates = Array.from(dates).sort().reverse();
  if (sortedDates.length === 0) return 0;
  
  const todayStr = new Date().toISOString().split("T")[0];
  const yesterday = new Date();
  yesterday.setDate(yesterday.getDate() - 1);
  const yesterdayStr = yesterday.toISOString().split("T")[0];
  
  // If the most recent date is neither today nor yesterday, streak is broken
  const latestDateStr = sortedDates[0];
  if (latestDateStr !== todayStr && latestDateStr !== yesterdayStr) {
    return 0;
  }
  
  let currentStreak = 0;
  let tempDate = new Date(latestDateStr);
  
  // Count consecutive days going backwards
  for (let i = 0; i < sortedDates.length; i++) {
    const expectedStr = tempDate.toISOString().split("T")[0];
    if (dates.has(expectedStr)) {
      currentStreak++;
      // Move to previous day
      tempDate.setDate(tempDate.getDate() - 1);
    } else {
      break;
    }
  }
  
  return currentStreak;
}

async function loadHistory() {
  try {
    const entries = await invoke("get_history");
    allHistoryEntries = entries;

    // Calculate actual total words from database
    let computedTotalWords = 0;
    entries.forEach(entry => {
      if (entry.text) {
        computedTotalWords += entry.text.split(/\s+/).filter(Boolean).length;
      }
    });
    totalWords = computedTotalWords;

    // Calculate actual day streak from database
    streak = calculateStreakFromHistory(entries);

    // Update the Stats Dashboard
    updateStatsUI();

    filterAndRenderHistory();
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

  // Group entries by date (e.g. "TODAY", "MAY 24, 2026")
  const groups = {};
  const todayStr = new Date().toDateString();

  entries.forEach((entry) => {
    let dateStr = "UNKNOWN";
    let timeStr = "";

    try {
      const parts = entry.timestamp.split(" ");
      if (parts.length >= 2) {
        const dateParts = parts[0].split("-");
        const year = parseInt(dateParts[0], 10);
        const month = parseInt(dateParts[1], 10) - 1;
        const day = parseInt(dateParts[2], 10);

        const dateObj = new Date(year, month, day);
        if (dateObj.toDateString() === todayStr) {
          dateStr = "TODAY";
        } else {
          dateStr = dateObj.toLocaleDateString(undefined, {
            month: 'long',
            day: 'numeric',
            year: 'numeric'
          }).toUpperCase();
        }

        // Format time (e.g., "08:53 AM")
        const timeParts = parts[1].split(":");
        const hour = parseInt(timeParts[0], 10);
        const minute = parseInt(timeParts[1], 10);
        const ampm = hour >= 12 ? "PM" : "AM";
        const displayHour = hour % 12 || 12;
        const displayMinute = minute < 10 ? "0" + minute : minute;
        timeStr = `${displayHour}:${displayMinute} ${ampm}`;
      } else {
        dateStr = "OLDER";
        timeStr = entry.timestamp;
      }
    } catch (e) {
      dateStr = "OLDER";
      timeStr = entry.timestamp;
    }

    if (!groups[dateStr]) {
      groups[dateStr] = [];
    }
    groups[dateStr].push({ entry, timeStr });
  });

  // Render each group
  Object.keys(groups).forEach((groupName) => {
    const groupHeader = document.createElement("div");
    groupHeader.className = "history-group-header";
    groupHeader.textContent = groupName;
    historyList.appendChild(groupHeader);

    groups[groupName].forEach(({ entry, timeStr }) => {
      const row = document.createElement("div");
      row.className = "history-row";

      // Time Column
      const timeCol = document.createElement("div");
      timeCol.className = "history-time-col";
      timeCol.textContent = timeStr;
      row.appendChild(timeCol);

      // Text Column
      const textCol = document.createElement("div");
      textCol.className = "history-text-col";
      textCol.textContent = entry.text;
      row.appendChild(textCol);

      // Hover Actions Column
      const actionsCol = document.createElement("div");
      actionsCol.className = "history-actions-col";

      // Copy Action Button
      const copyBtn = document.createElement("button");
      copyBtn.className = "history-action-icon-btn copy-btn-item";
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
      actionsCol.appendChild(copyBtn);

      // Delete Action Button
      const deleteBtn = document.createElement("button");
      deleteBtn.className = "history-action-icon-btn delete-btn";
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
      actionsCol.appendChild(deleteBtn);

      row.appendChild(actionsCol);
      historyList.appendChild(row);
    });
  });
}

function formatTimestamp(tsString) {
  try {
    if (!tsString) return "";
    let date;
    if (tsString.includes("T")) {
      date = new Date(tsString);
    } else {
      // SQLite format: "2026-05-21 22:34:56"
      const parts = tsString.split(" ");
      if (parts.length >= 2) {
        const dateParts = parts[0].split("-");
        const timeParts = parts[1].split(":");
        if (dateParts.length === 3 && timeParts.length >= 2) {
          const year = parseInt(dateParts[0], 10);
          const month = parseInt(dateParts[1], 10) - 1;
          const day = parseInt(dateParts[2], 10);
          const hour = parseInt(timeParts[0], 10);
          const minute = parseInt(timeParts[1], 10);
          date = new Date(year, month, day, hour, minute);
        }
      }
    }

    if (date && !isNaN(date.getTime())) {
      return date.toLocaleDateString(undefined, {
        month: 'short',
        day: 'numeric',
        year: 'numeric'
      }) + ' at ' + date.toLocaleTimeString(undefined, {
        hour: 'numeric',
        minute: '2-digit',
        hour12: true
      });
    }
    return tsString;
  } catch (e) {
    return tsString;
  }
}init();
