// @ts-check

import { buildLocalGuide, normalizeVisionGuide } from "./lib/guide-engine.js";
import { activityForTab, appendActivity, captureActivity, normalizedActivity, withoutActivityForTab } from "./lib/activity.js";
import { originForUrl } from "./lib/redaction.js";
import { isSafeBrowserContext } from "./lib/browser-context.js";
import { MESSAGE } from "./lib/messages.js";
import {
  ACTIVITY_KEY,
  GUIDE_KEY_PREFIX,
  PAGE_INSTANCE_KEY_PREFIX,
  VISUAL_CAPTURE_BUDGETS_KEY,
  captureStorageKeys,
  captureStorageSummary
} from "./lib/session-data.js";
import {
  aiConfigurationEpochIsCurrent,
  captureEpochIsCurrent,
  captureSequenceIsCurrent,
  captureTargetsActiveWindow,
  captureTargetsCurrentDocument,
  captureTargetsCurrentPage
} from "./lib/capture-guard.js";
import {
  endpointPermissionPattern,
  endpointPermissionsToRevoke,
  hasExactEndpointPermission,
  managedEndpointPermissions,
  normalizeVisualAiEndpoint
} from "./lib/endpoint-permissions.js";
import { normalizeVisualAiApiKey, normalizeVisualAiModel } from "./lib/ai-configuration.js";
import {
  buildVisualRequest,
  serializeVisualRequest,
  visualCaptureAllowedForContext
} from "./lib/visual-request.js";
import { parseBoundedVisualResponse, parseVisualGuideContent } from "./lib/visual-response.js";
import { visualCaptureDecision } from "./lib/visual-capture-throttle.js";
import {
  nextVisualCircuitFailure,
  visualCircuitCooldownRemaining,
  visualCircuitDecision
} from "./lib/visual-circuit-breaker.js";
import { createVisualRequestTimeout } from "./lib/visual-request-timeout.js";
import {
  MAX_VISUAL_CAPTURES_PER_ORIGIN,
  normalizeVisualCaptureBudgets,
  reserveVisualCaptureBudget,
  visualCaptureBudgetDecision,
  visualCaptureBudgetNextAvailableAt,
  visualCaptureBudgetRemaining
} from "./lib/visual-capture-budget.js";

const AI_SESSION_SECRET_KEY = "cymos.browser.ai.api-key";
const CAPTURE_COOLDOWN_MS = 2_000;
const MAX_CAPTURES_PER_MINUTE = 12;

/** @typedef {{ endpoint: string, model: string, enabled: boolean }} AiConfiguration */
/** @typedef {{ enabledOrigins: string[], visualCaptureOrigins: string[], aiConfiguration: AiConfiguration, aiEndpointPermissions: string[] }} CompanionSettings */

const defaultAiConfiguration = Object.freeze({ endpoint: "", model: "", enabled: false });
/** @type {Map<number, number[]>} */
const captureWindows = new Map();
/** @type {Map<number, number>} */
const navigationEpochs = new Map();
/** @type {Map<number, number>} */
const captureSequences = new Map();
/** @type {Map<number, string>} */
const pageInstances = new Map();
/** @type {Map<number, AbortController>} */
const visualRequests = new Map();
/** @type {Map<number, import("./lib/visual-capture-throttle.js").VisualCaptureTarget>} */
const visualCaptureTargets = new Map();
/** @type {Map<string, import("./lib/visual-circuit-breaker.js").VisualCircuitState>} */
const visualEndpointCircuits = new Map();
let aiConfigurationEpoch = 0;
/** @type {Promise<void>} */
let sessionWrite = Promise.resolve();

void initializeExtension();

chrome.runtime.onInstalled.addListener(() => void initializeExtension());
chrome.runtime.onStartup.addListener(() => void initializeExtension());
chrome.tabs.onUpdated.addListener((tabId, changeInfo, tab) => {
  if (typeof changeInfo.url === "string") {
    invalidateTabCapture(tabId);
    void resetTabCaptureForNavigation(tabId);
  }
  if (changeInfo.status === "complete" && tab.active) {
    void reinjectConsentedCompanion(tabId, tab);
  }
});
chrome.tabs.onRemoved.addListener((tabId) => {
  abortVisualRequest(tabId);
  captureWindows.delete(tabId);
  navigationEpochs.delete(tabId);
  captureSequences.delete(tabId);
  visualCaptureTargets.delete(tabId);
  void clearPageInstance(tabId);
  void clearActivityForRequestedTab(tabId);
});

