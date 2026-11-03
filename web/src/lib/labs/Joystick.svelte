<script lang="ts">
  // The joystick, after CoPIA's latent navigator: the image in the middle,
  // and around it one step in each of eight directions. Up/down and
  // left/right are the two directions in which the images near this one
  // differ most, worked out afresh at every step, so the axes mean
  // something different in every neighbourhood. Steer and see what the
  // store holds that you would never have scrolled to.
  import type { LabProps } from './index'
  import { meta, stick, type Step } from './data'
  import { thumbUrl } from '../api'
  import Pic from './Pic.svelte'

  let { store, image, onopen }: LabProps = $props()

  let centre = $state<string | null>(null)
  let steps = $state<Step[]>([])
  let trail = $state<string[]>([])
  let stride = $state(0.6)
  let fuzz = $state(0.15)
  let error = $state<string | null>(null)
  let busy = $state(false)
  let ids: string[] = []

  // Q W E / A · D / Z X C, and the arrows for the four sides.
  const KEYS: Record<string, [number, number]> = {
    KeyW: [0, -1], KeyE: [1, -1], KeyD: [1, 0], KeyC: [1, 1],
    KeyX: [0, 1], KeyZ: [-1, 1], KeyA: [-1, 0], KeyQ: [-1, -1],
    ArrowUp: [0, -1], ArrowRight: [1, 0], ArrowDown: [0, 1], ArrowLeft: [-1, 0],
  }
  const LABEL: Record<string, string> = { '0,-1': 'W', '1,-1': 'E', '1,0': 'D', '1,1': 'C', '0,1': 'X', '-1,1': 'Z', '-1,0': 'A', '-1,-1': 'Q' }

  $effect(() => {
    const st = store?.id
    if (!st) return
    error = null
    meta(st).then(
      (m) => {
        ids = m.ids
        const start = image && m.pos.has(image) ? image : ids[Math.floor(Math.random() * ids.length)]
        trail = []
        go(start)
      },
      (e) => (error = (e as Error).message),
    )
  })

  let seq = 0
  async function go(id: string, record = true) {
    if (!store) return
    if (record && centre) trail = [...trail, centre].slice(-400)
    centre = id
    busy = true
    const my = ++seq
    try {
      const r = await stick(store.id, id, stride, fuzz, trail)
      if (my === seq) steps = r.dirs
    } catch (e) {
      if (my === seq) error = (e as Error).message
    } finally {
      if (my === seq) busy = false
    }
  }

  function at(dx: number, dy: number) {
    return steps.find((s) => s.dx === dx && s.dy === dy) ?? null
  }

  function back() {
    const prev = trail.at(-1)
    if (!prev) return
    trail = trail.slice(0, -1)
    go(prev, false)
  }

  function jump() {
    if (ids.length) go(ids[Math.floor(Math.random() * ids.length)])
  }

  function onkey(e: KeyboardEvent) {
    const t = e.target as HTMLElement
    if (t?.tagName === 'INPUT' || e.metaKey || e.ctrlKey || e.altKey) return
    const d = KEYS[e.code]
    if (d) {
      const s = at(d[0], d[1])
      if (s) go(s.id)
    } else if (e.code === 'Space' || e.code === 'KeyR') jump()
    else if (e.code === 'Backspace') back()
    else if (e.code === 'Enter' && centre) onopen(centre)
    else return
    e.preventDefault()
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="joy">
  <header>
    <b>Joystick</b>
    <label>stride <input type="range" min="0.1" max="2" step="0.05" bind:value={stride} onchange={() => centre && go(centre, false)} /> {stride.toFixed(2)}</label>
    <label>fuzz <input type="range" min="0" max="1" step="0.05" bind:value={fuzz} /> {fuzz.toFixed(2)}</label>
    <span class="help">Q W E · A D · Z X C or the arrows to move · Space jump anywhere · Backspace back · Enter open in Library</span>
    {#if busy}<span class="busy">…</span>{/if}
  </header>
  {#if error}
    <p class="err">{error}</p>
  {:else}
    <div class="pad">
      {#each [-1, 0, 1] as dy (dy)}
        {#each [-1, 0, 1] as dx (dx)}
          {#if dx === 0 && dy === 0}
            <div class="centre">
              {#if centre}<img src={thumbUrl(centre)} alt="" />{/if}
            </div>
          {:else}
            {@const s = at(dx, dy)}
            <button class="dir" style="grid-column:{dx + 2};grid-row:{dy + 2}" onclick={() => s && go(s.id)} disabled={!s}>
              {#if s}<img src={thumbUrl(s.id)} alt="" />{/if}
              <span class="key">{LABEL[`${dx},${dy}`]}</span>
              {#if s}<span class="sim">{s.sim.toFixed(2)}</span>{/if}
            </button>
          {/if}
        {/each}
      {/each}
    </div>
    <div class="trail">
      {#each trail.slice(-40) as id, i (i + id)}
        <Pic {id} size={52} onclick={(x) => go(x)} {onopen} />
      {/each}
      <span class="count">{trail.length} steps</span>
    </div>
  {/if}
</div>

<style>
  .joy { position: absolute; inset: 0; display: flex; flex-direction: column; background: var(--stage); color: var(--stage-soft); }
  header { display: flex; gap: 1.2rem; align-items: center; padding: 0.4rem 0.9rem; font-size: 0.72rem; border-bottom: 1px solid #000; flex-wrap: wrap; }
  header b { color: var(--stage-ink); font-family: var(--display); font-size: 0.95rem; font-weight: normal; }
  label { display: inline-flex; gap: 0.4rem; align-items: center; }
  input[type='range'] { width: 7rem; }
  .help { color: #777; }
  .busy { color: var(--stage-ink); }
  .err { padding: 2rem; color: var(--warn); white-space: pre-wrap; }
  .pad {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 1fr 3fr 1fr;
    grid-template-rows: 1fr 3fr 1fr;
    gap: 6px;
    padding: 8px;
  }
  .centre { grid-column: 2; grid-row: 2; min-height: 0; display: flex; align-items: center; justify-content: center; }
  .centre img { max-width: 100%; max-height: 100%; object-fit: contain; }
  .dir { position: relative; min-height: 0; padding: 0; border: 0; background: var(--stage-cell); overflow: hidden; }
  .dir:hover:not(:disabled) { outline: 2px solid var(--stage-ink); background: var(--stage-cell); }
  .dir img { width: 100%; height: 100%; object-fit: contain; display: block; }
  .key { position: absolute; top: 4px; left: 4px; font-family: var(--mono); font-size: 0.7rem; color: #000; background: var(--stage-ink); padding: 0 0.3rem; }
  .sim { position: absolute; bottom: 3px; right: 4px; font-size: 0.62rem; color: #aaa; background: #000a; padding: 0 0.25rem; }
  .trail { display: flex; gap: 4px; align-items: center; padding: 4px 8px; border-top: 1px solid #000; overflow-x: auto; height: 76px; flex-shrink: 0; }
  .count { font-size: 0.68rem; color: #777; margin-left: 0.5rem; white-space: nowrap; }
</style>
