<script lang="ts">
  // Pose: drag a stick figure, and the store answers with the bodies that
  // stand that way. Each image's largest person was found by Pan's pose
  // stage (17 body joints); tools/lab_build.py centred every pose on the
  // hips and scaled it by the torso, so size and place in the frame do not
  // count, only the shape. The search runs in the page, over every pose,
  // as you drag.
  import type { LabProps } from './index'
  import { poses } from './data'
  import { count } from '../format'
  import Pic from './Pic.svelte'

  let { store, image, onopen }: LabProps = $props()

  const J = 17
  // COCO body joints: 0 nose, 1-2 eyes, 3-4 ears, 5-6 shoulders, 7-8 elbows,
  // 9-10 wrists, 11-12 hips, 13-14 knees, 15-16 ankles (left, right).
  const BONES: [number, number][] = [
    [5, 7], [7, 9], [6, 8], [8, 10], [5, 6], [5, 11], [6, 12], [11, 12],
    [11, 13], [13, 15], [12, 14], [14, 16], [0, 1], [0, 2], [1, 3], [2, 4],
  ]
  const NAMES = ['nose', 'left eye', 'right eye', 'left ear', 'right ear', 'left shoulder', 'right shoulder', 'left elbow', 'right elbow', 'left wrist', 'right wrist', 'left hip', 'right hip', 'left knee', 'right knee', 'left ankle', 'right ankle']
  // The face counts for less than the body: five face points would
  // otherwise outvote two arms.
  const WEIGHT = Array.from({ length: J }, (_, j) => (j < 5 ? 0.25 : 1))
  // Pose detectors give "left" and "right" from the person's side, so a
  // mirrored body swaps them.
  const MIRROR = [0, 2, 1, 4, 3, 6, 5, 8, 7, 10, 9, 12, 11, 14, 13, 16, 15]
  // A joint counts as seen when the detector scored it above this. Below
  // it are guesses for joints out of frame or hidden, and the guesses are
  // what turn a screenshot of a terminal into a "person".
  const SEEN = 3
  // The drawing area, in torso lengths from the hips; joints stay inside.
  const BOX = { x: -1.9, y: -2.7, w: 3.8, h: 4.3 }

  let data = $state.raw<{ xyc: Float32Array; ids: string[]; people: number[] } | null>(null)
  let error = $state<string | null>(null)
  let fig = $state<[number, number][]>([])
  let hits = $state<{ i: number; d: number }[]>([])
  let mirror = $state(true)
  let solo = $state(false)
  let ms = $state(0)
  let dragging = $state<number | null>(null)
  // The joints you have moved count three times over: they are what you
  // are asking about, and the rest is just a body to hang them on.
  let touched = $state<Set<number>>(new Set())
  let svg = $state<SVGSVGElement | null>(null)

  $effect(() => {
    const st = store?.id
    if (!st) return
    error = null
    poses(st).then(
      (p) => {
        data = p
        const k = image ? p.ids.indexOf(image) : -1
        fig = k >= 0 ? figOf(k) : average(p.xyc)
        search()
      },
      (e) => (error = (e as Error).message),
    )
  })

  /** The usual body: the median of each joint over every pose. */
  function average(xyc: Float32Array): [number, number][] {
    const n = xyc.length / (J * 3)
    const out: [number, number][] = []
    for (let j = 0; j < J; j++) {
      const xs: number[] = [],
        ys: number[] = []
      for (let i = 0; i < n; i += 7) {
        const b = (i * J + j) * 3
        if (xyc[b + 2] > SEEN) xs.push(xyc[b]), ys.push(xyc[b + 1])
      }
      xs.sort((a, b) => a - b)
      ys.sort((a, b) => a - b)
      out.push([xs[xs.length >> 1] ?? 0, ys[ys.length >> 1] ?? 0])
    }
    return out
  }

  function figOf(i: number): [number, number][] {
    const x = data!.xyc
    const fallback = average(x)
    return Array.from({ length: J }, (_, j) => {
      const b = (i * J + j) * 3
      return x[b + 2] > SEEN ? [x[b], x[b + 1]] : fallback[j]
    })
  }

  function search() {
    if (!data) return
    const t0 = performance.now()
    const x = data.xyc
    const n = data.ids.length
    const best: { i: number; d: number }[] = []
    let worst = Infinity
    const fx = fig.map((p) => p[0]),
      fy = fig.map((p) => p[1])
    const W = WEIGHT.map((w, j) => (touched.has(j) ? 3 : w))
    for (let i = 0; i < n; i++) {
      if (solo && data.people[i] !== 1) continue
      const base = i * J * 3
      let d = 0,
        dm = 0,
        w = 0,
        seen = 0
      for (let j = 0; j < J; j++) {
        const b = base + j * 3
        if (x[b + 2] <= SEEN) {
          // A joint you moved that this image does not show is a miss, not
          // a pass: otherwise a pose with its arms out of sight would match
          // any arms you draw.
          if (W[j] > 1) (d += W[j]), (dm += W[j]), (w += W[j])
          continue
        }
        const ex = x[b] - fx[j],
          ey = x[b + 1] - fy[j]
        d += W[j] * (ex * ex + ey * ey)
        if (mirror) {
          const m = MIRROR[j]
          const mx = -x[b] - fx[m],
            my = x[b + 1] - fy[m]
          dm += W[m] * (mx * mx + my * my)
        }
        w += W[j]
        seen++
      }
      if (seen < 8) continue // too few joints seen to say
      let score = (mirror ? Math.min(d, dm) : d) / w
      // A pose missing joints the figure has is less of a match.
      score *= 1 + (J - seen) * 0.05
      if (best.length < 60 || score < worst) {
        best.push({ i, d: score })
        if (best.length > 60) {
          best.sort((a, b) => a.d - b.d)
          best.length = 60
        }
        worst = best.length >= 60 ? best.reduce((m, b) => Math.max(m, b.d), 0) : Infinity
      }
    }
    best.sort((a, b) => a.d - b.d)
    hits = best
    ms = performance.now() - t0
  }

  let frame = 0
  function later() {
    if (!frame) frame = requestAnimationFrame(() => ((frame = 0), search()))
  }
  $effect(() => {
    void mirror, solo
    later()
  })

  function toWorld(e: PointerEvent): [number, number] {
    const pt = svg!.createSVGPoint()
    pt.x = e.clientX
    pt.y = e.clientY
    const p = pt.matrixTransform(svg!.getScreenCTM()!.inverse())
    return [p.x, p.y]
  }
  function grab(j: number, e: PointerEvent) {
    dragging = j
    touched = new Set([...touched, j])
    ;(e.target as Element).setPointerCapture(e.pointerId)
  }
  function drag(e: PointerEvent) {
    if (dragging === null) return
    const [px, py] = toWorld(e)
    fig[dragging] = [Math.max(BOX.x, Math.min(BOX.x + BOX.w, px)), Math.max(BOX.y, Math.min(BOX.y + BOX.h, py))]
    fig = [...fig]
    later()
  }

  function use(i: number) {
    fig = figOf(i)
    touched = new Set()
    later()
  }
  function flip() {
    fig = MIRROR.map((m) => [-fig[m][0], fig[m][1]] as [number, number])
    later()
  }
  function reset() {
    if (data) fig = average(data.xyc)
    touched = new Set()
    later()
  }
