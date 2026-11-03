<script lang="ts">
  // The Atlas: every image with an embedding, as one dot, placed so that
  // images that look alike sit close together (UMAP over the store's own
  // vectors). Colour is when it was made. Zoom in and the dots become the
  // images. Press play and watch the store fill in, day by day.
  import type { LabProps } from './index'
  import { meta, atlas, neighbours, near, type Hit } from './data'
  import { thumbUrl, getImages } from '../api'
  import { when, count } from '../format'
  import Pic from './Pic.svelte'

  let { store, image, onopen }: LabProps = $props()

  let canvas = $state<HTMLCanvasElement | null>(null)
  let w = $state(0)
  let h = $state(0)
  let error = $state<string | null>(null)
  let loading = $state(true)

  // The data, plain (not reactive): 65,000-long arrays.
  let ids = $state.raw<string[]>([])
  let ts = $state.raw<number[]>([])
  let xy: Float32Array = new Float32Array(0)
  let nb: { idx: Uint32Array; sim: Float32Array } | null = null
  let k = 16
  // Each image's place in time order, 0 (oldest) to 1 (newest). Colour
  // follows this rather than the date itself: most of a store is often
  // made in a few weeks, and a straight timeline would paint it all one
  // colour.
  let rank: Float32Array = new Float32Array(0)
  let order: number[] = []
  let n = $state(0)

  // The view: world (0..1) to screen.
  let zoom = $state(1)
  let cx = $state(0.5)
  let cy = $state(0.5)

  let picked = $state<number | null>(null)
  let strangers = $state<Hit[] | null>(null)
  let until = $state(1) // 0..1 of the time range shown
  let playing = $state(false)
  let colourBy = $state<'time' | 'set'>('time')
  let setRows = $state<Set<number> | null>(null)
  let setName = $state<string | null>(null)

  $effect(() => {
    const st = store?.id
    if (!st) return
    loading = true
    error = null
    Promise.all([meta(st), atlas(st), neighbours(st)]).then(
      ([m, a, nbr]) => {
        ids = m.ids
        ts = m.t
        xy = a
        nb = nbr
        k = m.k
        n = ids.length
        order = Array.from({ length: n }, (_, i) => i).sort((a, b) => ts[a] - ts[b])
        rank = new Float32Array(n)
        order.forEach((i, r) => (rank[i] = r / Math.max(1, n - 1)))
        loading = false
        if (image && m.pos.has(image)) pick(m.pos.get(image)!, true)
        draw()
      },
      (e) => ((error = (e as Error).message), (loading = false)),
    )
  })

  // Oldest is cool blue, newest warm amber, through green; f is the place
  // in time order, 0 to 1.
  function colour(f: number): string {
    const hue = 215 - f * 180
    return `hsl(${Math.round(hue)} 72% ${Math.round(48 + f * 12)}%)`
  }
  /** The date at a place in time order, for the legend and the slider. */
  const dateAt = (f: number) => (order.length ? ts[order[Math.min(order.length - 1, Math.floor(f * (order.length - 1)))]] : 0)

  const thumbs = new Map<string, HTMLImageElement>()
  function thumb(id: string): HTMLImageElement | null {
    let im = thumbs.get(id)
    if (!im) {
      if (thumbs.size > 1500) thumbs.clear()
      im = new Image()
      im.decoding = 'async'
      im.onload = () => schedule()
      im.src = thumbUrl(id)
      thumbs.set(id, im)
    }
    return im.complete && im.naturalWidth ? im : null
  }

  const scale = () => Math.min(w, h) * 0.92 * zoom
  const sx = (x: number) => w / 2 + (x - cx) * scale()
  const sy = (y: number) => h / 2 + (y - cy) * scale()

  let frame = 0
  function schedule() {
    if (!frame) frame = requestAnimationFrame(() => ((frame = 0), draw()))
  }

  function draw() {
    if (!canvas || !n) return
    const dpr = window.devicePixelRatio || 1
    if (canvas.width !== Math.round(w * dpr)) canvas.width = Math.round(w * dpr)
    if (canvas.height !== Math.round(h * dpr)) canvas.height = Math.round(h * dpr)
    const g = canvas.getContext('2d')!
    g.setTransform(dpr, 0, 0, dpr, 0, 0)
    g.fillStyle = '#121212'
    g.fillRect(0, 0, w, h)
    // Dot size grows a little with zoom; past a point, pictures take over.
    // The screen is cut into cells a picture wide, and each cell shows the
    // first image that falls in it, so pictures spread over the whole view
    // and never pile up.
    const dot = Math.max(1.2, Math.min(4, 1.2 * Math.sqrt(zoom)))
    const tsize = Math.min(110, 13 * Math.sqrt(zoom))
    const showThumbs = tsize >= 30
    const vis: number[] = []
    // Batch by colour bucket to keep fillStyle changes few.
    const buckets = new Map<string, number[]>()
    for (let i = 0; i < n; i++) {
      if (rank[i] > until) continue
      const x = sx(xy[2 * i]),
        y = sy(xy[2 * i + 1])
      if (x < -tsize || y < -tsize || x > w + tsize || y > h + tsize) continue
      if (showThumbs) vis.push(i)
      let c: string
      if (colourBy === 'set' && setRows) c = setRows.has(i) ? '#f5c542' : '#3a3a3a'
      else c = colour(Math.round(rank[i] * 40) / 40)
      let b = buckets.get(c)
      if (!b) buckets.set(c, (b = []))
      b.push(i)
    }
    for (const [c, rows] of buckets) {
      g.fillStyle = c
      for (const i of rows) g.fillRect(sx(xy[2 * i]) - dot / 2, sy(xy[2 * i + 1]) - dot / 2, dot, dot)
    }
    if (showThumbs) {
      const cell = tsize * 0.95
      const taken = new Set<number>()
      const chosen: number[] = []
      for (const i of vis) {
        const key = Math.floor(sx(xy[2 * i]) / cell) * 10007 + Math.floor(sy(xy[2 * i + 1]) / cell)
        if (taken.has(key)) continue
        taken.add(key)
        chosen.push(i)
        if (chosen.length >= 700) break
      }
      for (const i of chosen) {
        const im = thumb(ids[i])
        if (!im) continue
        const r = Math.min(tsize / im.naturalWidth, tsize / im.naturalHeight)
        const iw = im.naturalWidth * r,
          ih = im.naturalHeight * r
        g.drawImage(im, sx(xy[2 * i]) - iw / 2, sy(xy[2 * i + 1]) - ih / 2, iw, ih)
      }
    }
    if (picked !== null && nb) {
      const px = sx(xy[2 * picked]),
        py = sy(xy[2 * picked + 1])
      g.strokeStyle = '#ffffffaa'
      g.lineWidth = 1
      for (let j = 0; j < k; j++) {
        const q = nb.idx[picked * k + j]
        g.beginPath()
        g.moveTo(px, py)
        g.lineTo(sx(xy[2 * q]), sy(xy[2 * q + 1]))
        g.stroke()
      }
      g.strokeStyle = '#fff'
      g.lineWidth = 2
      g.strokeRect(px - 7, py - 7, 14, 14)
    }
  }

  $effect(() => {
    void w, h, zoom, cx, cy, until, picked, colourBy, setRows
    schedule()
  })

  // ---- pan, zoom, pick
  let drag: { x: number; y: number; cx: number; cy: number; moved: boolean } | null = null
  function down(e: PointerEvent) {
    drag = { x: e.clientX, y: e.clientY, cx, cy, moved: false }
    canvas?.setPointerCapture(e.pointerId)
  }
  function moveP(e: PointerEvent) {
    if (!drag) return
    const dx = e.clientX - drag.x,
      dy = e.clientY - drag.y
    if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true
    cx = drag.cx - dx / scale()
    cy = drag.cy - dy / scale()
  }
  function up(e: PointerEvent) {
    if (drag && !drag.moved) clickAt(e)
    drag = null
  }
  function wheel(e: WheelEvent) {
    e.preventDefault()
    const r = canvas!.getBoundingClientRect()
    const mx = e.clientX - r.left,
      my = e.clientY - r.top
    // Keep the point under the mouse where it is.
    const wx = cx + (mx - w / 2) / scale(),
      wy = cy + (my - h / 2) / scale()
    zoom = Math.max(0.5, Math.min(400, zoom * Math.exp(-e.deltaY * 0.0015)))
    cx = wx - (mx - w / 2) / scale()
    cy = wy - (my - h / 2) / scale()
  }
  function clickAt(e: PointerEvent) {
    const r = canvas!.getBoundingClientRect()
    const mx = e.clientX - r.left,
      my = e.clientY - r.top
    let best = -1,
      bd = 400
    for (let i = 0; i < n; i++) {
      if (rank[i] > until) continue
      const d = (sx(xy[2 * i]) - mx) ** 2 + (sy(xy[2 * i + 1]) - my) ** 2
      if (d < bd) (bd = d), (best = i)
    }
    if (best >= 0) pick(best, false)
  }

  function pick(i: number, centre: boolean) {
    picked = i
    strangers = null
    if (centre) {
      cx = xy[2 * i]
      cy = xy[2 * i + 1]
      zoom = Math.max(zoom, 8)
    }
    if (store) near(store.id, { plus: [ids[i]], k: 12, scope: 'outside' }).then((r) => picked === i && (strangers = r.hits), () => (strangers = []))
  }
  function goTo(id: string) {
    meta(store!.id).then((m) => {
      const i = m.pos.get(id)
      if (i !== undefined) pick(i, true)
    })
  }

  // ---- play: the store filling in over time
  let raf = 0
  function play() {
    if (playing) {
      playing = false
      cancelAnimationFrame(raf)
      return
    }
    playing = true
    if (until >= 1) until = 0
    let last = performance.now()
    const tick = (t: number) => {
      until = Math.min(1, until + (t - last) / 20000)
      last = t
      if (until >= 1) playing = false
      if (playing) raf = requestAnimationFrame(tick)
    }
    raf = requestAnimationFrame(tick)
  }

  // ---- colour by a set: the Library's sets, chosen here by name
  let setQuery = $state('')
  async function showSet() {
    if (!store) return
    const q = setQuery.trim()
    if (!q) {
      setRows = null
      setName = null
      colourBy = 'time'
      return
    }
    const ix = await fetch(`/api/stores/${store.id}/index`).then((r) => r.json())
    const s = (ix.sets ?? []).find((x: { id: string }) => x.id.includes(q))
    if (!s) {
      setName = `no set matches “${q}”`
      return
    }
    const im = await getImages(store.id, s.id)
    const m = await meta(store.id)
    setRows = new Set(im.ids.map((id) => m.pos.get(id)).filter((x): x is number => x !== undefined))
    setName = `${s.id} · ${count(setRows.size)} of ${count(im.ids.length)} on the map`
    colourBy = 'set'
  }

  const nbList = $derived.by(() => {
    if (picked === null || !nb) return []
    return Array.from({ length: k }, (_, j) => ({ id: ids[nb!.idx[picked! * k + j]], sim: nb!.sim[picked! * k + j] }))
  })
