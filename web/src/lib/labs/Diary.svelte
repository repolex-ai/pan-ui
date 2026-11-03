<script lang="ts">
  // The Diary: a soul's memory beside what it made. A Pan store and the
  // soul's conversation record (ravel) share the soul's id, so the finest
  // memory notes ravel keeps, each a few turns of conversation, can be laid
  // next to the images made while that note was the latest one: from its
  // start until the next note begins. What was being said, and what came
  // out of it.
  import type { LabProps } from './index'
  import { getImages } from '../api'
  import { when, count } from '../format'
  import Pic from './Pic.svelte'

  let { store, image, onopen }: LabProps = $props()

  interface Notes {
    start: number[]
    end: number[]
    text: string[]
  }
  let notes = $state.raw<Notes | null>(null)
  let ids = $state.raw<string[]>([])
  let ts = $state.raw<number[]>([])
  let error = $state<string | null>(null)
  let q = $state('')
  let onlyImages = $state(true)
  let shown = $state(40)
  let jumpTo = $state<number | null>(null)

  $effect(() => {
    const st = store?.id
    if (!st) return
    error = null
    notes = null
    Promise.all([
      fetch(`/api/lab/${st}/notes`).then(async (r) => (r.ok ? r.json() : Promise.reject(new Error(await r.text())))),
      getImages(st, null),
    ]).then(
      ([n, im]) => {
        notes = n
        ids = im.ids
        ts = im.t
        shown = 40
      },
      (e) => (error = (e as Error).message),
    )
  })

  /** First index in sorted ts at or after t. */
  function lower(t: number): number {
    let lo = 0,
      hi = ts.length
    while (lo < hi) {
      const m = (lo + hi) >> 1
      if (ts[m] < t) lo = m + 1
      else hi = m
    }
    return lo
  }

  interface Moment {
    k: number
    from: number
    to: number
  }
  // Every note with the slice of images made from its start to the next
  // note's start. Newest first.
  const moments = $derived.by(() => {
    if (!notes) return []
    const n = notes.start.length
    const out: Moment[] = []
    const words = q.trim().toLowerCase().split(/\s+/).filter(Boolean)
    for (let k = n - 1; k >= 0; k--) {
      const a = notes.start[k]
      const b = k + 1 < n ? notes.start[k + 1] : Infinity
      const from = lower(a),
        to = lower(b)
      if (onlyImages && to <= from) continue
      if (words.length) {
        const t = notes.text[k].toLowerCase()
        if (!words.every((w) => t.includes(w))) continue
      }
      out.push({ k, from, to })
    }
    return out
  })

  // From the Library's image: the note that was the latest when it was made.
  function findImage() {
    if (!image || !notes) return
    const at = ids.indexOf(image)
    if (at < 0) return
    const t = ts[at]
    const i = moments.findIndex((m) => notes!.start[m.k] <= t)
    if (i < 0) return
    shown = Math.max(shown, i + 10)
    jumpTo = moments[i].k
    requestAnimationFrame(() => document.getElementById(`note-${jumpTo}`)?.scrollIntoView({ block: 'center' }))
  }

  function more(node: HTMLElement) {
    const io = new IntersectionObserver((es) => es.some((e) => e.isIntersecting) && (shown += 40))
    io.observe(node)
    return { destroy: () => io.disconnect() }
  }
</script>

<div class="diary">
  <header>
    <b>Diary</b>
    {#if notes}<span>{count(notes.start.length)} memory notes from {store?.name}'s conversation, beside the {count(ids.length)} images</span>{/if}
    <input type="search" placeholder="find in the notes" bind:value={q} oninput={() => (shown = 40)} />
    <label><input type="checkbox" bind:checked={onlyImages} /> only notes with images</label>
    {#if image}<button onclick={findImage}>the Library image's moment</button>{/if}
    {#if notes}<span class="dim">{count(moments.length)} shown</span>{/if}
  </header>
  {#if error}
    <p class="err">{error}</p>
  {:else if !notes}
    <p class="dim pad">asking raveld for {store?.name}'s memory…</p>
  {:else}
    <div class="feed">
      {#each moments.slice(0, shown) as m (m.k)}
        <article id="note-{m.k}" class:hit={jumpTo === m.k}>
          <div class="note">
            <span class="when">{when(notes.start[m.k])}</span>
            <p>{notes.text[m.k]}</p>
            {#if m.to > m.from}<span class="dim">{count(m.to - m.from)} image{m.to - m.from > 1 ? 's' : ''} until the next note</span>{/if}
          </div>
          <div class="imgs">
            {#each ids.slice(m.from, Math.min(m.to, m.from + 14)) as id (id)}<Pic {id} size={84} {onopen} onclick={onopen} />{/each}
            {#if m.to - m.from > 14}<span class="dim">+{count(m.to - m.from - 14)}</span>{/if}
          </div>
        </article>
      {/each}
      {#if moments.length > shown}<div class="sentinel" use:more>…</div>{/if}
    </div>
  {/if}
</div>

<style>
  .diary { position: absolute; inset: 0; display: flex; flex-direction: column; background: var(--stage); color: var(--stage-soft); }
  header { display: flex; gap: 1rem; align-items: center; padding: 0.45rem 0.9rem; border-bottom: 1px solid #000; font-size: 0.72rem; flex-wrap: wrap; }
  header b { color: var(--stage-ink); font-family: var(--display); font-size: 0.95rem; font-weight: normal; }
  header input[type='search'] { width: 16rem; background: #222; color: #ddd; border: 1px solid #444; font: inherit; padding: 0.15rem 0.35rem; }
  header button { font-size: 0.72rem; background: #222; color: #ddd; border: 1px solid #555; padding: 0.1rem 0.5rem; }
  label { display: inline-flex; gap: 0.3rem; align-items: center; }
  .dim { color: #777; }
  .pad { padding: 1rem; }
  .err { color: var(--warn); padding: 1rem; white-space: pre-wrap; }
  .feed { flex: 1; overflow-y: auto; padding: 0.4rem 0.9rem 2rem; }
  article { display: grid; grid-template-columns: minmax(18rem, 30rem) 1fr; gap: 1.2rem; padding: 0.8rem 0; border-bottom: 1px solid #262626; }
  article.hit { background: #262620; }
  .note p { margin: 0.25rem 0; font-size: 0.82rem; line-height: 1.5; color: #ddd; }
  .when { font-family: var(--mono); font-size: 0.68rem; color: #999; }
  .imgs { display: flex; flex-wrap: wrap; gap: 4px; align-content: flex-start; align-items: center; }
  .sentinel { text-align: center; color: #555; padding: 1rem; }
</style>
