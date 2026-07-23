// @ts-check

/** @param {unknown} value */
export function normalizeVisualAiEndpoint(value) {
  if (typeof value !== "string" || !value.trim()) {
    throw new Error("Enter a valid HTTPS endpoint or a local development endpoint.");
  }
  let url;
  try {
    url = new URL(value.trim());
  } catch {
    throw new Error("Enter a valid HTTPS endpoint or a local development endpoint.");
  }
  const localHost = url.hostname === "localhost" || url.hostname === "127.0.0.1";
  if (url.protocol !== "https:" && !(url.protocol === "http:" && localHost)) {
    throw new Error("Visual AI endpoints must use HTTPS, except localhost development endpoints.");
  }
  if (url.username || url.password || url.search || url.hash) {
    throw new Error("Visual AI endpoints cannot contain credentials, query parameters, or fragments.");
  }
  return url.toString();
}

/** @param {unknown} endpoint */
export function endpointPermissionPattern(endpoint) {
  const url = new URL(normalizeVisualAiEndpoint(endpoint));
  return `${url.protocol}//${url.host}/*`;
}

/** @param {unknown} endpoint @param {unknown} permissions */
export function hasExactEndpointPermission(endpoint, permissions) {
  try {
    const expectedPermission = endpointPermissionPattern(endpoint);
    return Array.isArray(permissions) && permissions.includes(expectedPermission);
  } catch {
    return false;
  }
}

/** @param {unknown} permissions @param {string} fallbackPermission */
export function managedEndpointPermissions(permissions, fallbackPermission) {
  const candidates = [
    ...(Array.isArray(permissions) ? permissions : []),
    fallbackPermission
  ];
  return [...new Set(candidates.filter(isEndpointPermission))].sort();
}

/** @param {string[]} permissions @param {string} retainedPermission */
export function endpointPermissionsToRevoke(permissions, retainedPermission) {
  return permissions.filter((permission) => permission !== retainedPermission);
}

/** @param {unknown} value */
function isEndpointPermission(value) {
  if (typeof value !== "string") return false;
  try {
    const url = new URL(value);
    const localHost = url.hostname === "localhost" || url.hostname === "127.0.0.1";
    return url.pathname === "/*"
      && !url.hostname.includes("*")
      && (url.protocol === "https:" || (url.protocol === "http:" && localHost));
  } catch {
    return false;
  }
}
