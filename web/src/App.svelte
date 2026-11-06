<script lang="ts">
  import { onMount } from 'svelte'
  import LeftRail from './lib/LeftRail.svelte'
  import Grid from './lib/Grid.svelte'
  import Loupe from './lib/Loupe.svelte'
  import MarkToast from './lib/MarkToast.svelte'
  import Inspector from './lib/Inspector.svelte'
  import Config from './lib/Config.svelte'
  import Filmstrip from './lib/Filmstrip.svelte'
  import Lab from './lib/Lab.svelte'
  import FilterBar from './lib/FilterBar.svelte'
  import { apply as applyFilter, readFilter, writeFilter, type Filter } from './lib/filter'
  import { getStores, getIndex, getImages, getHealth, mark } from './lib/api'
  import type { Store, IndexState, Images, Health } from './lib/api'
  import { count } from './lib/format'

  let stores = $state<Store[]>([])
  let storesError = $state<string | null>(null)
  let health = $state<Health | null>(null)
  let current = $state<Store | null>(null)
  let index = $state<IndexState | null>(null)
  let set = $state<string | null>(null)
  // Raw, not deep: 200,000-long columns would cost a proxy per element. A
  // mark replaces the column it changes, which is what tells the page.
  // `loaded` is the whole set; `images` is what the filter lets through,
  // and is what every view and the cursor work on.
  let loaded = $state.raw<Images | null>(null)
  let filter = $state<Filter>(readFilter())
  const images = $derived(loaded ? applyFilter(loaded, filter) : null)
  let imagesError = $state<string | null>(null)
  let cursor = $state(0)
  let mode = $state<'grid' | 'loupe'>('grid')
  // Modules, Lightroom's way: each takes the whole screen below the top bar.
  // Library is the viewer; Lab holds development views that do not fit its
  // layout; Config edits pand's own files. The module is in the address
  // (?m=lab), so a lab view can be reloaded or sent as a link.
  type Module = 'library' | 'lab' | 'config'
  const MODULES: { id: Module; title: string }[] = [
    { id: 'library', title: 'Library' },
    { id: 'lab', title: 'Lab' },
    { id: 'config', title: 'Config' },
  ]
  let module = $state<Module>(readModule())
  function readModule(): Module {
    const m = new URLSearchParams(location.search).get('m')
    return m === 'lab' || m === 'config' ? m : 'library'
  }
  function setModule(m: Module) {
    module = m
    const q = new URLSearchParams(location.search)
    if (m === 'library') q.delete('m')
    else q.set('m', m)
    const qs = q.toString()
    history.replaceState(null, '', `${location.pathname}${qs ? `?${qs}` : ''}${location.hash}`)
  }

  // Panels that can be put away: the two sides and the filmstrip. Each is
  // remembered across reloads.
  type Panels = { left: boolean; right: boolean; film: boolean }
  let panels = $state<Panels>(readPanels())
  function readPanels(): Panels {
    try {
      return { left: true, right: true, film: true, ...JSON.parse(localStorage.getItem('pan-ui.panels') ?? '{}') }
    } catch {
      return { left: true, right: true, film: true }
    }
  }
  function setPanels(p: Partial<Panels>) {
    panels = { ...panels, ...p }
    try {
      localStorage.setItem('pan-ui.panels', JSON.stringify(panels))
    } catch {}
  }
  /** Tab: both sides away, or both back. Shift+Tab: the same, filmstrip too. */
  function togglePanels(all: boolean) {
    const anyOpen = panels.left || panels.right || (all && panels.film)
    setPanels(all ? { left: !anyOpen, right: !anyOpen, film: !anyOpen } : { left: !anyOpen, right: !anyOpen })
  }
  const filmShown = $derived(panels.film && mode === 'loupe')
  let dir = $state(1)
  let cols = $state(1)
  let size = $state(readSize())
  let markError = $state<string | null>(null)

  function readSize(): number {
    try {
      return Number(localStorage.getItem('pan-ui.size')) || 180
    } catch {
      return 180
    }
  }
  function setSize(n: number) {
    size = Math.max(80, Math.min(480, n))
    try {
      localStorage.setItem('pan-ui.size', String(size))
    } catch {}
  }

  // The address bar carries store, set and image, so a view can be sent to
  // someone or come back after a reload: #<store>/<set>/<image>, with an
  // empty set for "everything".
  function writeHash() {
    if (!current) return
    const id = images?.ids[cursor] ?? ''
    const h = `#${current.id}/${set ? encodeURIComponent(set) : ''}/${id}`
    if (location.hash !== h) history.replaceState(null, '', `${location.pathname}${location.search}${h}`)
  }
  function readHash(): [string | null, string | null, string | null] {
    const [s, t, i] = location.hash.replace(/^#/, '').split('/')
    return [s || null, t ? decodeURIComponent(t) : null, i || null]
  }

  let indexTimer: ReturnType<typeof setTimeout> | undefined
  async function openStore(s: Store, wantSet: string | null = null, wantId: string | null = null) {
    clearTimeout(indexTimer)
    current = s
    index = null
    loaded = null
    imagesError = null
    set = wantSet
    cursor = 0
    mode = 'grid'
    writeHash()
    await pollIndex(s.id, wantId)
  }

  /** Ask until the store's index is ready, then keep asking, more slowly,
   *  so a rebuild pan-ui does in the background shows up here too. */
  async function pollIndex(store: string, wantId: string | null = null) {
    let next = 1500
    try {
      const ix = await getIndex(store)
      if (current?.id !== store) return
      const changed = ix.built !== index?.built
      index = ix
      if (ix.ready && (changed || !images)) {
        if (set && !ix.sets?.some((x) => x.id === set)) set = null
        await loadImages(wantId ?? images?.ids[cursor] ?? null)
      }
      next = ix.ready && !ix.building ? 30000 : 1500
    } catch (e) {
      if (current?.id === store) index = { store, ready: false, building: false, error: (e as Error).message }
      next = 5000
    }
    if (current?.id === store) indexTimer = setTimeout(() => pollIndex(store), next)
  }

  let loadSeq = 0
  /** Load the open set's columns and put the cursor on keepId; if the set
   *  does not hold it, on the image nearest keepT in time. */
  async function loadImages(keepId: string | null = null, keepT: number | null = null) {
    if (!current) return
    const store = current.id
    const seq = ++loadSeq
    try {
      const im = newestFirst(await getImages(store, set))
      if (seq !== loadSeq) return
      loaded = im
      imagesError = null
      place(keepId, keepT)
    } catch (e) {
      if (seq === loadSeq) imagesError = (e as Error).message
    }
  }

  /** Put the cursor on keepId if it is shown, else on the shown image
   *  nearest keepT in time. */
  function place(keepId: string | null, keepT: number | null) {
    if (!images) return
    const k = keepId ? images.ids.indexOf(keepId) : -1
    cursor = k >= 0 ? k : keepT != null ? nearest(images.t, keepT) : 0
    writeHash()
  }

  /** Change the filter and stay with the image, or the nearest one shown. */
  function setFilter(f: Filter) {
    const keepId = images?.ids[cursor] ?? null
    const keepT = images?.t[cursor] ?? null
    filter = f
    writeFilter(f)
    place(keepId, keepT)
  }

  /** pand sends a set oldest first; the newest images are what you come to
   *  see, so they go at the top. Every column is turned round together. */
  function newestFirst(im: Images): Images {
    return { ...im, ids: im.ids.reverse(), t: im.t.reverse(), rating: im.rating.reverse(), flag: im.flag.reverse() }
  }

  /** Index of the time in a newest-first column closest to t. */
  function nearest(ts: number[], t: number): number {
    let lo = 0,
      hi = ts.length - 1
    if (hi < 0) return 0
    while (lo < hi) {
      const mid = (lo + hi) >> 1
      if (ts[mid] > t) lo = mid + 1
      else hi = mid
    }
    return lo > 0 && ts[lo - 1] - t <= t - ts[lo] ? lo - 1 : lo
  }

  /** Open a set and stay with the image under the cursor: on it, if the set
   *  holds it (as "everything" always does), or on the set's image nearest
   *  to it in time. The view, grid or full size, stays as it was. */
  function openSet(s: string | null) {
    const keepId = images?.ids[cursor] ?? null
    const keepT = images?.t[cursor] ?? null
    set = s
    loaded = null
    loadImages(keepId, keepT)
  }

  function move(to: number) {
    if (!images || !images.ids.length) return
    const k = Math.max(0, Math.min(images.ids.length - 1, to))
    dir = k >= cursor ? 1 : -1
    cursor = k
    writeHash()
  }

  /** Somewhere else in the set, at random: directed browsing only ever
   *  shows the newest work, and this brings the rest back into view. Only
   *  images the filter lets through; never the one already up. */
  function jump() {
    const n = images?.ids.length ?? 0
    if (n < 2) return
    const here = images!.ids[cursor]
    if (here && jumpedFrom.at(-1) !== here) jumpedFrom = [...jumpedFrom, here].slice(-200)
    const k = Math.floor(Math.random() * (n - 1))
    move(k >= cursor ? k + 1 : k)
  }

  // The images R left, latest last. Shift-R walks back through them, so
  // something glimpsed one R ago is not gone. Kept by id: a mark or a new
  // filter can move an image in the list, and one no longer shown is
  // stepped over.
  let notice = $state<string | null>(null)
  let noticeTimer: ReturnType<typeof setTimeout> | undefined
  /** A word in the top bar for a moment. */
  function say(msg: string) {
    notice = msg
    clearTimeout(noticeTimer)
    noticeTimer = setTimeout(() => (notice = null), 6000)
  }

  let jumpedFrom: string[] = []
  function jumpBack() {
    if (!images) return
    while (jumpedFrom.length) {
      const id = jumpedFrom[jumpedFrom.length - 1]
      jumpedFrom = jumpedFrom.slice(0, -1)
      const k = images.ids.indexOf(id)
      if (k >= 0 && k !== cursor) return move(k)
    }
    say('no random jump to go back to')
  }

  /** The marks of the image under the cursor, from the whole set. */
  function marksHere() {
    if (!images || !loaded) return null
    const j = images.src ? images.src[cursor] : cursor
    return { rating: loaded.rating[j] ?? 0, flag: loaded.flag[j] ?? 0 }
  }

  /** , and . step the flag: , toward pick, . toward reject, through no
   *  flag on the way. P X U stay for Lightroom hands; these sit together,
   *  so flagging needs no looking down (goodlux, 2026-11-05). */
  function stepFlag(toPick: boolean) {
    const m = marksHere()
    if (!m) return
    apply({ flag: toPick ? (m.flag < 0 ? 0 : 1) : m.flag > 0 ? 0 : -1 }, false)
  }

  /** Shift-Up and Shift-Down step the stars, with 3 as average. Most
   *  images are average or liked, so each is one press from unrated:
   *  Shift-Down gives 3, Shift-Up 4. Then one star at a time between 1
   *  and 5, so 2 and 5 are two presses and 1 is three. Back to unrated
   *  is 0 (goodlux, 2026-11-06). */
  function stepRating(up: boolean) {
    const m = marksHere()
    if (!m) return
    const r = m.rating === 0 ? (up ? 4 : 3) : Math.max(1, Math.min(5, m.rating + (up ? 1 : -1)))
    apply({ rating: r }, false)
  }

  let toast = $state<{ kind: 'rating' | 'flag'; rating: number; flag: number; n: number } | null>(null)

  /** Change the image's column at once; the server queues it for Pan. */
  function apply(m: { rating?: number; flag?: number }, advance: boolean) {
    if (!images || !loaded || !current) return
    const i = cursor
    const id = images.ids[i]
    if (!id) return
    // Written to the whole set; the filter then decides whether it stays.
    const j = images.src ? images.src[i] : i
    let { rating, flag } = loaded
    if (m.rating !== undefined && rating[j] !== m.rating) {
      rating = rating.slice()
      rating[j] = m.rating
    }
    if (m.flag !== undefined && flag[j] !== m.flag) {
      flag = flag.slice()
      flag[j] = m.flag
    }
    loaded = { ...loaded, rating, flag }
    toast = { kind: m.flag !== undefined ? 'flag' : 'rating', rating: rating[j], flag: flag[j], n: (toast?.n ?? 0) + 1 }
    mark(current.id, [id], m).then(
      (r) => {
        markError = null
        if (health) health = { ...health, queue: r.queue }
      },
      (e) => (markError = `not saved: ${(e as Error).message}`),
    )
    // If the mark took the image out of the filter, the next one has
    // already slid into its place; stepping on would skip it.
    if (images.ids[i] !== id) move(i)
    else if (advance) move(i + 1)
  }

  function onkey(e: KeyboardEvent) {
    const t = e.target as HTMLElement
    if (t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA') return
    // Config and Lab have their own keys; nothing here may mark an image
    // that is not even on screen.
    if (module !== 'library') return
    if (e.metaKey || e.ctrlKey || e.altKey) return
    if (e.key === 'Tab') {
      e.preventDefault()
      togglePanels(e.shiftKey)
      return
    }
    const adv = e.shiftKey
    const digit = e.code.match(/^(?:Digit|Numpad)([0-5])$/)
    let handled = true
    if (digit) apply({ rating: Number(digit[1]) }, adv)
    else if (e.code === 'KeyP') apply({ flag: 1 }, adv)
    else if (e.code === 'KeyX') apply({ flag: -1 }, adv)
    else if (e.code === 'KeyU') apply({ flag: 0 }, adv)
    else if (e.key === 'ArrowRight') move(cursor + 1)
    else if (e.key === 'ArrowLeft') move(cursor - 1)
    else if (e.key === 'ArrowUp' && adv) stepRating(true)
    else if (e.key === 'ArrowDown' && adv) stepRating(false)
    else if (e.code === 'Comma') stepFlag(true)
    else if (e.code === 'Period') stepFlag(false)
    else if (e.key === 'ArrowDown') move(cursor + (mode === 'grid' ? cols : 1))
    else if (e.key === 'ArrowUp') move(cursor - (mode === 'grid' ? cols : 1))
    // \ is R for the right hand, by the arrows (goodlux, 2026-11-06).
    else if (e.code === 'KeyR' || e.code === 'Backslash') adv ? jumpBack() : jump()
    else if (e.key === 'Home') move(0)
    else if (e.key === 'End') move(Infinity)
    else if (e.key === 'PageDown' && mode === 'grid') move(cursor + cols * 4)
    else if (e.key === 'PageUp' && mode === 'grid') move(cursor - cols * 4)
    else if (e.key === 'Enter' || e.key === ' ') mode = mode === 'grid' ? 'loupe' : 'grid'
    else if (e.code === 'KeyE') mode = 'loupe'
    else if (e.key === 'Escape' || e.code === 'KeyG') mode = 'grid'
    else if (e.key === '=' || e.key === '+') setSize(size + 40)
    else if (e.key === '-' || e.key === '_') setSize(size - 40)
    else handled = false
    if (handled) e.preventDefault()
  }

  const tally = $derived.by(() => {
    const images = loaded
    if (!images) return null
    let picked = 0,
      rejected = 0,
      rated = 0
    for (let i = 0; i < images.ids.length; i++) {
      if (images.flag[i] > 0) picked++
      else if (images.flag[i] < 0) rejected++
      if (images.rating[i] > 0) rated++
    }
    return { picked, rejected, rated }
  })

  const queue = $derived(health?.queue)

  onMount(() => {
    const poll = async () => {
      try {
        health = await getHealth()
      } catch (e) {
        health = { pand: null, pand_url: '', error: (e as Error).message, queue: { pending: 0, saved: 0, last_error: null } }
      }
    }
    poll()
    const iv = setInterval(poll, 2000)
    ;(async () => {
      try {
        stores = await getStores()
      } catch (e) {
        storesError = (e as Error).message
        return
      }
      const [s, t, i] = readHash()
      const want = stores.find((x) => x.id === s) ?? stores.find((x) => (x.images ?? 0) > 0)
      if (want) openStore(want, want.id === s ? t : null, want.id === s ? i : null)
    })()
    return () => {
      clearInterval(iv)
      clearTimeout(indexTimer)
    }
  })
</script>

<svelte:window onkeydown={onkey} />

<div
  class="page"
  class:no-left={!panels.left || module !== 'library'}
  class:no-right={!panels.right || module !== 'library'}
  class:no-film={!filmShown || module !== 'library'}
  class:film-edge={mode === 'loupe' && module === 'library'}
>
  <header class="top">
    <span class="brand">pan</span>
    {#if current}
      <span class="storename">{current.name}</span>
      {#if module === 'library'}
        <span class="setname" title={set ?? ''}>{set ?? 'everything, newest first'}</span>
      {/if}
    {/if}
    {#if images && module === 'library'}
      <span class="meta">
        {count(images.ids.length ? cursor + 1 : 0)} / {count(images.ids.length)}
        {#if images.src && loaded}(filtered from {count(loaded.ids.length)}){/if}
        {#if tally}
          · {count(tally.picked)} picked · {count(tally.rejected)} rejected · {count(tally.rated)} rated
        {/if}
      </span>
    {/if}
    <span class="save" class:warn={notice || markError || queue?.last_error}>
      {#if notice}
        {notice}
      {:else if markError}
        {markError}
      {:else if queue?.last_error}
        {queue.pending} waiting to save · {queue.last_error}
      {:else if queue?.pending}
        saving {queue.pending} to Pan…
      {:else if queue}
        saved
      {/if}
    </span>
    <nav class="modules">
      {#each MODULES as m, k (m.id)}
        {#if k}<span class="sep">|</span>{/if}
        <button class:on={module === m.id} onclick={() => setModule(m.id)}>{m.title}</button>
      {/each}
    </nav>
  </header>

  {#if module === 'library'}
    <button class="edge edge-left" onclick={() => setPanels({ left: !panels.left })} title="{panels.left ? 'hide' : 'show'} the left panel (Tab: both sides)">
      {panels.left ? '◂' : '▸'}
    </button>
    <div class="left">
      <LeftRail {stores} {current} {index} {set} {health} onstore={(s) => openStore(s)} onset={openSet} />
    </div>

    <main>
      {#if loaded}
        <FilterBar {filter} shown={images?.ids.length ?? 0} total={loaded.ids.length} onchange={setFilter} />
      {/if}
      <div class="stage">
      {#if storesError}
        <p class="state warn">Could not list stores: {storesError}</p>
      {:else if imagesError}
        <p class="state warn">{imagesError}</p>
      {:else if images && images.ids.length === 0}
        <p class="state">{images.src ? 'no image in this set passes the filter' : 'no images'}</p>
      {:else if images}
        {#if mode === 'grid'}
          <Grid {images} {cursor} {size} bind:cols onpick={move} onopen={(i) => (move(i), (mode = 'loupe'))} />
        {:else}
          <Loupe {images} {cursor} {dir} onclose={() => (mode = 'grid')} />
        {/if}
      {:else if current}
        <p class="state">reading {current.name}…</p>
      {/if}
      {#if toast}<MarkToast {...toast} />{/if}
      </div>
    </main>

    <div class="right">
      <Inspector store={current?.id ?? null} {images} {cursor} {set} onset={openSet} />
    </div>
    <button class="edge edge-right" onclick={() => setPanels({ right: !panels.right })} title="{panels.right ? 'hide' : 'show'} the right panel (Tab: both sides)">
      {panels.right ? '▸' : '◂'}
    </button>

    {#if mode === 'loupe'}
      <div class="film">
        {#if filmShown && images}<Filmstrip {images} {cursor} onpick={move} />{/if}
      </div>
      <button class="edge edge-bottom" onclick={() => setPanels({ film: !panels.film })} title="{panels.film ? 'hide' : 'show'} the filmstrip (Shift+Tab: all panels)">
        {panels.film ? '▾' : '▴'}
      </button>
    {/if}
  {:else}
    <section class="whole">
      {#if module === 'config'}
        <Config />
      {:else}
        <Lab store={current} image={images?.ids[cursor] ?? null} />
      {/if}
    </section>
  {/if}
</div>

<style>
  /* Five columns: an edge strip, the left panel, the stage, the right panel,
     an edge strip. A panel put away is a column of width 0; it stays in the
     page, so a typed filter or a scroll position survives hiding it. */
  .page {
    --left: 16rem;
    --right: 28rem;
    --film: 112px;
    --edge: 12px;
    display: grid;
    grid-template-columns: var(--edge) var(--left) 1fr var(--right) var(--edge);
    grid-template-rows: auto 1fr var(--film) 0;
    height: 100vh;
  }
  .page.no-left { --left: 0px; }
  .page.no-right { --right: 0px; }
  .page.no-film { --film: 0px; }
  .page.film-edge { grid-template-rows: auto 1fr var(--film) var(--edge); }
  .top {
    grid-column: 1 / -1;
    display: flex;
    align-items: baseline;
    gap: 0.9rem;
    padding: 0.45rem 0.9rem;
    border-bottom: 1px solid var(--rule-strong);
    white-space: nowrap;
    overflow: hidden;
  }
  .brand, .storename { font-family: var(--display); font-size: 1.05rem; }
  .setname { font-size: 0.78rem; overflow: hidden; text-overflow: ellipsis; min-width: 0; flex-shrink: 1; }
  .meta { font-size: 0.75rem; color: var(--ink-soft); }
  .save { margin-left: auto; font-size: 0.75rem; color: var(--ink-faint); overflow: hidden; text-overflow: ellipsis; }
  .modules { display: flex; align-items: baseline; gap: 0.35rem; }
  .modules button {
    border: 0;
    background: none;
    padding: 0 0.15rem;
    font-family: var(--display);
    font-size: 1rem;
    color: var(--ink-faint);
  }
  .modules button:hover { background: none; color: var(--ink); }
  .modules button.on { color: var(--ink); text-decoration: underline; text-underline-offset: 0.3em; }
  .sep { color: var(--rule-strong); opacity: 0.3; }

  .left { grid-column: 2; grid-row: 2 / 4; min-height: 0; overflow: hidden; }
  main { grid-column: 3; grid-row: 2; display: flex; flex-direction: column; min-width: 0; min-height: 0; background: var(--stage); }
  .stage { position: relative; flex: 1; min-height: 0; }
  .right { grid-column: 4; grid-row: 2 / 4; min-height: 0; overflow: hidden; }
  .film { grid-column: 3; grid-row: 3; position: relative; min-width: 0; overflow: hidden; }
  .whole { grid-column: 1 / -1; grid-row: 2 / -1; position: relative; min-height: 0; }

  /* The edge strips, Lightroom's: a thin bar along the screen's edge with a
     triangle that puts the panel away or brings it back. */
  .edge {
    border: 0;
    padding: 0;
    background: var(--paper-tint);
    color: var(--ink-faint);
    font-size: 0.6rem;
    line-height: 1;
  }
  .edge:hover:not(:disabled) { background: var(--rule); color: var(--ink); }
  .edge-left { grid-column: 1; grid-row: 2 / -1; border-right: 1px solid var(--rule); }
  .edge-right { grid-column: 5; grid-row: 2 / -1; border-left: 1px solid var(--rule); }
  .edge-bottom { grid-column: 2 / 5; grid-row: 4; border-top: 1px solid var(--rule); }

  .state { padding: 2rem; color: var(--stage-soft); }
  .warn { color: var(--warn); }
  .state.warn { white-space: pre-wrap; }
</style>
