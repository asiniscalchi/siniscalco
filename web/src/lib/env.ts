export const API_BASE_URL = "/api";

export function getApiBaseUrl() {
  return API_BASE_URL;
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
