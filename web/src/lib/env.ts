export function getApiBaseUrl() {
  return import.meta.env.VITE_API_BASE_URL?.trim() || "/api";
}

export function getHealthApiUrl() {
  return `${getApiBaseUrl()}/health`;
}

export function getAssistantChatApiUrl() {
  return `${getApiBaseUrl()}/assistant/chat`;
}

export function getAssistantModelsApiUrl() {
  return `${getApiBaseUrl()}/assistant/models`;
}

export function getAssistantSelectedModelApiUrl() {
  return `${getApiBaseUrl()}/assistant/models/selected`;
}

export function getAssistantReasoningEffortApiUrl() {
  return `${getApiBaseUrl()}/assistant/models/reasoning-effort`;
}

export function getAssistantSystemPromptApiUrl() {
  return `${getApiBaseUrl()}/assistant/system-prompt`;
}

export function getAssistantThreadsApiUrl() {
  return `${getApiBaseUrl()}/assistant/threads`;
}

export function getAssistantThreadApiUrl(threadId: string) {
  return `${getApiBaseUrl()}/assistant/threads/${encodeURIComponent(threadId)}`;
}

export function getAssistantThreadMessagesApiUrl(threadId: string) {
  return `${getApiBaseUrl()}/assistant/threads/${encodeURIComponent(threadId)}/messages`;
}
