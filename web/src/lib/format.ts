const pad = (n: number) => String(n).padStart(2, '0')

/** 2026-09-06 05:57, in local time. */
export function when(t: number | null | undefined): string {
  if (t == null) return '—'
  const d = new Date(t * 1000)
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())} ${pad(d.getHours())}:${pad(d.getMinutes())}`
}

export function day(t: number | null | undefined): string {
  if (t == null) return '—'
  const d = new Date(t * 1000)
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}`
}

export const count = (n: number | null | undefined) => (n == null ? '—' : n.toLocaleString('en-US'))

export const stars = (n: number) => '★'.repeat(n)

/** The local name of a predicate IRI: …/ontology/copia/genModel → genModel. */
export function localName(iri: string): string {
  const m = iri.match(/[#/]([^#/]+)$/)
  return m ? m[1] : iri
}

/** The namespace of a predicate IRI: everything before its local name. */
export function namespaceOf(iri: string): string {
  const m = iri.match(/^(.*[#/])[^#/]+$/)
  return m ? m[1] : ''
}

// The usual short names for vocabularies that image files carry, whose
// addresses do not end in a usable word.
const KNOWN: Record<string, string> = {
  'http://www.w3.org/1999/02/22-rdf-syntax-ns#': 'rdf',
  'http://www.w3.org/2000/01/rdf-schema#': 'rdfs',
  'http://purl.org/dc/elements/1.1/': 'dc',
  'http://purl.org/dc/terms/': 'dcterms',
  'http://ns.adobe.com/xap/1.0/': 'xmp',
  'http://ns.adobe.com/exif/1.0/': 'exif',
  'http://ns.adobe.com/tiff/1.0/': 'tiff',
}

/**
 * A short name for a predicate's vocabulary: copia, pan, photoshop, mflux.
 * The last word of the namespace, skipping version numbers like 1.0; if
 * nothing but versions is left, the site's name.
 */
export function prefixOf(iri: string): string {
  const ns = namespaceOf(iri)
  if (KNOWN[ns]) return KNOWN[ns]
  const m = ns.match(/^[a-z]+:\/\/([^/]+)\/?(.*)$/i)
  if (!m) return ns
  const word = m[2].split(/[/#]/).filter((s) => s && !/^v?\d[\d.]*$/.test(s)).at(-1)
  if (word) return word
  const host = m[1].split('.').filter((s) => !['www', 'ns'].includes(s))
  return host.length > 1 ? host.at(-2)! : (host[0] ?? ns)
}
