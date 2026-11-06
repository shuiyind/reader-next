import { describe, expect, it } from 'vitest'
import {
  appendAuthQueryParams,
  buildAuthHeaderValues,
  computeNeedSecureKey,
  SECURE_KEY_STORAGE_KEY,
  setActiveSecureKey,
} from './secureAccess'

describe('secureAccess', () => {
  it('ignores legacy session secrets stored in localStorage', () => {
    localStorage.setItem('accessToken', 'token-123')
    localStorage.setItem(SECURE_KEY_STORAGE_KEY, 'secure-456')

    expect(buildAuthHeaderValues()).toEqual({
      accessToken: undefined,
      secureKey: undefined,
    })
  })

  it('uses only the in-memory secure key for admin requests', () => {
    setActiveSecureKey('secure-456')

    expect(buildAuthHeaderValues()).toEqual({
      accessToken: undefined,
      secureKey: 'secure-456',
    })

    setActiveSecureKey('')
  })

  it('never adds auth values to event source query params', () => {
    setActiveSecureKey('admin-key')
    const params = new URLSearchParams()

    appendAuthQueryParams(params)

    expect(params.toString()).toBe('')
    setActiveSecureKey('')
  })

  it('requires secure key only when the server requires it and current request is not admin authorized', () => {
    expect(computeNeedSecureKey({
      secure: true,
      secureKeyRequired: true,
      adminAuthorized: false,
    })).toBe(true)

    expect(computeNeedSecureKey({
      secure: true,
      secureKeyRequired: true,
      adminAuthorized: true,
    })).toBe(false)

    expect(computeNeedSecureKey({
      secure: true,
      secureKeyRequired: false,
      adminAuthorized: false,
    })).toBe(false)
  })
})
