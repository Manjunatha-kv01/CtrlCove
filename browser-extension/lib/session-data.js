// @ts-check

export const GUIDE_KEY_PREFIX = "cymos.browser.guide.";
export const ACTIVITY_KEY = "cymos.browser.capture-activity";
export const PAGE_INSTANCE_KEY_PREFIX = "cymos.browser.page-instance.";
export const VISUAL_CAPTURE_BUDGETS_KEY = "cymos.browser.visual-capture-budgets";

/**
 * @param {unknown} stored
 * @returns {string[]}
 */
export function captureStorageKeys(stored) {
  if (!stored || typeof stored !== "object" || Array.isArray(stored)) {
    return [ACTIVITY_KEY, VISUAL_CAPTURE_BUDGETS_KEY];
  }
  return [
    ACTIVITY_KEY,
    VISUAL_CAPTURE_BUDGETS_KEY,
    ...Object.keys(stored).filter((key) => key.startsWith(GUIDE_KEY_PREFIX))
  ];
}

/**
 * @param {unknown} stored
 * @param {unknown} activityRecords
 */
export function captureStorageSummary(stored, activityRecords) {
  const guideCount = captureStorageKeys(stored).filter((key) => key.startsWith(GUIDE_KEY_PREFIX)).length;
  const storageRecord = stored && typeof stored === "object" && !Array.isArray(stored)
    ? /** @type {Record<string, unknown>} */ (stored)
    : null;
  const budgetRecord = storageRecord
    ? storageRecord[VISUAL_CAPTURE_BUDGETS_KEY]
    : undefined;
  const budgetOriginCount = budgetRecord && typeof budgetRecord === "object" && !Array.isArray(budgetRecord)
    ? Object.keys(budgetRecord).length
    : 0;
  return {
    guideCount,
    activityCount: Array.isArray(activityRecords) ? activityRecords.length : 0,
    budgetOriginCount
  };
}
