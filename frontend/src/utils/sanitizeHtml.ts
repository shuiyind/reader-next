const ALLOWED_TAGS = new Set([
  'a',
  'b',
  'blockquote',
  'br',
  'code',
  'div',
  'em',
  'h1',
  'h2',
  'h3',
  'h4',
  'h5',
  'h6',
  'i',
  'img',
  'li',
  'ol',
  'p',
  'pre',
  's',
  'small',
  'span',
  'strong',
  'sub',
  'sup',
  'table',
  'tbody',
  'td',
  'th',
  'thead',
  'tr',
  'u',
  'ul',
])

const DROP_WITH_CONTENT = new Set([
  'base',
  'button',
  'embed',
  'form',
  'iframe',
  'input',
  'link',
  'math',
  'meta',
  'noscript',
  'object',
  'script',
  'select',
  'style',
  'svg',
  'template',
  'textarea',
])

const ALLOWED_ATTRIBUTES = new Set([
  'alt',
  'class',
  'colspan',
  'height',
  'href',
  'loading',
  'rel',
  'rowspan',
  'src',
  'start',
  'title',
  'width',
])

export function sanitizeReaderHtml(raw: string): string {
  if (!raw) return ''
  const document = new DOMParser().parseFromString(`<body>${raw}</body>`, 'text/html')
  return serializeChildren(document.body)
}

function serializeChildren(node: Node): string {
  let output = ''
  for (const child of Array.from(node.childNodes)) {
    if (child.nodeType === Node.TEXT_NODE) {
      output += escapeHtml(child.textContent || '')
      continue
    }
    if (child.nodeType !== Node.ELEMENT_NODE) continue
    const element = child as Element
    const tag = element.tagName.toLowerCase()
    if (DROP_WITH_CONTENT.has(tag)) continue
    if (!ALLOWED_TAGS.has(tag)) {
      output += serializeChildren(element)
      continue
    }

    const attributes: string[] = []
    for (const attribute of Array.from(element.attributes)) {
      const name = attribute.name.toLowerCase()
      if (!ALLOWED_ATTRIBUTES.has(name)) continue
      const value = attribute.value.trim()
      if (!value) continue
      if (name === 'href' && !isSafeUrl(value)) continue
      if (name === 'src' && !isSafeUrl(value)) continue
      attributes.push(`${name}="${escapeAttribute(value)}"`)
    }

    if (tag === 'a') {
      attributes.push('rel="noopener noreferrer"')
    }
    if (tag === 'img') {
      attributes.push('loading="lazy"')
      attributes.push('referrerpolicy="no-referrer"')
    }

    const attrs = attributes.length ? ` ${attributes.join(' ')}` : ''
    if (tag === 'br') {
      output += `<br${attrs}>`
    } else {
      output += `<${tag}${attrs}>${serializeChildren(element)}</${tag}>`
    }
  }
  return output
}

function isSafeUrl(value: string): boolean {
  if (value.startsWith('#')) return true
  if (/^(?:\/(?!\/)|\.{1,2}\/)/.test(value)) return true
  try {
    const url = new URL(value, window.location.href)
    return url.protocol === 'http:' || url.protocol === 'https:'
  } catch {
    return false
  }
}

function escapeHtml(value: string): string {
  return value
    .replaceAll('&', '&amp;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;')
    .replaceAll('"', '&quot;')
    .replaceAll("'", '&#39;')
}

function escapeAttribute(value: string): string {
  return escapeHtml(value).replaceAll('`', '&#96;')
}