chrome.runtime.onMessage.addListener((message, sender, sendResponse) => {
  if (!message || typeof message.type !== "string") return;

  if (message.type === MESSAGE.CONTENT_READY) {
    void contentConfiguration(sender.tab, message.pageInstanceId)
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }

  if (message.type === MESSAGE.CURSOR_PAUSED) {
    void handleCursorPause(message.context, sender.tab);
    return;
  }

  if (message.type === MESSAGE.GET_ACTIVE_TAB) {
    void activeTabSummary()
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }

  if (message.type === MESSAGE.GET_SETTINGS) {
    void publicSettings()
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }

  if (message.type === MESSAGE.GET_GUIDE) {
    void guideForTab(message.tabId)
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }

  if (message.type === MESSAGE.GET_ACTIVITY) {
    void activityForRequestedTab(message.tabId)
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }

  if (message.type === MESSAGE.GET_CAPTURE_SUMMARY) {
    void captureSummary()
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }

  if (message.type === MESSAGE.CLEAR_ACTIVITY) {
    void clearCaptureForRequestedTab(message.tabId)
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }

  if (message.type === MESSAGE.CLEAR_ALL_CAPTURE_DATA) {
    void clearAllCaptureData()
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }

  if (message.type === MESSAGE.SET_SITE_CONFIGURATION) {
    void updateActiveSiteConfiguration(message.configuration)
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }

  if (message.type === MESSAGE.SET_AI_CONFIGURATION) {
    void updateAiConfiguration(message.configuration)
      .then(sendResponse)
      .catch((error) => sendResponse({ error: userMessage(error) }));
    return true;
  }
});

async function initializeExtension() {
  await chrome.sidePanel.setPanelBehavior({ openPanelOnActionClick: true });
  await chrome.storage.session.setAccessLevel({ accessLevel: "TRUSTED_CONTEXTS" });
}

/** @returns {Promise<CompanionSettings>} */
async function settings() {
  const stored = await chrome.storage.local.get([
    "enabledOrigins",
    "visualCaptureOrigins",
    "aiConfiguration",
    "aiEndpointPermissions"
  ]);
  const configuration = /** @type {Partial<AiConfiguration>} */ (stored.aiConfiguration ?? {});
  return {
    enabledOrigins: stringArray(stored.enabledOrigins),
    visualCaptureOrigins: stringArray(stored.visualCaptureOrigins),
    aiEndpointPermissions: stringArray(stored.aiEndpointPermissions),
    aiConfiguration: {
      endpoint: typeof configuration?.endpoint === "string" ? configuration.endpoint : "",
      model: typeof configuration?.model === "string" ? configuration.model : "",
      enabled: configuration?.enabled === true
    }
  };
}

/** @param {unknown} value */
function stringArray(value) {
  return Array.isArray(value) ? value.filter((item) => typeof item === "string") : [];
}

/** @returns {Promise<string>} */
async function sessionApiKey() {
  const stored = await chrome.storage.session.get(AI_SESSION_SECRET_KEY);
  const value = stored[AI_SESSION_SECRET_KEY];
  try {
    return normalizeVisualAiApiKey(value);
  } catch {
    return "";
  }
}

/** @param {chrome.tabs.Tab | undefined} tab @param {unknown} [pageInstanceId] */
async function contentConfiguration(tab, pageInstanceId) {
  if (!tab?.id) return { siteEnabled: false, visualCaptureEnabled: false };
  if (isPageInstanceId(pageInstanceId)) await setPageInstance(tab.id, pageInstanceId);
  const currentSettings = await settings();
  const origin = originForUrl(tab.url ?? "");
  return {
    siteEnabled: Boolean(origin) && currentSettings.enabledOrigins.includes(origin),
    visualCaptureEnabled: Boolean(origin) && currentSettings.visualCaptureOrigins.includes(origin)
  };
}

async function publicSettings() {
  const currentSettings = await settings();
  return {
    aiConfiguration: {
      ...currentSettings.aiConfiguration,
      hasSessionApiKey: Boolean(await sessionApiKey())
    }
  };
}

