<script lang="ts">
  // Daydream: the joystick with nobody holding it. Every few seconds it
  // takes one step in a random direction through the store, by look, and
  // the picture dissolves into the next. Now and then it leaps somewhere
  // else entirely. Put it on a second screen and let the store remind you
  // what is in it.
  import type { LabProps } from './index'
  import { meta, stick } from './data'
  import { thumbUrl, mediaUrl } from '../api'

  let { store, image, onopen }: LabProps = $props()

  let layers = $state<{ id: string; key: number }[]>([])
  let seconds = $state(4)
  let leap = $state(0.08)
  let paused = $state(false)
  let error = $state<string | null>(null)
  let ids: string[] = []
  let visited: string[] = []
  let current: string | null = null
  let key = 0
  let timer: ReturnType<typeof setTimeout> | undefined

  $effect(() => {
    const st = store?.id
    if (!st) return
    error = null
    meta(st).then(
      (m) => {
        ids = m.ids
        visited = []
        show(image && m.pos.has(image) ? image : ids[Math.floor(Math.random() * ids.length)])
        schedule()
      },
      (e) => (error = (e as Error).message),
    )
    return () => clearTimeout(timer)
  })

  function show(id: string) {
    current = id
    visited = [...visited, id].slice(-300)
    layers = [...layers, { id, key: ++key }].slice(-2)
  }

  async function step() {
    if (!store || !current) return
    if (Math.random() < leap) return show(ids[Math.floor(Math.random() * ids.length)])
    try {
      const r = await stick(store.id, current, 0.5 + Math.random() * 0.6, 0.4, visited)
      const d = r.dirs[Math.floor(Math.random() * r.dirs.length)]
      if (d) show(d.id)
    } catch (e) {
      error = (e as Error).message
    }
  }

  function schedule() {
    clearTimeout(timer)
    timer = setTimeout(async () => {
      if (!paused) await step()
      schedule()
    }, seconds * 1000)
  }

  function onkey(e: KeyboardEvent) {
    if ((e.target as HTMLElement)?.tagName === 'INPUT') return
    if (e.code === 'Space') paused = !paused
    else if (e.code === 'ArrowRight') step()
    else if (e.code === 'Enter' && current) onopen(current)
    else return
    e.preventDefault()
  }
</script>

<svelte:window onkeydown={onkey} />

<div class="dream">
  {#each layers as l (l.key)}
    <img class="layer" src={mediaUrl(l.id)} alt="" style="background-image:url({thumbUrl(l.id)})" />
  {/each}
  <div class="ctl">
    <b>Daydream</b>
    <label>every <input type="range" min="1.5" max="15" step="0.5" bind:value={seconds} onchange={schedule} /> {seconds}s</label>
    <label>leaps <input type="range" min="0" max="0.5" step="0.01" bind:value={leap} /> {Math.round(leap * 100)}%</label>
    <span>Space {paused ? 'go on' : 'pause'} · → next · Enter open in Library</span>
    {#if error}<span class="err">{error}</span>{/if}
  </div>
</div>

<style>
  .dream { position: absolute; inset: 0; background: #000; overflow: hidden; }
  .layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: contain;
    background-size: contain;
    background-position: center;
    background-repeat: no-repeat;
    animation: in 1.6s ease both;
  }
  @keyframes in {
    from { opacity: 0; transform: scale(1.015); }
    to { opacity: 1; transform: scale(1); }
  }
  .ctl {
    position: absolute;
    left: 0.8rem;
    bottom: 0.8rem;
    display: flex;
    gap: 1rem;
    align-items: center;
    font-size: 0.7rem;
    color: #888;
    background: #000a;
    padding: 0.35rem 0.7rem;
    opacity: 0.25;
    transition: opacity 0.3s;
  }
  .ctl:hover { opacity: 1; }
  .ctl b { font-family: var(--display); font-weight: normal; color: #ddd; font-size: 0.9rem; }
  label { display: inline-flex; gap: 0.3rem; align-items: center; }
  input[type='range'] { width: 6rem; }
  .err { color: var(--warn); }
</style>
