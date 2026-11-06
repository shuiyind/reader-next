export const SECURE_KEY_STORAGE_KEY = 'secureKey'

let activeSecureKey = ''

export function setActiveSecureKey(value: string) {
  activeSecureKey = value.trim()
}

export function buildAuthHeaderValues() {
  return {
    accessToken: undefined,
    secureKey: activeSecureKey || undefined,
  }
}

export function appendAuthQueryParams(_params: URLSearchParams) {
  // Session credentials are sent by the HttpOnly cookie. Never put secrets in URLs.
}

export function computeNeedSecureKey(params: {
  secure: boolean
  secureKeyRequired: boolean
  adminAuthorized: boolean
}) {
  return params.secure && params.secureKeyRequired && !params.adminAuthorized
}

export function readStoredSecureKey() {
  return activeSecureKey
}