async function activeTabSummary() {
  const tab = await activeTab();
  const configuration = await contentConfiguration(tab, undefined);
  const visualCaptureBudget = await visualCaptureBudgetSummaryForOrigin(originForUrl(tab.url ?? ""));
  const visualCircuit = visualAiCircuitSummary(await settings());
  if (configuration.siteEnabled) {
    await reinjectConsentedCompanion(tab.id, tab);
  }
  return {
    tabId: tab.id,
    title: tab.title ?? "Current page",
    url: tab.url ?? "",
    ...visualCaptureBudget,
    ...visualCircuit,
    ...configuration
  };
}

/** @returns {Promise<chrome.tabs.Tab & { id: number }>} */
async function activeTab() {
  const [tab] = await chrome.tabs.query({ active: true, lastFocusedWindow: true });
  if (!tab || typeof tab.id !== "number") {
    throw new Error("Open a standard web page before enabling CYMOS Browser Companion.");
  }
  return /** @type {chrome.tabs.Tab & { id: number }} */ (tab);
}

/** @param {unknown} configuration */
async function updateActiveSiteConfiguration(configuration) {
  const tab = await activeTab();
  const origin = originForUrl(tab.url ?? "");
  if (!origin) throw new Error("CYMOS Browser Companion only runs on standard HTTP or HTTPS pages.");
  if (!configuration || typeof configuration !== "object") throw new Error("Invalid site configuration.");

  const requested = /** @type {{ enabled?: unknown, visualCaptureEnabled?: unknown }} */ (configuration);
  const siteEnabled = requested.enabled === true;
  const visualCaptureEnabled = siteEnabled && requested.visualCaptureEnabled === true;
  const currentSettings = await settings();
  const enabledOrigins = updateOriginList(currentSettings.enabledOrigins, origin, siteEnabled);
  const visualCaptureOrigins = updateOriginList(
    currentSettings.visualCaptureOrigins,
    origin,
    visualCaptureEnabled
  );

  await chrome.storage.local.set({ enabledOrigins, visualCaptureOrigins });
  if (siteEnabled) {
    await chrome.scripting.executeScript({ target: { tabId: tab.id }, files: ["content.js"] });
  } else {
    invalidateTabCapture(tab.id);
    await clearTabCapture(tab.id);
  }
  const response = { siteEnabled, visualCaptureEnabled };
  await sendContentConfiguration(tab.id, response);
  return response;
}

/** @param {string[]} origins @param {string} origin @param {boolean} enabled */
function updateOriginList(origins, origin, enabled) {
  const next = new Set(origins);
  if (enabled) next.add(origin);
  else next.delete(origin);
  return [...next].sort();
}

/** @param {number} tabId @param {{ siteEnabled: boolean, visualCaptureEnabled: boolean }} configuration */
async function sendContentConfiguration(tabId, configuration) {
  try {
    await chrome.tabs.sendMessage(tabId, { type: MESSAGE.SITE_CONFIGURATION, configuration });
  } catch {
    // The page can navigate between injection and configuration delivery.
  }
}

/** @param {number} tabId @param {chrome.tabs.Tab} tab */
async function reinjectConsentedCompanion(tabId, tab) {
  const configuration = await contentConfiguration(tab);
  if (!configuration.siteEnabled) return;
  try {
    await chrome.scripting.executeScript({ target: { tabId }, files: ["content.js"] });
    await sendContentConfiguration(tabId, configuration);
  } catch {
    // activeTab access ends after navigation to an unapproved origin.
  }
}

