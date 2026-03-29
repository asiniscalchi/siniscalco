export function getApiBaseUrl() {
  return import.meta.env.VITE_API_BASE_URL?.trim() || "http://127.0.0.1:3000";
}

export function getHealthApiUrl() {
  return new URL("/health", getApiBaseUrl()).toString();
}

export function getAssistantChatApiUrl() {
  return new URL("/assistant/chat", getApiBaseUrl()).toString();
}
