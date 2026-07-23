// @ts-check

import { guideToCymosMarkdown } from "./lib/guide-engine.js";
import { MESSAGE } from "./lib/messages.js";
import { endpointPermissionPattern, normalizeVisualAiEndpoint } from "./lib/endpoint-permissions.js";
import { normalizeVisualAiApiKey, normalizeVisualAiModel } from "./lib/ai-configuration.js";

/** @typedef {{ tabId: number, title: string, url: string, siteEnabled: boolean, visualCaptureEnabled: boolean, visualCaptureBudgetRemaining: number, visualCaptureBudgetLimit: number, visualCaptureBudgetNextAvailableAt: number | null, visualAiCooldownRemainingMs: number }} ActiveTab */
/** @typedef {import("./lib/guide-engine.js").BrowserGuide} BrowserGuide */
/** @typedef {{ endpoint: string, model: string, enabled: boolean, hasSessionApiKey: boolean }} AiConfiguration */
/** @typedef {import("./lib/activity.js").CaptureActivity} CaptureActivity */
/** @typedef {{ guideCount: number, activityCount: number, budgetOriginCount: number }} CaptureSummary */

const elements = {
  captureStatus: byId("capture-status"),
  siteHeading: byId("site-heading"),
  siteUrl: byId("site-url"),
  siteToggle: /** @type {HTMLButtonElement} */ (byId("site-toggle")),
  visualCapture: /** @type {HTMLInputElement} */ (byId("visual-capture")),
  visualBudget: byId("visual-budget"),
  guideHeading: byId("guide-heading"),
  guideSource: byId("guide-source"),
  guideSummary: byId("guide-summary"),
  guideList: /** @type {HTMLOListElement} */ (byId("guide-list")),
  saveGuide: /** @type {HTMLButtonElement} */ (byId("save-guide")),
  activityList: /** @type {HTMLUListElement} */ (byId("activity-list")),
  clearActivity: /** @type {HTMLButtonElement} */ (byId("clear-activity")),
  clearAllCaptureData: /** @type {HTMLButtonElement} */ (byId("clear-all-capture-data")),
  activitySummary: byId("activity-summary"),
  aiForm: /** @type {HTMLFormElement} */ (byId("ai-form")),
  aiEndpoint: /** @type {HTMLInputElement} */ (byId("ai-endpoint")),
  aiModel: /** @type {HTMLInputElement} */ (byId("ai-model")),
  aiKey: /** @type {HTMLInputElement} */ (byId("ai-key")),
  aiEnabled: /** @type {HTMLInputElement} */ (byId("ai-enabled")),
  aiStatus: byId("ai-status"),
  clearKey: /** @type {HTMLButtonElement} */ (byId("clear-key")),
  notice: byId("notice")
};

/** @type {ActiveTab | null} */
let activeTab = null;
/** @type {BrowserGuide | null} */
let activeGuide = null;
/** @type {AiConfiguration | null} */
let aiConfiguration = null;
/** @type {CaptureActivity[]} */
let captureActivity = [];
/** @type {CaptureSummary} */
let captureSummary = { guideCount: 0, activityCount: 0, budgetOriginCount: 0 };
let cooldownRefreshTimer = 0;

elements.siteToggle.addEventListener("click", () => void toggleSite());
elements.visualCapture.addEventListener("change", () => void updateVisualCapture());
elements.saveGuide.addEventListener("click", () => void saveGuideToCymos());
elements.clearActivity.addEventListener("click", () => void clearPageActivity());
elements.clearAllCaptureData.addEventListener("click", () => void clearAllCaptureData());
elements.aiForm.addEventListener("submit", (event) => void saveAiConfiguration(event));
elements.clearKey.addEventListener("click", () => void clearSessionKey());

chrome.runtime.onMessage.addListener((message) => {
  const currentTab = activeTab;
  if (!currentTab || message?.tabId !== currentTab.tabId) return;
  if (message.type === MESSAGE.GUIDE_UPDATED) {
    activeGuide = /** @type {BrowserGuide} */ (message.guide);
    if (
      typeof message.visualCaptureBudgetRemaining === "number"
      && typeof message.visualCaptureBudgetLimit === "number"
      && (typeof message.visualCaptureBudgetNextAvailableAt === "number" || message.visualCaptureBudgetNextAvailableAt === null)
      && typeof message.visualAiCooldownRemainingMs === "number"
    ) {
      activeTab = {
        ...currentTab,
        visualCaptureBudgetRemaining: message.visualCaptureBudgetRemaining,
        visualCaptureBudgetLimit: message.visualCaptureBudgetLimit,
        visualCaptureBudgetNextAvailableAt: message.visualCaptureBudgetNextAvailableAt,
        visualAiCooldownRemainingMs: message.visualAiCooldownRemainingMs
      };
      renderSite();
    }
    renderGuide();
    void refreshActivity();
    return;
  }
  if (message.type === MESSAGE.PAGE_CAPTURE_CLEARED) {
    activeGuide = null;
    captureActivity = [];
    renderGuide();
    void refreshActivity();
  }
});

