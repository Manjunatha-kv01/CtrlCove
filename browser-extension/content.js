// @ts-check

(() => {
  if (document.documentElement.dataset.cymosCompanionLoaded === "true") return;
  document.documentElement.dataset.cymosCompanionLoaded = "true";

  const pageInstanceId = crypto.randomUUID();
  let siteEnabled = false;
  let pauseTimer = 0;
  /** @type {{ x: number, y: number } | null} */
  let lastPointer = null;

  chrome.runtime.onMessage.addListener((message) => {
    if (message?.type !== "cymos.site.configuration") return;
    siteEnabled = message.configuration?.siteEnabled === true;
    if (!siteEnabled) window.clearTimeout(pauseTimer);
  });

  document.addEventListener("pointermove", (event) => {
    if (!siteEnabled || event.pointerType === "touch") return;
    lastPointer = { x: event.clientX, y: event.clientY };
    window.clearTimeout(pauseTimer);
    pauseTimer = window.setTimeout(() => emitPauseContext(), 800);
  }, { passive: true });

  void chrome.runtime.sendMessage({ type: "cymos.content.ready", pageInstanceId })
    .then((configuration) => {
      siteEnabled = configuration?.siteEnabled === true;
    })
    .catch(() => undefined);

  function emitPauseContext() {
    if (!siteEnabled || !lastPointer) return;
    const element = document.elementFromPoint(lastPointer.x, lastPointer.y);
    if (!(element instanceof HTMLElement)) return;
    void chrome.runtime.sendMessage({
      type: "cymos.cursor.paused",
      context: {
        pageUrl: safePageUrl(window.location.href),
        pageTitle: redactText(document.title),
        pageInstanceId,
        pointer: {
          x: lastPointer.x,
          y: lastPointer.y,
          viewportWidth: window.innerWidth,
          viewportHeight: window.innerHeight
        },
        element: describeElement(element),
        capturedAt: new Date().toISOString()
      }
    }).catch(() => undefined);
  }

  /** @param {HTMLElement} element */
  function describeElement(element) {
    const nearbyControl = element.closest("button, a, input, textarea, select, [contenteditable='true'], [role], label");
    const target = nearbyControl instanceof HTMLElement ? nearbyControl : element;
    const bounds = target.getBoundingClientRect();
    return {
      tagName: target.tagName,
      role: target.getAttribute("role") ?? "",
      label: redactText(accessibleLabel(target)),
      text: redactText(target.innerText || target.textContent || ""),
      inputType: target instanceof HTMLInputElement ? target.type : "",
      autocomplete: target.getAttribute("autocomplete") ?? "",
      bounds: {
        left: bounds.left,
        top: bounds.top,
        width: bounds.width,
        height: bounds.height
      }
    };
  }

  /** @param {HTMLElement} element */
  function accessibleLabel(element) {
    const ariaLabel = element.getAttribute("aria-label");
    if (ariaLabel) return ariaLabel;
    const labelledBy = element.getAttribute("aria-labelledby");
    if (labelledBy) {
      return labelledBy.split(/\s+/).map((id) => document.getElementById(id)?.textContent ?? "").join(" ");
    }
    if (element instanceof HTMLInputElement || element instanceof HTMLTextAreaElement || element instanceof HTMLSelectElement) {
      const label = element.labels?.item(0)?.textContent;
      if (label) return label;
      const placeholder = element instanceof HTMLSelectElement ? "" : element.placeholder;
      return placeholder || element.name || element.id;
    }
    return element.title || "";
  }

  /** @param {string} value */
  function redactText(value) {
    let redacted = value
      .replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F\u007F-\u009F\u202A-\u202E\u2066-\u2069]/g, "")
      .replace(/\s+/g, " ")
      .trim();
    const patterns = [
      /-----BEGIN(?: [A-Z]+)? PRIVATE KEY-----[\s\S]*?-----END(?: [A-Z]+)? PRIVATE KEY-----/gi,
      /\b(?:eyJ[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,}\.[a-zA-Z0-9_-]{10,})\b/g,
      /\b(?:sk|pk|ghp|gho|github_pat)_[a-zA-Z0-9_-]{12,}\b/gi,
      /\b(?:api[_-]?key|access[_-]?token|auth(?:orization)?|bearer|password|secret|token)\s*[:=]\s*[^\s,;]+/gi
    ];
    for (const pattern of patterns) redacted = redacted.replace(pattern, "[REDACTED]");
    return redacted.length > 360 ? `${redacted.slice(0, 357)}...` : redacted;
  }

  /** @param {string} value */
  function safePageUrl(value) {
    try {
      const url = new URL(value);
      return `${url.origin}${url.pathname}`;
    } catch {
      return "";
    }
  }
})();
