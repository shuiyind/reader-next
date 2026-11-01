import { describe, expect, it } from 'vitest'
import { sanitizeReaderHtml } from './sanitizeHtml'

describe('sanitizeReaderHtml', () => {
  it('removes executable markup and event handlers', () => {
    const result = sanitizeReaderHtml(
      '<p onclick="alert(1)">safe<img src=x onerror="alert(2)"></p><script>alert(3)</script>',
    )
    expect(result).not.toContain('onclick')
    expect(result).not.toContain('onerror')
    expect(result).not.toContain('<script')
    expect(result).toContain('safe')
  })

  it('rejects dangerous URL schemes', () => {
    const result = sanitizeReaderHtml('<a href="javascript:alert(1)">x</a><img src="data:text/html,x">')
    expect(result).not.toContain('javascript:')
    expect(result).not.toContain('data:text/html')
  })

  it('keeps a conservative reading subset', () => {
    const result = sanitizeReaderHtml('<p>one <strong>two</strong><iframe src="https://evil"></iframe></p>')
    expect(result).toBe('<p>one <strong>two</strong></p>')
  })
})
