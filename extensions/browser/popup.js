const CYMOS_API_HEALTH = "http://localhost:11435/api/v1/health";
const CYMOS_API_MEMORIES = "http://localhost:11435/api/v1/memories";

document.addEventListener("DOMContentLoaded", async () => {
  const statusIndicator = document.getElementById("status-indicator");
  const captureBtn = document.getElementById("capture-page");
  const messageEl = document.getElementById("message");

  // Check connection to CYMOS
  try {
    const res = await fetch(CYMOS_API_HEALTH);
    if (res.ok) {
      statusIndicator.className = "status online";
      statusIndicator.title = "Connected to CYMOS";
    } else {
      throw new Error("Bad status");
    }
  } catch (e) {
    statusIndicator.className = "status offline";
    statusIndicator.title = "CYMOS is offline";
    captureBtn.disabled = true;
    messageEl.textContent = "Start the CYMOS desktop app to connect.";
    return;
  }

  // Handle Capture Page click
  captureBtn.addEventListener("click", async () => {
    captureBtn.disabled = true;
    captureBtn.textContent = "Saving...";
    messageEl.textContent = "";

    try {
      // Get current active tab
      const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
      if (!tab) throw new Error("No active tab found");

      const content = `[${tab.title}](${tab.url})`;

      const res = await fetch(CYMOS_API_MEMORIES, {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          content: content,
          source_app: "Chrome Extension"
        })
      });

      if (!res.ok) throw new Error("Failed to save");

      messageEl.style.color = "var(--success)";
      messageEl.textContent = "Saved to CYMOS!";
      setTimeout(() => window.close(), 1500);

    } catch (e) {
      console.error(e);
      messageEl.style.color = "var(--danger)";
      messageEl.textContent = "Failed to save.";
      captureBtn.disabled = false;
      captureBtn.textContent = "Capture Page";
    }
  });
});
