<script lang="ts">
  // The Mixer: a recipe of images. Add images to "more like" and "less
  // like", weight them, and the store answers with what sits at that point
  // in the space between them. Click a result to add it to the recipe;
  // Shift-click to add it as something to move away from.
  //
  // Scope, after W4R3Z's suggestion: images cluster hard by the set they
  // were made in, so "outside" leaves the first image's own sets and shows
  // what it resembles anywhere else, and "both" puts the two side by side.
  import type { LabProps } from './index'
  import { meta, near, type Hit } from './data'
  import Pic from './Pic.svelte'

  let { store, image, onopen }: LabProps = $props()

  let plus = $state<{ id: string; w: number }[]>([])
  let minus = $state<string[]>([])
  let scope = $state<'all' | 'inside' | 'outside' | 'both'>('all')
  let wander = $state(0)
  let hits = $state<Hit[]>([])
  let inside = $state<Hit[]>([])
  let outside = $state<Hit[]>([])
  let error = $state<string | null>(null)
  let ms = $state<number | null>(null)

  $effect(() => {
    const st = store?.id
    if (!st) return
    meta(st).then((m) => {
      const start = image && m.pos.has(image) ? image : m.ids[Math.floor(Math.random() * m.ids.length)]
      plus = [{ id: start, w: 1 }]
      minus = []
    })
  })

  let seq = 0
  $effect(() => {
    const st = store?.id
    const p = plus.map((x) => x.id)
    const pw = plus.map((x) => x.w)
    const m = [...minus]
    const sc = scope
    const j = wander
    if (!st || !p.length) return
    const my = ++seq
    const t0 = performance.now()
    const ask = (s: 'all' | 'inside' | 'outside') => near(st, { plus: p, plus_w: pw, minus: m, k: 60, scope: s, jitter: j })
    const go = sc === 'both' ? Promise.all([ask('inside'), ask('outside')]) : ask(sc).then((r) => [r, r])
    go.then(
      ([a, b]) => {
        if (my !== seq) return
        error = null
        ms = performance.now() - t0
        if (sc === 'both') (inside = a.hits), (outside = b.hits)
        else hits = a.hits
      },
      (e) => my === seq && (error = (e as Error).message),
    )
  })

  function add(id: string, e?: MouseEvent) {
    if (e?.shiftKey) {
      if (!minus.includes(id)) minus = [...minus, id]
    } else if (!plus.some((x) => x.id === id)) plus = [...plus, { id, w: 1 }]
  }
</script>

<div class="mix">
  <header>
    <b>Mixer</b>
    <span class="seg">
      {#each [['all', 'everywhere'], ['inside', 'inside its sets'], ['outside', 'outside its sets'], ['both', 'inside | outside']] as [v, label] (v)}
        <button class:on={scope === v} onclick={() => (scope = v as typeof scope)}>{label}</button>
      {/each}
    </span>
    <label>wander <input type="range" min="0" max="1.5" step="0.05" bind:value={wander} /> {wander.toFixed(2)}</label>
    <span class="help">click a result: more like it · Shift-click: less like it · double-click: open in the Library</span>
    {#if ms !== null}<span class="dim">{Math.round(ms)} ms</span>{/if}
  </header>
  <div class="recipe">
    <div class="side">
      <h4>more like</h4>
      <div class="row">
        {#each plus as p, i (p.id)}
          <div class="slot">
            <Pic id={p.id} size={92} {onopen} onclick={() => (plus = plus.filter((_, j) => j !== i))} />
            <input type="range" min="0" max="3" step="0.1" bind:value={p.w} title="weight" />
            <span class="w">× {p.w.toFixed(1)} · click to remove</span>
          </div>
        {/each}
      </div>
    </div>
    <div class="side minus">
      <h4>less like</h4>
      <div class="row">
        {#each minus as id (id)}
          <div class="slot"><Pic {id} size={92} {onopen} onclick={() => (minus = minus.filter((x) => x !== id))} /><span class="w">click to remove</span></div>
        {/each}
        {#if !minus.length}<span class="dim">Shift-click a result to add one</span>{/if}
      </div>
    </div>
  </div>
  {#if error}<p class="err">{error}</p>{/if}
  {#if scope === 'both'}
    <div class="split">
      <section><h4>inside its own sets</h4><div class="grid">{#each inside as h (h.id)}<Pic id={h.id} size={104} note={h.sim.toFixed(2)} onclick={add} {onopen} />{/each}</div></section>
      <section><h4>outside them: what it resembles elsewhere</h4><div class="grid">{#each outside as h (h.id)}<Pic id={h.id} size={104} note={h.sim.toFixed(2)} onclick={add} {onopen} />{/each}</div></section>
    </div>
  {:else}
    <section class="all"><div class="grid">{#each hits as h (h.id)}<Pic id={h.id} size={128} note={h.sim.toFixed(2)} onclick={add} {onopen} />{/each}</div></section>
  {/if}
</div>

<style>
  .mix { position: absolute; inset: 0; display: flex; flex-direction: column; background: var(--stage); color: var(--stage-soft); }
  header { display: flex; gap: 1rem; align-items: center; padding: 0.45rem 0.9rem; border-bottom: 1px solid #000; font-size: 0.72rem; flex-wrap: wrap; }
  header b { color: var(--stage-ink); font-family: var(--display); font-size: 0.95rem; font-weight: normal; }
  .seg { display: inline-flex; }
  .seg button { font-size: 0.7rem; background: #222; color: #aaa; border: 1px solid #444; padding: 0.1rem 0.5rem; margin-left: -1px; }
  .seg button.on { background: #ddd; color: #111; }
  label { display: inline-flex; gap: 0.4rem; align-items: center; }
  .help, .dim { color: #777; }
  .recipe { display: grid; grid-template-columns: 2fr 1fr; border-bottom: 1px solid #000; }
  .side { padding: 0.5rem 0.9rem; min-width: 0; }
  .side.minus { border-left: 1px solid #000; background: #1a1414; }
  .row { display: flex; gap: 8px; overflow-x: auto; align-items: flex-start; min-height: 100px; }
  .slot { display: flex; flex-direction: column; gap: 2px; font-size: 0.65rem; }
  .slot input { width: 92px; }
  .w { color: #777; }
  h4 { font-size: 0.7rem; letter-spacing: 0.1em; text-transform: uppercase; font-weight: normal; color: #ddd; margin: 0 0 0.4rem; }
  .err { color: var(--warn); padding: 0.4rem 0.9rem; margin: 0; }
  .split { flex: 1; min-height: 0; display: grid; grid-template-columns: 1fr 1fr; }
  section { overflow-y: auto; padding: 0.6rem 0.9rem; min-height: 0; }
  .split section + section { border-left: 1px solid #000; }
  section.all { flex: 1; }
  .grid { display: flex; flex-wrap: wrap; gap: 6px; }
</style>
