// @ts-check

import { safePageUrl } from "./redaction.js";

/** @param {string} tabUrl @param {string} contextPageUrl */
export function captureTargetsCurrentPage(tabUrl, contextPageUrl) {
  const currentPage = safePageUrl(tabUrl);
  return Boolean(currentPage) && currentPage === safePageUrl(contextPageUrl);
}

/** @param {number} activeEpoch @param {number} capturedEpoch */
export function captureEpochIsCurrent(activeEpoch, capturedEpoch) {
  return activeEpoch === capturedEpoch;
}

/** @param {number | undefined} activeSequence @param {number} capturedSequence */
export function captureSequenceIsCurrent(activeSequence, capturedSequence) {
  return activeSequence === capturedSequence;
}

/** @param {string | undefined} activePageInstanceId @param {string} capturedPageInstanceId */
export function captureTargetsCurrentDocument(activePageInstanceId, capturedPageInstanceId) {
  return Boolean(activePageInstanceId) && activePageInstanceId === capturedPageInstanceId;
}

/** @param {boolean | undefined} isActive @param {number | undefined} currentWindowId @param {number} expectedWindowId */
export function captureTargetsActiveWindow(isActive, currentWindowId, expectedWindowId) {
  return isActive === true && currentWindowId === expectedWindowId;
}

/** @param {number} activeEpoch @param {number} capturedEpoch */
export function aiConfigurationEpochIsCurrent(activeEpoch, capturedEpoch) {
  return activeEpoch === capturedEpoch;
}