/** @param {unknown} configuration */
async function updateAiConfiguration(configuration) {
  if (!configuration || typeof configuration !== "object") throw new Error("Invalid AI configuration.");
  const requested = /** @type {{ endpoint?: unknown, model?: unknown, enabled?: unknown, apiKey?: unknown, clearApiKey?: unknown }} */ (configuration);
  let endpoint = typeof requested.endpoint === "string" ? requested.endpoint.trim() : "";
  let model = typeof requested.model === "string" ? requested.model.trim() : "";
  const enabled = requested.enabled === true;
  const clearApiKey = requested.clearApiKey === true;
  const rawApiKey = typeof requested.apiKey === "string" ? requested.apiKey : "";
  const apiKey = enabled && rawApiKey ? normalizeVisualAiApiKey(rawApiKey) : "";
  const currentSettings = await settings();
  let nextPermission = "";

  if (enabled) {
    endpoint = normalizeVisualAiEndpoint(endpoint);
    model = normalizeVisualAiModel(model);
    nextPermission = endpointPermissionPattern(endpoint);
    const allowed = await chrome.permissions.contains({ origins: [nextPermission] });
    if (!allowed) throw new Error("Approve the endpoint permission before enabling visual AI.");
    if (!apiKey && (clearApiKey || !(await sessionApiKey()))) {
      throw new Error("Enter an API key for this browser session before enabling visual AI.");
    }
  }

  const nextConfiguration = enabled
    ? { endpoint, model, enabled: true }
    : { ...defaultAiConfiguration };
  const previousPermission = storedEndpointPermissionPattern(currentSettings.aiConfiguration.endpoint);
  const managedPermissions = managedEndpointPermissions(
    currentSettings.aiEndpointPermissions,
    previousPermission
  );
  const permissionsToRevoke = endpointPermissionsToRevoke(managedPermissions, nextPermission);
  // Treat the configuration as changed before asynchronous storage/permission work begins.
  // Queued visual responses will observe the new epoch before they can be stored.
  invalidateVisualAiWork();
  if (permissionsToRevoke.length > 0) {
    await chrome.permissions.remove({ origins: permissionsToRevoke });
  }
  await chrome.storage.local.set({
    aiConfiguration: nextConfiguration,
    aiEndpointPermissions: nextPermission ? [nextPermission] : []
  });
  if (clearApiKey) await chrome.storage.session.remove(AI_SESSION_SECRET_KEY);
  else if (enabled && apiKey) await chrome.storage.session.set({ [AI_SESSION_SECRET_KEY]: apiKey });
  return publicSettings();
}

/** @param {string} endpoint */
function storedEndpointPermissionPattern(endpoint) {
  if (!endpoint) return "";
  try {
    return endpointPermissionPattern(endpoint);
  } catch {
    return "";
  }
}

