export function getApiBaseUrl() {
  return import.meta.env.VITE_API_BASE_URL?.trim() || "/api";
}

export function getHealthApiUrl() {
  return `${getApiBaseUrl()}/health`;
}

export function getVersionApiUrl() {
  return `${getApiBaseUrl()}/version`;
}

declare const __APP_VERSION__: string;
export const APP_VERSION: string = __APP_VERSION__;

export function getConfigApiUrl() {
  return `${getApiBaseUrl()}/config`;
}
