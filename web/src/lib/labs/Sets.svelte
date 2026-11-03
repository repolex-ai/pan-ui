<script lang="ts">
  // Sets as a graph. Each set is a circle at the middle of its images on
  // the Atlas, sized by how many images it holds, coloured by when it was
  // made. A line joins a set to the sets it most looks like (the cosine of
  // their average vectors). The faint dots behind are every image.
  import type { LabProps } from './index'
  import { meta, atlas } from './data'
  import { thumbUrl, getImages } from '../api'
  import { when, count, day } from '../format'
  import Pic from './Pic.svelte'

  let { store, onopen, onset }: LabProps = $props()

  interface Node {
    id: string
    count: number
    first: number
    last: number
    on_map: number
    x: number | null
    y: number | null
    cover: string | null
    near?: { i: number; sim: number }[]
  }

  let canvas = $state<HTMLCanvasElement | null>(null)
  let w = $state(0)
  let h = $state(0)
  let nodes = $state.raw<Node[]>([])
  let xy: Float32Array = new Float32Array(0)
  let error = $state<string | null>(null)
  let picked = $state<number | null>(null)
  let hover = $state<number | null>(null)
  let members = $state<string[]>([])
  let minSim = $state(0.6)
  let zoom = $state(1)
  let cx = $state(0.5)
  let cy = $state(0.5)
  let rankOf: Map<number, number> = new Map()

  $effect(() => {
    const st = store?.id
    if (!st) return
    error = null
    const load = async () => {
      for (let tries = 0; tries < 20; tries++) {
        const r = await fetch(`/api/lab/${st}/sets`)
        if (r.ok) return r.json()
        if (r.status !== 503) throw new Error(await r.text())
        await new Promise((ok) => setTimeout(ok, 1500))
      }
      throw new Error('the store index did not finish building')
    }
    Promise.all([load(), atlas(st), meta(st)]).then(
      ([g, a]) => {
        nodes = g.sets
        xy = a
        const placed = nodes.map((n, i) => [n.first, i]).sort((p, q) => p[0] - q[0])
        rankOf = new Map(placed.map(([, i], r) => [i, r / Math.max(1, placed.length - 1)]))
        schedule()
      },
      (e) => (error = (e as Error).message),
    )
  })

  const colour = (f: number, a = 1) => `hsla(${Math.round(215 - f * 180)} 72% ${Math.round(50 + f * 10)}% / ${a})`
  const scale = () => Math.min(w, h) * 0.92 * zoom
  const sx = (x: number) => w / 2 + (x - cx) * scale()
  const sy = (y: number) => h / 2 + (y - cy) * scale()
  const radius = (n: Node) => Math.max(2.5, Math.sqrt(n.on_map) * 0.9) * Math.sqrt(zoom)

  let frame = 0
  function schedule() {
    if (!frame) frame = requestAnimationFrame(() => ((frame = 0), draw()))
  }
  $effect(() => {
    void w, h, picked, hover, minSim, zoom, cx, cy
    schedule()
  })

  function draw() {
    if (!canvas || !nodes.length) return
    const dpr = window.devicePixelRatio || 1
    canvas.width = Math.round(w * dpr)
    canvas.height = Math.round(h * dpr)
    const g = canvas.getContext('2d')!
    g.setTransform(dpr, 0, 0, dpr, 0, 0)
    g.fillStyle = '#121212'
    g.fillRect(0, 0, w, h)
    g.fillStyle = '#2b2b2b'
    for (let i = 0; i < xy.length; i += 2) g.fillRect(sx(xy[i]), sy(xy[i + 1]), 1, 1)
    // Lines first, under the circles.
    const focus = picked ?? hover
    nodes.forEach((n, i) => {
      if (n.x === null || !n.near) return
      for (const e of n.near) {
        if (e.sim < minSim) continue
        const m = nodes[e.i]
        if (m.x === null) continue
        const lit = focus === i || focus === e.i
        if (focus !== null && !lit) continue
        g.strokeStyle = lit ? `rgba(255,255,255,${0.25 + (e.sim - minSim) * 2})` : `rgba(200,200,200,${(e.sim - minSim) * 0.9})`
        g.lineWidth = lit ? 1.5 : 0.7
        g.beginPath()
        g.moveTo(sx(n.x), sy(n.y!))
        g.lineTo(sx(m.x), sy(m.y!))
        g.stroke()
      }
    })
    nodes.forEach((n, i) => {
      if (n.x === null) return
      const r = radius(n)
      g.fillStyle = colour(rankOf.get(i) ?? 0, focus === null || focus === i ? 0.85 : 0.35)
      g.beginPath()
      g.arc(sx(n.x), sy(n.y!), r, 0, Math.PI * 2)
      g.fill()
      if (i === focus) {
        g.strokeStyle = '#fff'
        g.lineWidth = 2
        g.stroke()
      }
    })
  }

  function at(e: PointerEvent | MouseEvent): number | null {
    const r = canvas!.getBoundingClientRect()
    const mx = e.clientX - r.left,
      my = e.clientY - r.top
    let best: number | null = null,
      bd = Infinity
    nodes.forEach((n, i) => {
      if (n.x === null) return
      const d = Math.hypot(sx(n.x) - mx, sy(n.y!) - my) - radius(n)
      if (d < bd) (bd = d), (best = i)
    })
    return bd < 6 ? best : null
  }

  let drag: { x: number; y: number; cx: number; cy: number; moved: boolean } | null = null
  function down(e: PointerEvent) {
    drag = { x: e.clientX, y: e.clientY, cx, cy, moved: false }
    canvas?.setPointerCapture(e.pointerId)
  }
  function move(e: PointerEvent) {
    if (drag) {
      const dx = e.clientX - drag.x,
        dy = e.clientY - drag.y
      if (Math.abs(dx) + Math.abs(dy) > 3) drag.moved = true
      cx = drag.cx - dx / scale()
      cy = drag.cy - dy / scale()
    } else hover = at(e)
  }
  function up(e: PointerEvent) {
    if (drag && !drag.moved) {
      const i = at(e)
      if (i !== null) choose(i)
      else picked = null
    }
    drag = null
  }
  function wheel(e: WheelEvent) {
    e.preventDefault()
    const r = canvas!.getBoundingClientRect()
    const mx = e.clientX - r.left,
      my = e.clientY - r.top
    const wx = cx + (mx - w / 2) / scale(),
      wy = cy + (my - h / 2) / scale()
    zoom = Math.max(0.5, Math.min(60, zoom * Math.exp(-e.deltaY * 0.0015)))
    cx = wx - (mx - w / 2) / scale()
    cy = wy - (my - h / 2) / scale()
  }

  async function choose(i: number) {
    picked = i
    members = []
    if (!store) return
    const im = await getImages(store.id, nodes[i].id)
    if (picked === i) members = im.ids.slice().reverse().slice(0, 40)
  }

  const placed = $derived(nodes.filter((n) => n.x !== null).length)
  const p = $derived(picked !== null ? nodes[picked] : null)
