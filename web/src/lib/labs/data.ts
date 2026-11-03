// What the Lab views read: the precomputed files for a store (built by
// tools/lab_build.py) and the Lab's routes on the server. Files are fetched
// once per store and kept, since a view switch should not refetch 20 MB.

export interface LabStatus {
  store: string
  ready: boolean
  n: number | null
  dim: number | null
  built: string | null
  atlas: boolean | null
  files: string[]
  /** How many embeddings pand has now; more than n means the Lab is behind. */
  embeddings_now: number | null
  rebuild: string
}

export interface Meta {
  n: number
  dim: number
  k: number
  built: string
  ids: string[]
  t: number[]
}

export interface Hit {
  id: string
  sim: number
}

async function json<T>(url: string, init?: RequestInit): Promise<T> {
  const r = await fetch(url, init)
  if (!r.ok) throw new Error(await r.text())
  return r.json()
}

async function bytes(url: string): Promise<ArrayBuffer> {
  const r = await fetch(url)
  if (!r.ok) throw new Error(await r.text())
  return r.arrayBuffer()
}

const cache = new Map<string, Promise<unknown>>()
function once<T>(key: string, f: () => Promise<T>): Promise<T> {
  let p = cache.get(key) as Promise<T> | undefined
  if (!p) {
    p = f()
    cache.set(key, p)
    p.catch(() => cache.delete(key))
  }
  return p
}

const file = (store: string, name: string) => `/api/lab/${store}/file/${name}`

export const status = (store: string) => json<LabStatus>(`/api/lab/${store}/status`)

export const meta = (store: string) =>
  once(`${store}/meta`, async () => {
    const m = await json<Meta>(file(store, 'meta.json'))
    const pos = new Map<string, number>()
    m.ids.forEach((id, i) => pos.set(id, i))
    return { ...m, pos }
  })

export const atlas = (store: string) => once(`${store}/atlas`, async () => new Float32Array(await bytes(file(store, 'atlas.f32'))))

export const neighbours = (store: string) =>
  once(`${store}/nbr`, async () => {
    const [i, s] = await Promise.all([bytes(file(store, 'nbr.u32')), bytes(file(store, 'nbr.f32'))])
    return { idx: new Uint32Array(i), sim: new Float32Array(s) }
  })

export interface Echo {
  a: number
  b: number
  sim: number
}
export const echoes = (store: string) => once(`${store}/echoes`, () => json<{ gap_days: number; pairs: Echo[] }>(file(store, 'echoes.json')))

export const poses = (store: string) =>
  once(`${store}/poses`, async () => {
    const [raw, which] = await Promise.all([
      bytes(file(store, 'pose.f32')),
      json<{ ids: string[]; people: number[] }>(file(store, 'pose.json')),
    ])
    return { xyc: new Float32Array(raw), ids: which.ids, people: which.people }
  })

export interface NearOpts {
  plus: string[]
  plus_w?: number[]
  minus?: string[]
  skip?: string[]
  k?: number
  scope?: 'all' | 'inside' | 'outside'
  jitter?: number
}
export const near = (store: string, o: NearOpts) =>
  json<{ hits: Hit[]; of: number }>(`/api/lab/${store}/near`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify(o),
  })

export interface Step {
  dx: number
  dy: number
  id: string
  sim: number
}
export const stick = (store: string, id: string, stride: number, fuzz: number, skip: string[]) =>
  json<{ centre: string; dirs: Step[] }>(
    `/api/lab/${store}/stick/${id}?stride=${stride}&fuzz=${fuzz}&skip=${encodeURIComponent(skip.slice(-200).join(','))}`,
  )

export const words = (store: string, q: string, k = 60) =>
  json<{ ids: string[]; captions: string[]; total: number }>(`/api/lab/${store}/words?q=${encodeURIComponent(q)}&k=${k}`)

export const hybrid = (store: string, q: string, k = 60) =>
  json<{ said: string[]; looks: Hit[]; total: number; seeds: number }>(`/api/lab/${store}/hybrid`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ q, k }),
  })
