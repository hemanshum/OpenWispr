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

const inputApiKey = document.getElementById("api-key");
const selectProvider = document.getElementById("provider-select");
const groupGemini = document.getElementById("gemini-settings");
const selectModel = document.getElementById("model-select");
const groupCustomModel = document.getElementById("custom-model-group");
const inputCustomModel = document.getElementById("custom-model");
const groupOllama = document.getElementById("ollama-settings");
const inputOllamaUrl = document.getElementById("ollama-url");
const inputOllamaModel = document.getElementById("ollama-model");
const inputPrompt = document.getElementById("refine-prompt");
const btnSaveSettings = document.getElementById("btn-save-settings");
const btnToggleKey = document.getElementById("btn-toggle-key");
const presetBadges = document.querySelectorAll(".preset-badge");

const toastAlert = document.getElementById("toast-alert");
const toastText = document.getElementById("toast-text");

const canvas = document.getElementById("canvas-visualizer");
const ctx = canvas.getContext("2d");

// App State
let currentStatus = "Idle";
let isPasswordVisible = false;
let phase = 0;
let isTestingMic = false;
let micLevelUnlisten = null;

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

// Toggle custom model input group based on dropdown selection
selectModel.addEventListener("change", () => {
  if (selectModel.value === "custom") {
    groupCustomModel.style.display = "flex";
  } else {
    groupCustomModel.style.display = "none";
  }
});

// Toggle provider-specific settings based on provider dropdown selection
selectProvider.addEventListener("change", () => {
  if (selectProvider.value === "ollama") {
    groupOllama.style.display = "block";
    groupGemini.style.display = "none";
  } else {
    groupOllama.style.display = "none";
    groupGemini.style.display = "block";
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

// Preset Badges Click Handler
presetBadges.forEach((badge) => {
  badge.addEventListener("click", () => {
    inputPrompt.value = badge.getAttribute("data-prompt");
    showToast("Selected preset prompt");
  });
});

// Save Settings
btnSaveSettings.addEventListener("click", async () => {
  let modelVal = selectModel.value;
  if (modelVal === "custom") {
    modelVal = inputCustomModel.value.trim();
  }

  const config = {
    api_key: inputApiKey.value,
    prompt: inputPrompt.value,
    model: modelVal,
    provider: selectProvider.value,
    ollama_url: inputOllamaUrl.value.trim(),
    ollama_model: inputOllamaModel.value.trim(),
    audio_device: selectMic.value,
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
    
    const savedProvider = config.provider || "gemini";
    selectProvider.value = savedProvider;
    if (savedProvider === "ollama") {
      groupOllama.style.display = "block";
      groupGemini.style.display = "none";
    } else {
      groupOllama.style.display = "none";
      groupGemini.style.display = "block";
    }

    inputOllamaUrl.value = config.ollama_url || "http://localhost:11434";
    inputOllamaModel.value = config.ollama_model || "llama3";
    
    const savedModel = config.model || "gemini-1.5-flash";
    const presetModels = ["gemini-1.5-flash", "gemini-1.5-pro", "gemini-2.0-flash", "gemini-2.5-flash"];
    
    if (presetModels.includes(savedModel)) {
      selectModel.value = savedModel;
      groupCustomModel.style.display = "none";
      inputCustomModel.value = "";
    } else {
      selectModel.value = "custom";
      groupCustomModel.style.display = "flex";
      inputCustomModel.value = savedModel;
    }

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
    // 3 layered moving gradient waves
    drawWave(3, 28, 0.015, "#a855f7", 0.4);
    drawWave(2, 18, 0.025, "#6366f1", 0.3);
    drawWave(1.5, 10, 0.035, "#06b6d4", 0.2);
  } else if (currentStatus === "Transcribing" || currentStatus === "Pasting") {
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