/** @param {unknown} rawContext @param {chrome.tabs.Tab | undefined} senderTab */
async function handleCursorPause(rawContext, senderTab) {
  if (!senderTab || typeof senderTab.id !== "number" || typeof senderTab.windowId !== "number" || !isSafeBrowserContext(rawContext)) return;
  const tabId = senderTab.id;
  const windowId = senderTab.windowId;
  const context = /** @type {import("./lib/guide-engine.js").BrowserContext} */ (rawContext);
  const capturedEpoch = navigationEpochs.get(tabId) ?? 0;
  let tab;
  try {
    tab = await chrome.tabs.get(tabId);
  } catch {
    return;
  }
  if (!tab.active) return;
  if (!captureTargetsCurrentPage(tab.url ?? "", context.pageUrl)) return;
  if (!captureTargetsCurrentDocument(await pageInstanceForTab(tabId), context.pageInstanceId)) return;

  const siteConfiguration = await contentConfiguration(tab);
  if (!siteConfiguration.siteEnabled || !captureAllowed(tabId)) return;
  const captureSequence = beginCapture(tabId);
  const capturedAiConfigurationEpoch = aiConfigurationEpoch;

  /** @type {import("./lib/guide-engine.js").BrowserGuide} */
  let guide = buildLocalGuide(context);
  let visualAiAttempted = false;
  let visualGuideApplied = false;
  /** @type {import("./lib/activity.js").VisualAiOutcome} */
  let visualAiOutcome = "not_requested";
  const currentSettings = await settings();
  const apiKey = await sessionApiKey();
  const visualCaptureRequested = siteConfiguration.visualCaptureEnabled
    && currentSettings.aiConfiguration.enabled
    && Boolean(apiKey);
  const repeatDecision = visualCaptureDecision(visualCaptureTargets.get(tabId), context);
  const circuitDecision = visualCircuitDecision(
    visualEndpointCircuits.get(currentSettings.aiConfiguration.endpoint)
  );
  const visualContextAllowsCapture = visualCaptureAllowedForContext(context);
  const visualEndpointPermissionIsCurrent = visualCaptureRequested
    && circuitDecision.allowed
    && repeatDecision.allowed
    && visualContextAllowsCapture
    && await visualAiEndpointPermissionIsCurrent(currentSettings);
  const visualBudgetReserved = visualEndpointPermissionIsCurrent
    && aiConfigurationEpochIsCurrent(aiConfigurationEpoch, capturedAiConfigurationEpoch)
    && await reserveVisualCaptureBudgetForOrigin(originForUrl(context.pageUrl));
  if (visualCaptureRequested && !aiConfigurationEpochIsCurrent(aiConfigurationEpoch, capturedAiConfigurationEpoch)) {
    visualAiOutcome = "unavailable";
  } else if (visualCaptureRequested && !circuitDecision.allowed) {
    visualAiOutcome = "cooldown";
  } else if (visualCaptureRequested && !repeatDecision.allowed) {
    // The current local guide remains useful without repeating remote image egress for the same target.
  } else if (visualCaptureRequested && (!visualEndpointPermissionIsCurrent || !visualContextAllowsCapture)) {
    // Never capture a visible-page image when the focused control signals sensitive data entry.
    visualAiOutcome = "unavailable";
  } else if (visualCaptureRequested && !visualBudgetReserved) {
    visualAiOutcome = "quota";
  } else if (visualCaptureRequested) {
    const targetIsCurrent = await visualCaptureTargetIsCurrent(
      tabId,
      windowId,
      context,
      capturedEpoch,
      captureSequence
    );
    if (!targetIsCurrent || !aiConfigurationEpochIsCurrent(aiConfigurationEpoch, capturedAiConfigurationEpoch)) {
      visualAiOutcome = "unavailable";
    } else {
      const controller = beginVisualRequest(tabId);
      try {
        const screenshot = await chrome.tabs.captureVisibleTab(windowId, { format: "jpeg", quality: 70 });
        if (
          !aiConfigurationEpochIsCurrent(aiConfigurationEpoch, capturedAiConfigurationEpoch)
          || !await visualCaptureTargetIsCurrent(tabId, windowId, context, capturedEpoch, captureSequence)
        ) {
          throw new Error("The active page changed before visual analysis could begin.");
        }
        const visualRequest = buildVisualRequest(context, screenshot, currentSettings.aiConfiguration);
        const visualRequestBody = serializeVisualRequest(visualRequest);
        visualCaptureTargets.set(tabId, {
          fingerprint: repeatDecision.fingerprint,
          capturedAt: Date.now()
        });
        visualAiAttempted = true;
        const visualGuide = await requestVisualGuide(
          context,
          visualRequestBody,
          currentSettings.aiConfiguration,
          apiKey,
          controller
        );
        if (!aiConfigurationEpochIsCurrent(aiConfigurationEpoch, capturedAiConfigurationEpoch)) {
          throw new Error("Visual AI configuration changed before the guide could be applied.");
        }
        guide = visualGuide;
        visualGuideApplied = true;
        visualEndpointCircuits.delete(currentSettings.aiConfiguration.endpoint);
        visualAiOutcome = "completed";
      } catch {
        // Visual analysis is optional. The local guide remains available when it fails.
        const configurationIsCurrent = aiConfigurationEpochIsCurrent(
          aiConfigurationEpoch,
          capturedAiConfigurationEpoch
        );
        if (visualAiAttempted && configurationIsCurrent && !controller.signal.aborted) {
          visualEndpointCircuits.set(
            currentSettings.aiConfiguration.endpoint,
            nextVisualCircuitFailure(visualEndpointCircuits.get(currentSettings.aiConfiguration.endpoint))
          );
        }
        visualAiOutcome = configurationIsCurrent && visualAiAttempted ? "fallback" : "unavailable";
      } finally {
        completeVisualRequest(tabId, controller);
      }
    }
  }

  if (
    (visualGuideApplied && !aiConfigurationEpochIsCurrent(aiConfigurationEpoch, capturedAiConfigurationEpoch))
    ||
    !captureEpochIsCurrent(navigationEpochs.get(tabId) ?? 0, capturedEpoch)
    || !captureSequenceIsCurrent(captureSequences.get(tabId), captureSequence)
    || !captureTargetsCurrentDocument(await pageInstanceForTab(tabId), context.pageInstanceId)
  ) return;
  await persistCaptureState(
    tabId,
    guide,
    visualAiAttempted,
    visualAiOutcome,
    visualGuideApplied ? capturedAiConfigurationEpoch : undefined
  );
  if (visualGuideApplied && !aiConfigurationEpochIsCurrent(aiConfigurationEpoch, capturedAiConfigurationEpoch)) {
    return;
  }
  const visualCaptureBudget = await visualCaptureBudgetSummaryForOrigin(originForUrl(context.pageUrl));
  const visualCircuit = visualAiCircuitSummary(await settings());
  void chrome.runtime.sendMessage({
    type: MESSAGE.GUIDE_UPDATED,
    tabId,
    guide,
    ...visualCaptureBudget,
    ...visualCircuit
  }).catch(() => undefined);
}