</script>

<div class="sets">
  <div class="map" bind:clientWidth={w} bind:clientHeight={h}>
    <canvas bind:this={canvas} style="width:{w}px;height:{h}px" onpointerdown={down} onpointermove={move} onpointerup={up} onwheel={wheel}></canvas>
    <div class="ctl">
      <b>Sets</b>
      {#if error}<span class="err">{error}</span>{:else if !nodes.length}<span>reading…</span>{/if}
      {#if nodes.length}
        <span>{count(placed)} of {count(nodes.length)} sets have images with embeddings and are drawn; the rest wait for Pan's embed stage. Each sits at the middle of its images on the Atlas.</span>
        <label>join sets at least <input type="range" min="0.4" max="0.95" step="0.01" bind:value={minSim} /> {minSim.toFixed(2)} alike</label>
      {/if}
    </div>
    {#if hover !== null && hover !== picked && nodes[hover]}
      {@const n = nodes[hover]}
      <div class="tip">
        {#if n.cover}<img src={thumbUrl(n.cover)} alt="" />{/if}
        <span>{n.id}</span>
        <span class="dim">{count(n.count)} images · {day(n.first)}</span>
      </div>
    {/if}
  </div>
  <aside>
    {#if p && picked !== null}
      <h3>{p.id}</h3>
      <p class="dim">{count(p.count)} images, {count(p.on_map)} on the map · {when(p.first)} → {when(p.last)}</p>
      <p><button onclick={() => onset(p.id)}>open the set in the Library</button></p>
      <div class="grid">
        {#each members as id (id)}<Pic {id} size={64} {onopen} />{/each}
      </div>
      <h4>looks most like</h4>
      <ul>
        {#each p.near ?? [] as e (e.i)}
          <li><button onclick={() => choose(e.i)}><span class="sim">{e.sim.toFixed(2)}</span> {nodes[e.i].id}</button></li>
        {/each}
      </ul>
    {:else}
      <p class="dim">Hover a circle to see the set; click to open it here. Lines join sets whose images, on average, look alike. Drag to move, scroll to zoom.</p>
    {/if}
  </aside>
</div>

<style>
  .sets { position: absolute; inset: 0; display: grid; grid-template-columns: 1fr 22rem; background: #121212; color: #bbb; }
  .map { position: relative; min-width: 0; overflow: hidden; }
  canvas { display: block; cursor: pointer; touch-action: none; }
  .ctl { position: absolute; top: 0.6rem; left: 0.8rem; display: flex; flex-direction: column; gap: 0.35rem; font-size: 0.72rem; background: #000b; padding: 0.5rem 0.7rem; max-width: 30rem; }
  .ctl b { font-family: var(--display); font-size: 1rem; font-weight: normal; color: #eee; }
  label { display: flex; gap: 0.4rem; align-items: center; }
  .err { color: var(--warn); }
  .tip { position: absolute; bottom: 0.8rem; left: 0.8rem; display: flex; gap: 0.6rem; align-items: center; background: #000c; padding: 0.4rem; font-size: 0.72rem; max-width: 40rem; }
  .tip img { width: 64px; height: 64px; object-fit: cover; }
  aside { border-left: 1px solid #000; overflow-y: auto; padding: 0.8rem; background: #181818; }
  h3 { font-family: var(--mono); font-size: 0.78rem; color: #eee; overflow-wrap: anywhere; margin: 0 0 0.3rem; font-weight: normal; }
  h4 { font-size: 0.7rem; letter-spacing: 0.1em; text-transform: uppercase; font-weight: normal; color: #ddd; margin: 1rem 0 0.4rem; }
  .dim { color: #777; font-size: 0.72rem; }
  aside button { font-size: 0.72rem; background: #222; color: #ddd; border: 1px solid #555; padding: 0.15rem 0.6rem; }
  ul { list-style: none; margin: 0; padding: 0; }
  li button { width: 100%; text-align: left; border: 0 !important; background: none !important; padding: 0.2rem 0 !important; overflow-wrap: anywhere; font-family: var(--mono); }
  li button:hover { color: #fff; }
  .sim { color: #888; margin-right: 0.3rem; }
  .grid { display: flex; flex-wrap: wrap; gap: 4px; margin-top: 0.5rem; }
</style>