</script>

<div class="pose">
  <div class="left">
    <header>
      <b>Pose</b>
      <span>drag the joints</span>
    </header>
    {#if error}<p class="err">{error}</p>{/if}
    <svg bind:this={svg} viewBox="{BOX.x} {BOX.y} {BOX.w} {BOX.h}" onpointermove={drag} onpointerup={() => (dragging = null)} role="application">
      <rect class="box" x={BOX.x} y={BOX.y} width={BOX.w} height={BOX.h} />
      <line class="ground" x1={BOX.x} x2={BOX.x + BOX.w} y1="0" y2="0" />
      {#each BONES as [a, b] (a + '-' + b)}
        {#if fig[a] && fig[b]}<line class="bone" class:face={a < 5 && b < 5} x1={fig[a][0]} y1={fig[a][1]} x2={fig[b][0]} y2={fig[b][1]} />{/if}
      {/each}
      {#if fig[5] && fig[6] && fig[0]}
        <line class="bone face" x1={(fig[5][0] + fig[6][0]) / 2} y1={(fig[5][1] + fig[6][1]) / 2} x2={fig[0][0]} y2={fig[0][1]} />
      {/if}
      {#each fig as p, j (j)}
        <circle
          class="joint"
          class:left={j % 2 === 1 && j > 0}
          class:on={dragging === j}
          class:moved={touched.has(j)}
          cx={p[0]}
          cy={p[1]}
          r={j < 5 ? 0.07 : 0.11}
          onpointerdown={(e) => grab(j, e)}
          role="slider"
          tabindex="-1"
          aria-label={NAMES[j]}
          aria-valuenow={0}
        ><title>{NAMES[j]}</title></circle>
      {/each}
    </svg>
    <div class="opts">
      <button onclick={reset}>usual body</button>
      <button onclick={flip}>flip</button>
      <label><input type="checkbox" bind:checked={mirror} /> match mirrored too</label>
      <label><input type="checkbox" bind:checked={solo} /> one person only</label>
    </div>
    <p class="note">
      {#if data}{count(data.ids.length)} poses searched in {Math.round(ms)} ms, in this page.{/if}
      Blue joints are the person's left. Joints you move (ringed white) count three times over. Click a result to take its pose, then drag from there.
    </p>
  </div>
  <section>
    <div class="grid">
      {#each hits as h (h.i)}
        <Pic id={data!.ids[h.i]} size={128} note={data!.people[h.i] > 1 ? `${data!.people[h.i]} people` : null} onclick={() => use(h.i)} {onopen} />
      {/each}
    </div>
  </section>
</div>

<style>
  .pose { position: absolute; inset: 0; display: grid; grid-template-columns: 28rem 1fr; background: var(--stage); color: var(--stage-soft); }
  .left { display: flex; flex-direction: column; border-right: 1px solid #000; min-height: 0; background: #181818; }
  header { display: flex; gap: 1rem; align-items: baseline; padding: 0.45rem 0.9rem; font-size: 0.72rem; border-bottom: 1px solid #000; }
  header b { color: var(--stage-ink); font-family: var(--display); font-size: 0.95rem; font-weight: normal; }
  svg { flex: 1; min-height: 0; width: 100%; touch-action: none; user-select: none; }
  .box { fill: none; stroke: #2a2a2a; stroke-width: 0.01; }
  .ground { stroke: #333; stroke-width: 0.01; stroke-dasharray: 0.05 0.05; }
  .bone { stroke: #ddd; stroke-width: 0.06; stroke-linecap: round; }
  .bone.face { stroke: #777; stroke-width: 0.03; }
  .joint { fill: #f0a050; stroke: #111; stroke-width: 0.02; cursor: grab; outline: none; }
  .joint.left { fill: #5aa0f0; }
  .joint.moved { stroke: #fff; stroke-width: 0.03; }
  .joint.on { fill: #fff; cursor: grabbing; }
  .opts { display: flex; flex-wrap: wrap; gap: 0.5rem; padding: 0.5rem 0.9rem; font-size: 0.72rem; align-items: center; }
  .opts button { font-size: 0.72rem; background: #222; color: #ddd; border: 1px solid #555; padding: 0.1rem 0.5rem; }
  label { display: inline-flex; gap: 0.3rem; align-items: center; }
  .note { font-size: 0.68rem; color: #777; padding: 0 0.9rem 0.6rem; margin: 0; line-height: 1.5; }
  .err { color: var(--warn); padding: 0.5rem 0.9rem; }
  section { overflow-y: auto; padding: 0.7rem; }
  .grid { display: flex; flex-wrap: wrap; gap: 6px; }
</style>