/** @param {CompanionSettings} currentSettings */
async function visualAiEndpointPermissionIsCurrent(currentSettings) {
  const endpoint = currentSettings.aiConfiguration.endpoint;
  if (!hasExactEndpointPermission(endpoint, currentSettings.aiEndpointPermissions)) return false;
  try {
    return await chrome.permissions.contains({ origins: [endpointPermissionPattern(endpoint)] });
  } catch {
    return false;
  }
}

/**
 * `captureVisibleTab` is window-scoped, so verify the original tab before and after image capture.
 * @param {number} tabId
 * @param {number} expectedWindowId
 * @param {import("./lib/guide-engine.js").BrowserContext} context
 * @param {number} capturedEpoch
 * @param {number} captureSequence
 */
async function visualCaptureTargetIsCurrent(tabId, expectedWindowId, context, capturedEpoch, captureSequence) {
  if (
    !captureEpochIsCurrent(navigationEpochs.get(tabId) ?? 0, capturedEpoch)
    || !captureSequenceIsCurrent(captureSequences.get(tabId), captureSequence)
  ) return false;
  try {
    const tab = await chrome.tabs.get(tabId);
    return captureTargetsActiveWindow(tab.active, tab.windowId, expectedWindowId)
      && captureTargetsCurrentPage(tab.url ?? "", context.pageUrl)
      && captureTargetsCurrentDocument(await pageInstanceForTab(tabId), context.pageInstanceId);
  } catch {
    return false;
  }
}

/** @param {number} tabId */
function captureAllowed(tabId) {
  const now = Date.now();
  const previous = captureWindows.get(tabId) ?? [];
  const recent = previous.filter((timestamp) => now - timestamp < 60_000);
  const lastCapture = recent.at(-1) ?? 0;
  if (now - lastCapture < CAPTURE_COOLDOWN_MS || recent.length >= MAX_CAPTURES_PER_MINUTE) return false;
  recent.push(now);
  captureWindows.set(tabId, recent);
  return true;
}

/** @param {unknown} tabId */
async function guideForTab(tabId) {
  if (typeof tabId !== "number") return null;
  const stored = await chrome.storage.session.get(`${GUIDE_KEY_PREFIX}${tabId}`);
  return stored[`${GUIDE_KEY_PREFIX}${tabId}`] ?? null;
}

/** @param {unknown} tabId */
async function activityForRequestedTab(tabId) {
  if (typeof tabId !== "number") return [];
  const stored = await chrome.storage.session.get(ACTIVITY_KEY);
  return activityForTab(stored[ACTIVITY_KEY], tabId);
}

async function captureSummary() {
  const stored = await chrome.storage.session.get(null);
  return captureStorageSummary(stored, normalizedActivity(stored[ACTIVITY_KEY]));
}

/** @param {unknown} tabId */
async function clearActivityForRequestedTab(tabId) {
  if (typeof tabId !== "number") return [];
  await queueSessionWrite(async () => {
    const stored = await chrome.storage.session.get(ACTIVITY_KEY);
    const remainingActivity = withoutActivityForTab(stored[ACTIVITY_KEY], tabId);
    await Promise.all([
      remainingActivity.length > 0
        ? chrome.storage.session.set({ [ACTIVITY_KEY]: remainingActivity })
        : chrome.storage.session.remove(ACTIVITY_KEY),
      chrome.storage.session.remove(`${GUIDE_KEY_PREFIX}${tabId}`)
    ]);
  });
  return [];
}

/** @param {unknown} tabId */
async function clearCaptureForRequestedTab(tabId) {
  if (typeof tabId !== "number") return [];
  invalidateTabCapture(tabId);
  return clearActivityForRequestedTab(tabId);
}

