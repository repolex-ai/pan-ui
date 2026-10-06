// Everything the page asks for, and the shapes it comes back in. One place,
// so a field the server renames breaks here, loudly, and not in a corner of
// a component.

export interface Store {
  id: string
  name: string
  root: string
  is_default: boolean
  /** null when pand's health could not be read: not the same as zero. */
  images: number | null
  captions: number | null
  embeddings: number | null
}

export interface SetInfo {
  id: string
  count: number
  /** Unix seconds of the set's oldest and newest image. */
  first: number
  last: number
}

export interface IndexState {
  store: string
  ready: boolean
  building: boolean
  error: string | null
  built?: string
  count?: number
  undated?: number
  first?: number | null
  last?: number | null
  sets?: SetInfo[]
}

/** Columns, in time order. Index i is one image across all four. */
export interface Images {
  store: string
  set: string | null
  built: string
  ids: string[]
  t: number[]
  rating: number[]
  /** 1 picked, 0 none, -1 rejected. */
  flag: number[]
}

export interface QueueStatus {
  pending: number
  saved: number
  last_error: string | null
}

export interface Health {
  pand: { ok: boolean; version: string; uptime_secs: number } | null
  pand_url: string
  error?: string
  queue: QueueStatus
}

export interface Facts {
  id: string
  store: string
  facts: Record<string, string[]>
}

async function json<T>(url: string, init?: RequestInit): Promise<T> {
  const r = await fetch(url, init)
  if (!r.ok) throw new Error(await r.text())
  return r.json()
}

export const getHealth = () => json<Health>('/api/health')
export const getStores = () => json<Store[]>('/api/stores')
export const getIndex = (store: string) => json<IndexState>(`/api/stores/${store}/index`)
export const getImages = (store: string, set: string | null) =>
  json<Images>(`/api/stores/${store}/images${set ? `?set=${encodeURIComponent(set)}` : ''}`)
export const getFacts = (id: string) => json<Facts>(`/api/facts/${id}`)
export const rebuild = (store: string) => json<{ ok: boolean }>(`/api/stores/${store}/rebuild`, { method: 'POST' })

export function mark(store: string, ids: string[], m: { rating?: number; flag?: number }) {
  return json<{ ok: boolean; queue: QueueStatus }>(`/api/stores/${store}/marks`, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ ids, ...m }),
  })
}

export const thumbUrl = (id: string) => `/api/thumb/${id}`
export const mediaUrl = (id: string) => `/api/media/${id}`
