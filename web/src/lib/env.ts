export function getApiBaseUrl() {
  return import.meta.env.VITE_API_BASE_URL?.trim() || "http://127.0.0.1:3000";
}

export function getHealthApiUrl() {
  return new URL("/health", getApiBaseUrl()).toString();
}

export function getAssistantChatApiUrl() {
  return new URL("/assistant/chat", getApiBaseUrl()).toString();
}

export function getAssistantModelsApiUrl() {
  return new URL("/assistant/models", getApiBaseUrl()).toString();
}

export function getAssistantSelectedModelApiUrl() {
  return new URL("/assistant/models/selected", getApiBaseUrl()).toString();
}

export function getAssistantThreadsApiUrl() {
  return new URL("/assistant/threads", getApiBaseUrl()).toString();
}

export function getAssistantThreadApiUrl(threadId: string) {
  return new URL(`/assistant/threads/${encodeURIComponent(threadId)}`, getApiBaseUrl()).toString();
}

export function getAssistantThreadMessagesApiUrl(threadId: string) {
  return new URL(`/assistant/threads/${encodeURIComponent(threadId)}/messages`, getApiBaseUrl()).toString();
}