/** @param {number} tabId */
async function resetTabCaptureForNavigation(tabId) {
  await clearPageInstance(tabId);
  await clearTabCapture(tabId);
}

/** @param {number} tabId */
async function clearTabCapture(tabId) {
  await clearActivityForRequestedTab(tabId);
  void chrome.runtime.sendMessage({ type: MESSAGE.PAGE_CAPTURE_CLEARED, tabId }).catch(() => undefined);
}

async function clearAllCaptureData() {
  for (const tabId of new Set([...captureSequences.keys(), ...visualRequests.keys(), ...visualCaptureTargets.keys()])) {
    invalidateTabCapture(tabId);
  }
  await queueSessionWrite(async () => {
    const stored = await chrome.storage.session.get(null);
    await chrome.storage.session.remove(captureStorageKeys(stored));
  });
  return { guideCount: 0, activityCount: 0 };
}

/** @param {number} tabId @param {import("./lib/guide-engine.js").BrowserGuide} guide @param {boolean} visualAiAttempted @param {import("./lib/activity.js").VisualAiOutcome} visualAiOutcome @param {number | undefined} visualGuideConfigurationEpoch */
function persistCaptureState(tabId, guide, visualAiAttempted, visualAiOutcome, visualGuideConfigurationEpoch) {
  const activity = captureActivity({ tabId, guide, visualAiAttempted, visualAiOutcome });
  return queueSessionWrite(async () => {
    if (
      visualGuideConfigurationEpoch !== undefined
      && !aiConfigurationEpochIsCurrent(aiConfigurationEpoch, visualGuideConfigurationEpoch)
    ) return;
    const stored = await chrome.storage.session.get(ACTIVITY_KEY);
    if (
      visualGuideConfigurationEpoch !== undefined
      && !aiConfigurationEpochIsCurrent(aiConfigurationEpoch, visualGuideConfigurationEpoch)
    ) return;
    await chrome.storage.session.set({
      [`${GUIDE_KEY_PREFIX}${tabId}`]: guide,
      [ACTIVITY_KEY]: appendActivity(stored[ACTIVITY_KEY], activity)
    });
  });
}

/**
 * @template T
 * @param {() => Promise<T>} operation
 * @returns {Promise<T>}
 */
function queueSessionWrite(operation) {
  const write = sessionWrite.then(operation);
  sessionWrite = write.then(() => undefined, () => undefined);
  return write;
}

/** @param {string} origin */
async function reserveVisualCaptureBudgetForOrigin(origin) {
  if (!origin) return false;
  return queueSessionWrite(async () => {
    const stored = await chrome.storage.session.get(VISUAL_CAPTURE_BUDGETS_KEY);
    const budgets = normalizeVisualCaptureBudgets(stored[VISUAL_CAPTURE_BUDGETS_KEY]);
    const decision = visualCaptureBudgetDecision(budgets[origin]);
    if (!decision.allowed) return false;
    budgets[origin] = reserveVisualCaptureBudget(decision);
    await chrome.storage.session.set({
      [VISUAL_CAPTURE_BUDGETS_KEY]: normalizeVisualCaptureBudgets(budgets)
    });
    return true;
  });
}

/** @param {string} origin */
async function visualCaptureBudgetSummaryForOrigin(origin) {
  if (!origin) {
    return {
      visualCaptureBudgetRemaining: 0,
      visualCaptureBudgetLimit: 0,
      visualCaptureBudgetNextAvailableAt: null
    };
  }
  const stored = await chrome.storage.session.get(VISUAL_CAPTURE_BUDGETS_KEY);
  const budgets = normalizeVisualCaptureBudgets(stored[VISUAL_CAPTURE_BUDGETS_KEY]);
  const decision = visualCaptureBudgetDecision(budgets[origin]);
  return {
    visualCaptureBudgetRemaining: visualCaptureBudgetRemaining(decision),
    visualCaptureBudgetLimit: MAX_VISUAL_CAPTURES_PER_ORIGIN,
    visualCaptureBudgetNextAvailableAt: visualCaptureBudgetNextAvailableAt(decision)
  };
}

/** @param {CompanionSettings} currentSettings */
function visualAiCircuitSummary(currentSettings) {
  const configuration = currentSettings.aiConfiguration;
  return {
    visualAiCooldownRemainingMs: configuration.enabled
      ? visualCircuitCooldownRemaining(visualEndpointCircuits.get(configuration.endpoint))
      : 0
  };
}

