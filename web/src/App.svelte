<script lang="ts">
  import { onMount } from 'svelte'
  import LeftRail from './lib/LeftRail.svelte'
  import Grid from './lib/Grid.svelte'
  import Loupe from './lib/Loupe.svelte'
  import Inspector from './lib/Inspector.svelte'
  import Config from './lib/Config.svelte'
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
  let images = $state.raw<Images | null>(null)
  let imagesError = $state<string | null>(null)
  let cursor = $state(0)
  let mode = $state<'grid' | 'loupe' | 'config'>('grid')
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
    images = null
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
  async function loadImages(keepId: string | null = null) {
    if (!current) return
    const store = current.id
    const seq = ++loadSeq
    try {
      const im = await getImages(store, set)
      if (seq !== loadSeq) return
      images = im
      imagesError = null
      const k = keepId ? im.ids.indexOf(keepId) : -1
      cursor = k >= 0 ? k : 0
      writeHash()
    } catch (e) {
      if (seq === loadSeq) imagesError = (e as Error).message
    }
  }

  function toggleConfig() {
    mode = mode === 'config' ? 'grid' : 'config'
  }

  function openSet(s: string | null) {
    set = s
    cursor = 0
    mode = 'grid'
    images = null
    loadImages()
  }

  function move(to: number) {
    if (!images || !images.ids.length) return
    const k = Math.max(0, Math.min(images.ids.length - 1, to))
    dir = k >= cursor ? 1 : -1
    cursor = k
    writeHash()
  }

  /** Change the image's column at once; the server queues it for Pan. */
  function apply(m: { rating?: number; flag?: number }, advance: boolean) {
    if (!images || !current) return
    const i = cursor
    const id = images.ids[i]
    if (!id) return
    let { rating, flag } = images
    if (m.rating !== undefined && rating[i] !== m.rating) {
      rating = rating.slice()
      rating[i] = m.rating
    }
    if (m.flag !== undefined && flag[i] !== m.flag) {
      flag = flag.slice()
      flag[i] = m.flag
    }
    images = { ...images, rating, flag }
    mark(current.id, [id], m).then(
      (r) => {
        markError = null
        if (health) health = { ...health, queue: r.queue }
      },
      (e) => (markError = `not saved: ${(e as Error).message}`),
    )
    if (advance) move(i + 1)
  }

  function onkey(e: KeyboardEvent) {
    const t = e.target as HTMLElement
    if (t?.tagName === 'INPUT' || t?.tagName === 'TEXTAREA') return
    // The config editor has its own keys; nothing here may mark an image
    // that is not even on screen.
    if (mode === 'config') return
    if (e.metaKey || e.ctrlKey || e.altKey) return
    const adv = e.shiftKey
    const digit = e.code.match(/^(?:Digit|Numpad)([0-5])$/)
    let handled = true
    if (digit) apply({ rating: Number(digit[1]) }, adv)
    else if (e.code === 'KeyP') apply({ flag: 1 }, adv)
    else if (e.code === 'KeyX') apply({ flag: -1 }, adv)
    else if (e.code === 'KeyU') apply({ flag: 0 }, adv)
    else if (e.key === 'ArrowRight') move(cursor + 1)
    else if (e.key === 'ArrowLeft') move(cursor - 1)
    else if (e.key === 'ArrowDown') move(cursor + (mode === 'grid' ? cols : 1))
    else if (e.key === 'ArrowUp') move(cursor - (mode === 'grid' ? cols : 1))
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

<div class="page">
  <header class="top">
    <span class="brand">pan</span>
    {#if current}
      <span class="storename">{current.name}</span>
      <span class="setname" title={set ?? ''}>{set ?? 'everything, in time order'}</span>
    {/if}
    {#if images}
      <span class="meta">
        {count(images.ids.length ? cursor + 1 : 0)} / {count(images.ids.length)}
        {#if tally}
          · {count(tally.picked)} picked · {count(tally.rejected)} rejected · {count(tally.rated)} rated
        {/if}
      </span>
    {/if}
    <span class="save" class:warn={markError || queue?.last_error}>
      {#if markError}
        {markError}
      {:else if queue?.last_error}
        {queue.pending} waiting to save · {queue.last_error}
      {:else if queue?.pending}
        saving {queue.pending} to Pan…
      {:else if queue}
        saved
      {/if}
    </span>
  </header>

  <LeftRail {stores} {current} {index} {set} {health} onstore={(s) => openStore(s)} onset={openSet} configOpen={mode === 'config'} onconfig={toggleConfig} />

  <main>
    {#if mode === 'config'}
      <Config />
    {:else if storesError}
      <p class="state warn">Could not list stores: {storesError}</p>
    {:else if imagesError}
      <p class="state warn">{imagesError}</p>
    {:else if images && images.ids.length === 0}
      <p class="state">no images</p>
    {:else if images}
      {#if mode === 'grid'}
        <Grid {images} {cursor} {size} bind:cols onpick={move} onopen={(i) => (move(i), (mode = 'loupe'))} />
      {:else}
        <Loupe {images} {cursor} {dir} onclose={() => (mode = 'grid')} />
      {/if}
    {:else if current}
      <p class="state">reading {current.name}…</p>
    {/if}
  </main>

  <Inspector {images} {cursor} />
</div>

<style>
  .page {
    display: grid;
    grid-template-columns: 16rem 1fr 22rem;
    grid-template-rows: auto 1fr;
    height: 100vh;
  }
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
  main { position: relative; min-width: 0; min-height: 0; background: var(--stage); }
  .state { padding: 2rem; color: var(--stage-soft); }
  .warn { color: var(--warn); }
  .state.warn { white-space: pre-wrap; }
</style>
