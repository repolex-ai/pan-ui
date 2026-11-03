<script lang="ts">
  // Echoes: pairs of images that look almost alike but were made weeks
  // apart. Each image appears in at most one pair. It is the R key's idea
  // turned into a feed: what you made before, found by what you made since.
  import type { LabProps } from './index'
  import { meta, echoes, type Echo } from './data'
  import { thumbUrl } from '../api'
  import { when, count } from '../format'

  let { store, onopen }: LabProps = $props()

  let ids = $state.raw<string[]>([])
  let ts = $state.raw<number[]>([])
  let pairs = $state.raw<Echo[]>([])
  let gapDays = $state(21)
  let order = $state<'strongest' | 'farthest' | 'shuffle'>('shuffle')
  let shown = $state(30)
  let error = $state<string | null>(null)
  let seed = $state(1)
  // Most pairs are only loosely alike; the strong ones are the echoes.
  let floor = $state(0.6)

  $effect(() => {
    const st = store?.id
    if (!st) return
    error = null
    Promise.all([meta(st), echoes(st)]).then(
      ([m, e]) => {
        ids = m.ids
        ts = m.t
        pairs = e.pairs
        gapDays = e.gap_days
        shown = 30
      },
      (e) => (error = (e as Error).message),
    )
  })

  const days = (p: Echo) => Math.round(Math.abs(ts[p.a] - ts[p.b]) / 86400)
  const list = $derived.by(() => {
    void seed
    const xs = pairs.filter((p) => p.sim >= floor)
    if (order === 'strongest') xs.sort((a, b) => b.sim - a.sim)
    else if (order === 'farthest') xs.sort((a, b) => days(b) - days(a))
    else
      for (let i = xs.length - 1; i > 0; i--) {
        const j = Math.floor(Math.random() * (i + 1))
        ;[xs[i], xs[j]] = [xs[j], xs[i]]
      }
    return xs
  })
  // Older on the left: then, and now.
  const thenNow = (p: Echo) => (ts[p.a] <= ts[p.b] ? [p.a, p.b] : [p.b, p.a])
</script>

<div class="echo">
  <header>
    <b>Echoes</b>
    <span>{count(list.length)} of {count(pairs.length)} pairs made at least {gapDays} days apart; each image in one pair at most</span>
    <label>at least <input type="range" min="0.3" max="0.95" step="0.01" bind:value={floor} /> {floor.toFixed(2)} alike</label>
    <span class="seg">
      <button class:on={order === 'shuffle'} onclick={() => ((order = 'shuffle'), seed++)}>shuffle</button>
      <button class:on={order === 'strongest'} onclick={() => (order = 'strongest')}>most alike</button>
      <button class:on={order === 'farthest'} onclick={() => (order = 'farthest')}>farthest apart</button>
    </span>
  </header>
  {#if error}<p class="err">{error}</p>{/if}
  <div class="feed">
    {#each list.slice(0, shown) as p (p.a + ':' + p.b)}
      {@const [a, b] = thenNow(p)}
      <div class="pair">
        <button class="img" ondblclick={() => onopen(ids[a])} title="double-click to open in the Library">
          <img src={thumbUrl(ids[a])} alt="" loading="lazy" />
          <span class="d">then · {when(ts[a]).slice(0, 10)}</span>
        </button>
        <div class="mid">
          <span class="gap">{days(p)}</span>
          <span class="unit">days</span>
          <span class="sim">{p.sim.toFixed(2)} alike</span>
        </div>
        <button class="img" ondblclick={() => onopen(ids[b])} title="double-click to open in the Library">
          <img src={thumbUrl(ids[b])} alt="" loading="lazy" />
          <span class="d">now · {when(ts[b]).slice(0, 10)}</span>
        </button>
      </div>
    {/each}
    {#if list.length > shown}
      <button class="more" onclick={() => (shown += 30)}>{count(list.length - shown)} more</button>
    {/if}
  </div>
</div>

<style>
  .echo { position: absolute; inset: 0; display: flex; flex-direction: column; background: var(--stage); color: var(--stage-soft); }
  header { display: flex; gap: 1rem; align-items: center; padding: 0.45rem 0.9rem; border-bottom: 1px solid #000; font-size: 0.72rem; flex-wrap: wrap; }
  header b { color: var(--stage-ink); font-family: var(--display); font-size: 0.95rem; font-weight: normal; }
  .seg { display: inline-flex; }
  label { display: inline-flex; gap: 0.4rem; align-items: center; }
  .seg button { font-size: 0.7rem; background: #222; color: #aaa; border: 1px solid #444; padding: 0.1rem 0.5rem; margin-left: -1px; }
  .seg button.on { background: #ddd; color: #111; }
  .err { color: var(--warn); padding: 0.5rem 0.9rem; margin: 0; }
  .feed { flex: 1; overflow-y: auto; padding: 1rem; display: grid; grid-template-columns: repeat(auto-fill, minmax(36rem, 1fr)); gap: 1.4rem 2rem; align-content: start; }
  .pair { display: grid; grid-template-columns: 1fr 4.5rem 1fr; align-items: center; }
  .img { position: relative; padding: 0; border: 0; background: var(--stage-cell); height: 15rem; }
  .img:hover { background: var(--stage-cell); outline: 1px solid #666; }
  .img img { width: 100%; height: 100%; object-fit: contain; display: block; }
  .d { position: absolute; left: 4px; bottom: 4px; font-size: 0.65rem; background: #000b; color: #ccc; padding: 0 0.3rem; }
  .mid { display: flex; flex-direction: column; align-items: center; font-size: 0.65rem; color: #888; }
  .gap { font-family: var(--display); font-size: 1.5rem; color: #ddd; }
  .sim { margin-top: 0.3rem; }
  .more { grid-column: 1 / -1; justify-self: center; font-size: 0.75rem; background: #222; color: #ddd; border: 1px solid #555; padding: 0.2rem 0.8rem; }
</style>