/** @param {number} tabId */
function invalidateTabCapture(tabId) {
  abortVisualRequest(tabId);
  navigationEpochs.set(tabId, (navigationEpochs.get(tabId) ?? 0) + 1);
  captureSequences.set(tabId, (captureSequences.get(tabId) ?? 0) + 1);
  captureWindows.delete(tabId);
  visualCaptureTargets.delete(tabId);
}

function invalidateVisualAiWork() {
  aiConfigurationEpoch += 1;
  visualEndpointCircuits.clear();
  visualCaptureTargets.clear();
  for (const tabId of visualRequests.keys()) abortVisualRequest(tabId);
}

/** @param {number} tabId */
function beginCapture(tabId) {
  abortVisualRequest(tabId);
  const nextSequence = (captureSequences.get(tabId) ?? 0) + 1;
  captureSequences.set(tabId, nextSequence);
  return nextSequence;
}

/** @param {number} tabId */
function beginVisualRequest(tabId) {
  abortVisualRequest(tabId);
  const controller = new AbortController();
  visualRequests.set(tabId, controller);
  return controller;
}

/** @param {number} tabId @param {AbortController} controller */
function completeVisualRequest(tabId, controller) {
  if (visualRequests.get(tabId) === controller) visualRequests.delete(tabId);
}

/** @param {number} tabId */
function abortVisualRequest(tabId) {
  const controller = visualRequests.get(tabId);
  if (!controller) return;
  controller.abort();
  visualRequests.delete(tabId);
}

/** @param {number} tabId @param {string} pageInstanceId */
async function setPageInstance(tabId, pageInstanceId) {
  pageInstances.set(tabId, pageInstanceId);
  await queueSessionWrite(async () => {
    await chrome.storage.session.set({ [`${PAGE_INSTANCE_KEY_PREFIX}${tabId}`]: pageInstanceId });
  });
}

/** @param {number} tabId */
async function pageInstanceForTab(tabId) {
  const cached = pageInstances.get(tabId);
  if (cached) return cached;
  const key = `${PAGE_INSTANCE_KEY_PREFIX}${tabId}`;
  const stored = await chrome.storage.session.get(key);
  const value = stored[key];
  if (!isPageInstanceId(value)) return undefined;
  pageInstances.set(tabId, value);
  return value;
}

/** @param {number} tabId */
async function clearPageInstance(tabId) {
  pageInstances.delete(tabId);
  await queueSessionWrite(async () => {
    await chrome.storage.session.remove(`${PAGE_INSTANCE_KEY_PREFIX}${tabId}`);
  });
}

/** @param {unknown} value @returns {value is string} */
function isPageInstanceId(value) {
  return typeof value === "string" && /^[a-zA-Z0-9_-]{1,64}$/.test(value);
}

/**
 * @param {import("./lib/guide-engine.js").BrowserContext} context
 * @param {string} visualRequestBody
 * @param {AiConfiguration} configuration
 * @param {string} apiKey
 * @param {AbortController} controller
 */
async function requestVisualGuide(context, visualRequestBody, configuration, apiKey, controller) {
  const requestTimeout = createVisualRequestTimeout(controller.signal);
  try {
    const response = await fetch(configuration.endpoint, {
      method: "POST",
      headers: {
        "Content-Type": "application/json",
        Accept: "application/json",
        Authorization: `Bearer ${apiKey}`
      },
      body: visualRequestBody,
      signal: requestTimeout.signal,
      cache: "no-store",
      credentials: "omit",
      redirect: "error",
      referrerPolicy: "no-referrer"
    });
    if (!response.ok) throw new Error(`Visual AI request failed with status ${response.status}.`);
    const payload = await parseBoundedVisualResponse(response);
    return normalizeVisionGuide(parseVisualGuideContent(payload), context);
  } catch (error) {
    if (requestTimeout.didTimeout()) {
      throw new Error("Visual AI request timed out after 12 seconds.");
    }
    throw error;
  } finally {
    requestTimeout.dispose();
  }
}

/** @param {unknown} error */
function userMessage(error) {
  return error instanceof Error ? error.message : "CYMOS Browser Companion could not complete that request.";
}
