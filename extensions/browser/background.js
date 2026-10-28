const CYMOS_API_URL = "http://localhost:11435/api/v1/memories";

chrome.runtime.onInstalled.addListener(() => {
  chrome.contextMenus.create({
    id: "save-to-cymos",
    title: "Save to CYMOS",
    contexts: ["selection", "page"]
  });
});

chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  if (info.menuItemId === "save-to-cymos") {
    let contentToSave = info.selectionText;
    let type = "Text";

    if (!contentToSave) {
      // If no text selected, save the page URL and title
      contentToSave = `[${tab.title}](${tab.url})`;
      type = "URL";
    }

    try {
      const response = await fetch(CYMOS_API_URL, {
        method: "POST",
        headers: {
          "Content-Type": "application/json"
        },
        body: JSON.stringify({
          content: contentToSave,
          source_app: "Chrome Extension"
        })
      });

      if (!response.ok) {
        throw new Error("Failed to save to CYMOS");
      }

      console.log("Successfully saved to CYMOS");
      // Optional: Show a notification to the user
      chrome.notifications.create({
        type: "basic",
        iconUrl: "icons/icon48.png",
        title: "CYMOS",
        message: "Memory captured successfully."
      });
    } catch (error) {
      console.error("Error capturing to CYMOS:", error);
      chrome.notifications.create({
        type: "basic",
        iconUrl: "icons/icon48.png",
        title: "CYMOS Error",
        message: "Failed to connect to local CYMOS API. Is the app running?"
      });
    }
  }
});
