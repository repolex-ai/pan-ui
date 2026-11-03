// The Library filter, Lightroom's: which flags to show and a star rating to
// compare against. One filter for the page; it stays as you move between
// sets, and is remembered across reloads.

import type { Images } from './api'

export type Op = '>=' | '<=' | '='

export interface Filter {
  /** Flags to show. None on means every flag. */
  picked: boolean
  unflagged: boolean
  rejected: boolean
  /** null: any rating. 0 means no stars. */
  stars: number | null
  op: Op
}

export const NO_FILTER: Filter = { picked: false, unflagged: false, rejected: false, stars: null, op: '>=' }

export function isOff(f: Filter): boolean {
  return !f.picked && !f.unflagged && !f.rejected && f.stars === null
}

export function readFilter(): Filter {
  try {
    return { ...NO_FILTER, ...JSON.parse(localStorage.getItem('pan-ui.filter') ?? '{}') }
  } catch {
    return { ...NO_FILTER }
  }
}

export function writeFilter(f: Filter) {
  try {
    localStorage.setItem('pan-ui.filter', JSON.stringify(f))
  } catch {}
}

/** The images that pass, as columns of their own, plus where each came
 *  from in the full set so a mark can be written back to it. */
export interface View extends Images {
  src: number[] | null
}

export function apply(im: Images, f: Filter): View {
  if (isOff(f)) return { ...im, src: null }
  const anyFlag = f.picked || f.unflagged || f.rejected
  const src: number[] = []
  for (let i = 0; i < im.ids.length; i++) {
    const fl = im.flag[i]
    if (anyFlag && !(fl > 0 ? f.picked : fl < 0 ? f.rejected : f.unflagged)) continue
    if (f.stars !== null) {
      const r = im.rating[i]
      if (f.op === '>=' ? r < f.stars : f.op === '<=' ? r > f.stars : r !== f.stars) continue
    }
    src.push(i)
  }
  return {
    ...im,
    ids: src.map((i) => im.ids[i]),
    t: src.map((i) => im.t[i]),
    rating: src.map((i) => im.rating[i]),
    flag: src.map((i) => im.flag[i]),
    src,
  }
}