void refreshPanel();

/** @param {string} id */
function byId(id) {
  const element = document.getElementById(id);
  if (!element) throw new Error(`Missing CYMOS Browser Companion element: ${id}`);
  return element;
}

/** @param {{ type: string, [key: string]: unknown }} message */
async function request(message) {
  const response = await chrome.runtime.sendMessage(message);
  if (response?.error) throw new Error(response.error);
  return response;
}

async function refreshPanel() {
  try {
    const [tab, settings] = await Promise.all([
      request({ type: MESSAGE.GET_ACTIVE_TAB }),
      request({ type: MESSAGE.GET_SETTINGS })
    ]);
    activeTab = /** @type {ActiveTab} */ (tab);
    aiConfiguration = /** @type {AiConfiguration} */ (settings.aiConfiguration);
    const [guide, activity, summary] = await Promise.all([
      request({ type: MESSAGE.GET_GUIDE, tabId: activeTab.tabId }),
      request({ type: MESSAGE.GET_ACTIVITY, tabId: activeTab.tabId }),
      request({ type: MESSAGE.GET_CAPTURE_SUMMARY })
    ]);
    activeGuide = /** @type {BrowserGuide | null} */ (guide);
    captureActivity = /** @type {CaptureActivity[]} */ (activity);
    captureSummary = /** @type {CaptureSummary} */ (summary);
    renderSite();
    renderAiConfiguration();
    renderGuide();
    renderActivity();
    setNotice("");
  } catch (error) {
    activeTab = null;
    activeGuide = null;
    captureActivity = [];
    captureSummary = { guideCount: 0, activityCount: 0, budgetOriginCount: 0 };
    renderSite();
    renderGuide();
    renderActivity();
    setNotice(userMessage(error), "error");
  }
}

function renderSite() {
  const enabled = activeTab?.siteEnabled === true;
  const visualCaptureEnabled = activeTab?.visualCaptureEnabled === true;
  const visualAiCoolingDown = visualCaptureEnabled
    && aiConfiguration?.enabled === true
    && (activeTab?.visualAiCooldownRemainingMs ?? 0) > 0;
  elements.siteHeading.textContent = activeTab?.title || "No supported page";
  elements.siteUrl.textContent = pageUrl(activeTab?.url) || "Open an HTTP or HTTPS page to begin.";
  elements.siteToggle.textContent = enabled ? "Disable this site" : "Enable this site";
  elements.siteToggle.disabled = !activeTab;
  elements.visualCapture.checked = visualCaptureEnabled;
  elements.visualCapture.disabled = !enabled;
  elements.captureStatus.textContent = enabled
    ? (visualAiCoolingDown ? "Visual AI cooling down" : visualCaptureEnabled && aiConfiguration?.enabled ? "Visual AI active" : "Context guide active")
    : "Paused";
  elements.visualBudget.textContent = visualBudgetText(activeTab, enabled);
  window.clearTimeout(cooldownRefreshTimer);
  if (visualAiCoolingDown && activeTab) {
    cooldownRefreshTimer = window.setTimeout(
      () => void refreshPanel(),
      activeTab.visualAiCooldownRemainingMs + 100
    );
  }
}

function renderAiConfiguration() {
  elements.aiEndpoint.value = aiConfiguration?.endpoint || "";
  elements.aiModel.value = aiConfiguration?.model || "";
  elements.aiEnabled.checked = aiConfiguration?.enabled === true;
  elements.aiStatus.textContent = aiConfiguration?.enabled
    ? (aiConfiguration.hasSessionApiKey ? "Session ready" : "Key needed")
    : "Disabled";
}

function renderGuide() {
  elements.guideList.replaceChildren();
  if (!activeGuide) {
    elements.guideHeading.textContent = "Waiting for a pause";
    elements.guideSource.textContent = "Local only";
    elements.guideSummary.textContent = activeTab?.siteEnabled
      ? "Pause over a field, link, or control to generate a local guide."
      : "Enable this site before CYMOS observes pause context.";
    elements.saveGuide.disabled = true;
    return;
  }

  elements.guideHeading.textContent = "Suggested next steps";
  elements.guideSource.textContent = activeGuide.source || "Local context guide";
  elements.guideSummary.textContent = activeGuide.summary || "CYMOS prepared a local guide.";
  for (const step of Array.isArray(activeGuide.steps) ? activeGuide.steps : []) {
    const item = document.createElement("li");
    item.textContent = step;
    elements.guideList.append(item);
  }
  elements.saveGuide.disabled = !Array.isArray(activeGuide.steps) || activeGuide.steps.length === 0;
}