</script>

<div class="atlas">
  <div class="map" bind:clientWidth={w} bind:clientHeight={h}>
    <canvas
      bind:this={canvas}
      style="width:{w}px;height:{h}px"
      onpointerdown={down}
      onpointermove={moveP}
      onpointerup={up}
      onwheel={wheel}
    ></canvas>
    <div class="ctl">
      <b>Atlas</b>
      {#if loading}<span>reading the map…</span>{/if}
      {#if error}<span class="err">{error}</span>{/if}
      {#if n}
        <span>{count(n)} images, placed by look · colour is when · scroll to zoom, drag to move, click a dot</span>
        <div class="row">
          <button onclick={play}>{playing ? '❚❚' : '▶'} history</button>
          <input type="range" min="0" max="1" step="0.001" bind:value={until} />
          <span class="date">{when(dateAt(until))}</span>
          <button onclick={() => ((zoom = 1), (cx = 0.5), (cy = 0.5))}>whole map</button>
        </div>
        <div class="row">
          <input type="search" placeholder="light up a set (part of its name)" bind:value={setQuery} onchange={showSet} />
          {#if setName}<span class="date">{setName}</span>{/if}
        </div>
        <div class="row legend">
          {#each [0, 0.25, 0.5, 0.75, 1] as f (f)}
            <span class="sw" style="background:{colour(f)}"></span><span class="lg">{when(dateAt(f)).slice(5, 10)}</span>
          {/each}
          <span class="lg">(evenly by count, not by date)</span>
        </div>
      {/if}
    </div>
  </div>
  <aside>
    {#if picked !== null}
      <img class="big" src={thumbUrl(ids[picked])} alt="" />
      <p class="meta"><code>{ids[picked]}</code> · {when(ts[picked])}</p>
      <p><button onclick={() => onopen(ids[picked!])}>open in the Library</button></p>
      <h4>nearest by look <span>lines on the map</span></h4>
      <div class="grid">
        {#each nbList as x (x.id)}<Pic id={x.id} size={70} note={x.sim.toFixed(2)} onclick={goTo} {onopen} />{/each}
      </div>
      <h4>nearest outside its own sets <span>what it resembles elsewhere</span></h4>
      <div class="grid">
        {#if strangers === null}…{:else if !strangers.length}<span class="dim">none (or in no set)</span>{/if}
        {#each strangers ?? [] as x (x.id)}<Pic id={x.id} size={70} note={x.sim.toFixed(2)} onclick={goTo} {onopen} />{/each}
      </div>
    {:else}
      <p class="dim">Click a dot. Its sixteen nearest images by look are drawn as lines, and listed here with the nearest ones from outside its own sets.</p>
    {/if}
  </aside>
</div>

<style>
  .atlas { position: absolute; inset: 0; display: grid; grid-template-columns: 1fr 22rem; background: #121212; color: #bbb; }
  .map { position: relative; min-width: 0; overflow: hidden; }
  canvas { display: block; cursor: crosshair; touch-action: none; }
  .ctl { position: absolute; top: 0.6rem; left: 0.8rem; display: flex; flex-direction: column; gap: 0.35rem; font-size: 0.72rem; background: #000b; padding: 0.5rem 0.7rem; max-width: 34rem; }
  .ctl b { font-family: var(--display); font-size: 1rem; font-weight: normal; color: #eee; }
  .row { display: flex; gap: 0.5rem; align-items: center; }
  .row input[type='range'] { flex: 1; }
  .row input[type='search'] { flex: 1; background: #222; color: #ddd; border: 1px solid #444; font: inherit; padding: 0.15rem 0.35rem; }
  .ctl button { font-size: 0.72rem; padding: 0.1rem 0.5rem; background: #222; color: #ddd; border: 1px solid #555; }
  .date { color: #999; white-space: nowrap; }
  .legend .sw { display: inline-block; width: 1.6rem; height: 0.5rem; }
  .legend .lg { color: #888; margin-right: 0.3rem; white-space: nowrap; }
  .err { color: var(--warn); white-space: pre-wrap; }
  aside { border-left: 1px solid #000; overflow-y: auto; padding: 0.8rem; background: #181818; }
  .big { width: 100%; max-height: 22rem; object-fit: contain; background: #000; display: block; }
  .meta { font-size: 0.72rem; margin: 0.4rem 0; }
  aside button { font-size: 0.72rem; background: #222; color: #ddd; border: 1px solid #555; padding: 0.15rem 0.6rem; }
  h4 { font-size: 0.7rem; letter-spacing: 0.1em; text-transform: uppercase; font-weight: normal; color: #ddd; margin: 1rem 0 0.4rem; }
  h4 span { text-transform: none; letter-spacing: 0; color: #777; margin-left: 0.4rem; }
  .grid { display: flex; flex-wrap: wrap; gap: 5px; font-size: 0.7rem; }
  .dim { color: #777; font-size: 0.75rem; }
</style>
