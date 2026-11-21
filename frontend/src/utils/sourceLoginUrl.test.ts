import { describe, expect, it } from 'vitest'
import { buildSourceLoginProxyUrl, normalizeSourceLoginUrl } from './sourceLoginUrl'

describe('normalizeSourceLoginUrl', () => {
  it('opens an absolute registration URL even when bookSourceUrl is a display name', () => {
    expect(
      normalizeSourceLoginUrl(
        'https://v2.gyks.cf/register',
        '光遇聚合',
        'https://reader.example/source',
      ),
    ).toBe('https://v2.gyks.cf/register')
  })

  it('resolves a relative URL against a valid source URL', () => {
    expect(
      normalizeSourceLoginUrl('/register', 'https://v2.gyks.cf', 'https://reader.example/source'),
    ).toBe('https://v2.gyks.cf/register')
  })
})

describe('buildSourceLoginProxyUrl', () => {
  it('wraps public HTTP login pages behind the authenticated book-source proxy', () => {
    const value = buildSourceLoginProxyUrl(
      'https://source.example',
      'https://login.example/form?next=/reader',
    )
    const params = new URL(value, 'https://reader.example').searchParams

    expect(value.startsWith('/reader3/bookSourceProxy?')).toBe(true)
    expect(params.get('bookSourceUrl')).toBe('https://source.example')
    expect(params.get('url')).toBe('https://login.example/form?next=/reader')
  })

  it('rejects non-HTTP preview documents instead of executing them inline', () => {
    expect(() =>
      buildSourceLoginProxyUrl(
        'https://source.example',
        'data:text/html,<script>top.location="https://evil.example"</script>',
      ),
    ).toThrow()
  })
})