function renderActivity() {
  elements.activityList.replaceChildren();
  elements.clearActivity.disabled = !activeTab || (captureActivity.length === 0 && !activeGuide);
  elements.clearAllCaptureData.disabled = captureSummary.guideCount === 0
    && captureSummary.activityCount === 0
    && captureSummary.budgetOriginCount === 0;
  elements.activitySummary.textContent = captureSummaryText(captureSummary);
  if (captureActivity.length === 0) {
    const item = document.createElement("li");
    item.textContent = "No cursor-pause guides for this page in the current browser session.";
    elements.activityList.append(item);
    return;
  }
  for (const event of captureActivity.slice(0, 6)) {
    const item = document.createElement("li");
    const title = document.createElement("strong");
    title.textContent = `${activityLabel(event)}: ${event.focus}`;
    const detail = document.createElement("span");
    detail.textContent = formatActivityTime(event.capturedAt);
    item.append(title, detail);
    elements.activityList.append(item);
  }
}

/** @param {CaptureActivity} event */
function activityLabel(event) {
  if (event.visualAiOutcome === "completed") return "Visual AI guide";
  if (event.visualAiOutcome === "fallback") return "Visual AI fallback";
  if (event.visualAiOutcome === "unavailable") return "Visual capture unavailable";
  if (event.visualAiOutcome === "cooldown") return "Visual AI cooling down";
  if (event.visualAiOutcome === "quota") return "Visual capture limit reached";
  return "Local guide";
}

async function clearPageActivity() {
  const currentTab = activeTab;
  if (!currentTab) return;
  try {
    captureActivity = /** @type {CaptureActivity[]} */ (await request({ type: MESSAGE.CLEAR_ACTIVITY, tabId: currentTab.tabId }));
    activeGuide = null;
    await refreshActivity();
    renderGuide();
    setNotice("Current-page guide and session activity were cleared.", "success");
  } catch (error) {
    setNotice(userMessage(error), "error");
  }
}

async function clearAllCaptureData() {
  if (
    captureSummary.guideCount === 0
    && captureSummary.activityCount === 0
    && captureSummary.budgetOriginCount === 0
  ) return;
  const confirmed = window.confirm(
    "Clear every Browser Companion guide, activity record, and visual-capture budget for this browser session? Site settings and the visual-AI session key will remain."
  );
  if (!confirmed) return;
  try {
    captureSummary = /** @type {CaptureSummary} */ (await request({ type: MESSAGE.CLEAR_ALL_CAPTURE_DATA }));
    activeTab = /** @type {ActiveTab} */ (await request({ type: MESSAGE.GET_ACTIVE_TAB }));
    activeGuide = null;
    captureActivity = [];
    renderGuide();
    renderSite();
    renderActivity();
    setNotice("All Browser Companion capture data was cleared for this browser session.", "success");
  } catch (error) {
    setNotice(userMessage(error), "error");
  }
}

async function refreshActivity() {
  if (!activeTab) return;
  try {
    const [activity, summary] = await Promise.all([
      request({ type: MESSAGE.GET_ACTIVITY, tabId: activeTab.tabId }),
      request({ type: MESSAGE.GET_CAPTURE_SUMMARY })
    ]);
    captureActivity = /** @type {CaptureActivity[]} */ (activity);
    captureSummary = /** @type {CaptureSummary} */ (summary);
    renderActivity();
  } catch (error) {
    setNotice(userMessage(error), "error");
  }
}

async function toggleSite() {
  if (!activeTab) return;
  setNotice("");
  try {
    const configuration = await request({
      type: MESSAGE.SET_SITE_CONFIGURATION,
      configuration: {
        enabled: !activeTab.siteEnabled,
        visualCaptureEnabled: activeTab.visualCaptureEnabled
      }
    });
    activeTab = { ...activeTab, ...configuration };
    activeGuide = null;
    renderSite();
    renderGuide();
    setNotice(
      configuration.siteEnabled
        ? "CYMOS pause guidance is active for this site."
        : "CYMOS pause guidance is disabled and current-page capture data was cleared.",
      "success"
    );
  } catch (error) {
    setNotice(userMessage(error), "error");
  }
}

async function updateVisualCapture() {
  const currentTab = activeTab;
  if (!currentTab?.siteEnabled) return;
  setNotice("");
  try {
    const configuration = await request({
      type: MESSAGE.SET_SITE_CONFIGURATION,
      configuration: {
        enabled: true,
        visualCaptureEnabled: elements.visualCapture.checked
      }
    });
    activeTab = { ...currentTab, ...configuration };
    renderSite();
    setNotice(
      configuration.visualCaptureEnabled
        ? "Visible-page images remain transient and are sent only when visual AI is enabled."
        : "Visible-page image capture is disabled.",
      "success"
    );
  } catch (error) {
    elements.visualCapture.checked = currentTab.visualCaptureEnabled;
    setNotice(userMessage(error), "error");
  }
}

