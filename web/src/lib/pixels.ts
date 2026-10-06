// Full-size images, fetched and decoded before they are needed.
//
// Speed in a viewer is not drawing fast, it is never waiting: by the time a
// key moves to the next image, that image has already crossed the network
// and been decoded into pixels. So the loupe keeps a small pool of decoded
// <img> elements around the current one, and showing an image is moving an
// element that is already ready into place.
//
// A 1536×1536 image decodes to about 9 MB, so the pool is kept small and the
// oldest are let go first.

import { mediaUrl } from './api'

interface Entry {
  img: HTMLImageElement
  ready: Promise<HTMLImageElement>
  done: boolean
  failed: boolean
  used: number
}

const LIMIT = 14
const pool = new Map<string, Entry>()
let clock = 0

export function get(id: string): Entry {
  let e = pool.get(id)
  if (!e) {
    const img = new Image()
    img.decoding = 'async'
    img.draggable = false
    img.src = mediaUrl(id)
    const entry: Entry = {
      img,
      done: false,
      failed: false,
      used: ++clock,
      ready: img.decode().then(
        () => {
          entry.done = true
          return img
        },
        (err) => {
          entry.failed = true
          pool.delete(id)
          throw err
        },
      ),
    }
    entry.ready.catch(() => {})
    e = entry
    pool.set(id, e)
    trim()
  }
  e.used = ++clock
  return e
}

/** Decoded and ready to show without a wait. */
export function ready(id: string): HTMLImageElement | null {
  const e = pool.get(id)
  if (!e || !e.done) return null
  e.used = ++clock
  return e.img
}

/** Warm the pool around a position: mostly ahead in the direction of travel. */
export function around(ids: string[], i: number, dir: number) {
  const order = dir >= 0 ? [0, 1, 2, 3, -1, 4, -2] : [0, -1, -2, -3, 1, -4, 2]
  for (const d of order) {
    const k = i + d
    if (k >= 0 && k < ids.length) get(ids[k])
  }
}

function trim() {
  if (pool.size <= LIMIT) return
  const old = [...pool.entries()].sort((a, b) => a[1].used - b[1].used)
  for (const [id, e] of old.slice(0, pool.size - LIMIT)) {
    // An element on screen is the one in use; never take it away.
    if (e.img.isConnected) continue
    e.img.src = ''
    pool.delete(id)
  }
}
