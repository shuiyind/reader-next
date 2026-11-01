export const SECURE_KEY_STORAGE_KEY = 'secureKey'

let activeSecureKey = ''

export function setActiveSecureKey(value: string) {
  activeSecureKey = value.trim()
}

type StorageLike = Pick<Storage, 'getItem'>

export function buildAuthHeaderValues(storage: StorageLike) {
  const accessToken = storage.getItem('accessToken')?.trim() || ''
  const secureKey = activeSecureKey || storage.getItem(SECURE_KEY_STORAGE_KEY)?.trim() || ''

  return {
    accessToken: accessToken || undefined,
    secureKey: secureKey || undefined,
  }
}

export function appendAuthQueryParams(_params: URLSearchParams, _storage: StorageLike = localStorage) {
  // Session credentials are sent by the HttpOnly cookie. Never put secrets in URLs.
}

export function computeNeedSecureKey(params: {
  secure: boolean
  secureKeyRequired: boolean
  adminAuthorized: boolean
}) {
  return params.secure && params.secureKeyRequired && !params.adminAuthorized
}

export function readStoredSecureKey(storage: StorageLike = localStorage) {
  return activeSecureKey || storage.getItem(SECURE_KEY_STORAGE_KEY)?.trim() || ''
}