/** @param {SubmitEvent} event */
async function saveAiConfiguration(event) {
  event.preventDefault();
  let endpoint = elements.aiEndpoint.value.trim();
  let model = elements.aiModel.value.trim();
  let apiKey = elements.aiKey.value;
  const enabled = elements.aiEnabled.checked;
  try {
    if (enabled) {
      endpoint = normalizeVisualAiEndpoint(endpoint);
      model = normalizeVisualAiModel(model);
      if (apiKey) apiKey = normalizeVisualAiApiKey(apiKey);
      const pattern = endpointPermissionPattern(endpoint);
      const granted = await chrome.permissions.request({ origins: [pattern] });
      if (!granted) throw new Error("Endpoint permission is required before visual AI can be enabled.");
      if (!window.confirm("Visible-page images will be sent to the configured AI endpoint while visual context is enabled for a site. Continue?")) return;
    }
    const settings = await request({
      type: MESSAGE.SET_AI_CONFIGURATION,
      configuration: {
        endpoint,
        model,
        enabled,
        apiKey
      }
    });
    aiConfiguration = settings.aiConfiguration;
    elements.aiKey.value = "";
    renderAiConfiguration();
    renderSite();
    setNotice(enabled ? "Visual AI is enabled for this browser session." : "Visual AI is disabled.", "success");
  } catch (error) {
    setNotice(userMessage(error), "error");
  }
}

async function clearSessionKey() {
  try {
    const settings = await request({
      type: MESSAGE.SET_AI_CONFIGURATION,
      configuration: { endpoint: "", model: "", enabled: false, clearApiKey: true }
    });
    aiConfiguration = settings.aiConfiguration;
    elements.aiKey.value = "";
    renderAiConfiguration();
    renderSite();
    setNotice("The visual AI connection, managed endpoint permission, and session key were cleared.", "success");
  } catch (error) {
    setNotice(userMessage(error), "error");
  }
}

async function saveGuideToCymos() {
  if (!activeGuide) return;
  try {
    await navigator.clipboard.writeText(guideToCymosMarkdown(activeGuide));
    setNotice("Guide copied to the local clipboard. CYMOS will capture it when desktop monitoring is active.", "success");
  } catch (error) {
    setNotice(userMessage(error), "error");
  }
}

/** @param {string | undefined} value */
function pageUrl(value) {
  try {
    const url = new URL(value || "");
    return `${url.origin}${url.pathname}`;
  } catch {
    return "";
  }
}

/** @param {string} capturedAt */
function formatActivityTime(capturedAt) {
  const timestamp = new Date(capturedAt);
  return Number.isNaN(timestamp.valueOf()) ? "Time unavailable" : timestamp.toLocaleTimeString();
}

/** @param {CaptureSummary} summary */
function captureSummaryText(summary) {
  const guides = `${summary.guideCount} guide${summary.guideCount === 1 ? "" : "s"}`;
  const events = `${summary.activityCount} event${summary.activityCount === 1 ? "" : "s"}`;
  const budgets = `${summary.budgetOriginCount} visual-site budget${summary.budgetOriginCount === 1 ? "" : "s"}`;
  return `${guides}, ${events}, and ${budgets} held only for this browser session.`;
}

/** @param {ActiveTab | null} tab @param {boolean} siteEnabled */
function visualBudgetText(tab, siteEnabled) {
  if (!tab || tab.visualCaptureBudgetLimit === 0) {
    return "Visual budget is available on HTTP or HTTPS pages.";
  }
  const remaining = tab.visualCaptureBudgetRemaining;
  if (remaining === 0 && typeof tab.visualCaptureBudgetNextAvailableAt === "number") {
    return `Visual capture limit reached. Next allowance at ${formatLocalTime(tab.visualCaptureBudgetNextAvailableAt)}.`;
  }
  const captures = `${remaining} of ${tab.visualCaptureBudgetLimit} visual captures remaining this hour`;
  return siteEnabled ? captures : `${captures} when this site is enabled.`;
}

/** @param {number} timestamp */
function formatLocalTime(timestamp) {
  const time = new Date(timestamp);
  return Number.isNaN(time.valueOf()) ? "a later time" : time.toLocaleTimeString([], { hour: "numeric", minute: "2-digit" });
}

/** @param {string} message @param {"error" | "success" | ""} [tone] */
function setNotice(message, tone = "") {
  elements.notice.textContent = message;
  if (tone) elements.notice.dataset.tone = tone;
  else delete elements.notice.dataset.tone;
}

/** @param {unknown} error */
function userMessage(error) {
  return error instanceof Error ? error.message : "CYMOS Browser Companion could not complete that request.";
}
