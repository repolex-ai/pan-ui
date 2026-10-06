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

/** copia, pan, … : the namespace's last path segment. */
export function prefixOf(iri: string): string {
  const m = iri.match(/\/([^/#]+)[/#][^/#]+$/)
  return m ? m[1] : ''
}
