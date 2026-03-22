type ApiErrorResponse = {
  error: string
  message: string
}

export function getApiBaseUrl() {
  return import.meta.env.VITE_API_BASE_URL?.trim() || 'http://127.0.0.1:3000'
}

export function getHealthApiUrl() {
  return new URL('/health', getApiBaseUrl()).toString()
}

export function getAccountsApiUrl() {
  return new URL('/accounts', getApiBaseUrl()).toString()
}

export function getAccountDetailApiUrl(accountId: string) {
  return new URL(`/accounts/${accountId}`, getApiBaseUrl()).toString()
}

export function getAccountBalanceApiUrl(accountId: string, currency: string) {
  return new URL(
    `/accounts/${accountId}/balances/${currency}`,
    getApiBaseUrl()
  ).toString()
}

export async function readApiErrorMessage(
  response: Response,
  fallbackMessage: string
) {
  try {
    const data = (await response.json()) as ApiErrorResponse
    return data.message || fallbackMessage
  } catch {
    return fallbackMessage
  }
}
