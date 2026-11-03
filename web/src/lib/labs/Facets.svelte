<script lang="ts">
  // Facets: whatever a store's images carry, counted. Pick a property on
  // the left; its commonest values appear as bars; pick a value to see the
  // images that have it, newest first. Nothing here knows what kind of
  // store it is: a store of screenshots shows its media types and sizes,
  // a store of renders its moods and models.
  import type { LabProps } from './index'
  import { localName, prefixOf, count } from '../format'
  import Pic from './Pic.svelte'

  let { store, onopen }: LabProps = $props()

  interface Term {
    type: string
    value: string
    datatype?: string
    'xml:lang'?: string
  }
  let properties = $state<{ p: string; n: number }[]>([])
  let filter = $state('')
  let prop = $state<string | null>(null)
  let values = $state<{ v: Term; n: number }[]>([])
  let value = $state<Term | null>(null)
  let ids = $state<string[]>([])
  let total = $state(0)
  let error = $state<string | null>(null)
  let busy = $state('')

  async function get<T>(url: string, init?: RequestInit): Promise<T> {
    const r = await fetch(url, init)
    if (!r.ok) throw new Error(await r.text())
    return r.json()
  }

  $effect(() => {
    const st = store?.id
    if (!st) return
    properties = []
    prop = null
    error = null
    busy = 'counting the properties (a few seconds on a big store)…'
    get<{ props: { p: string; n: number }[] }>(`/api/lab/${st}/props`).then(
      (r) => ((properties = r.props), (busy = '')),
      (e) => ((error = (e as Error).message), (busy = '')),
    )
  })

  async function pickProp(p: string) {
    if (!store) return
    prop = p
    value = null
    ids = []
    values = []
    try {
      const r = await get<{ values: { v: Term; n: Term }[] }>(`/api/lab/${store.id}/facet?p=${encodeURIComponent(p)}`)
      if (prop === p) values = r.values.map((x) => ({ v: x.v, n: Number(x.n.value) }))
    } catch (e) {
      error = (e as Error).message
    }
  }

  async function pickValue(v: Term) {
    if (!store || !prop) return
    value = v
    ids = []
    try {
      const r = await get<{ ids: string[]; total: number }>(`/api/lab/${store.id}/facet`, {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ p: prop, v }),
      })
      if (value === v) (ids = r.ids), (total = r.total)
    } catch (e) {
      error = (e as Error).message
    }
  }

  const shownProps = $derived(
    properties.filter((x) => !filter || x.p.toLowerCase().includes(filter.toLowerCase())),
  )
  const top = $derived(values.reduce((m, x) => Math.max(m, x.n), 1))
  const label = (v: Term) => (v.type === 'uri' ? localName(v.value) : v.value)
</script>

<div class="facets">
  <nav>
    <header><b>Facets</b></header>
    <input type="search" placeholder="filter properties" bind:value={filter} />
    {#if busy}<p class="dim">{busy}</p>{/if}
    {#if error}<p class="err">{error}</p>{/if}
    <ul>
      {#each shownProps as x (x.p)}
        <li>
          <button class:on={prop === x.p} onclick={() => pickProp(x.p)} title={x.p}>
            <span class="ns">{prefixOf(x.p)}</span><span class="nm">{localName(x.p)}</span><span class="n">{count(x.n)}</span>
          </button>
        </li>
      {/each}
    </ul>
  </nav>
  <section class="vals">
    {#if prop}
      <h4>{localName(prop)} <span>the {values.length} commonest values</span></h4>
      {#each values as x (x.v.value + x.v.type)}
        <button class="bar" class:on={value?.value === x.v.value} onclick={() => pickValue(x.v)} title={x.v.value}>
          <span class="fill" style="width:{(100 * x.n) / top}%"></span>
          <span class="lab">{label(x.v)}</span>
          <span class="cnt">{count(x.n)}</span>
        </button>
      {/each}
    {:else}
      <p class="dim">Pick a property. Every fact Pan holds on the images is here, with how many statements use it.</p>
    {/if}
  </section>
  <section class="imgs">
    {#if value}
      <h4>{label(value)} <span>{count(total)} images{total > ids.length ? `, the newest ${count(ids.length)}` : ''}</span></h4>
      <div class="grid">{#each ids as id (id)}<Pic {id} size={104} {onopen} onclick={onopen} />{/each}</div>
    {/if}
  </section>
</div>

<style>
  .facets { position: absolute; inset: 0; display: grid; grid-template-columns: 19rem 24rem 1fr; background: var(--stage); color: var(--stage-soft); }
  nav { display: flex; flex-direction: column; min-height: 0; border-right: 1px solid #000; background: #181818; }
  header { padding: 0.45rem 0.9rem; }
  header b { color: var(--stage-ink); font-family: var(--display); font-size: 0.95rem; font-weight: normal; }
  nav input { margin: 0 0.9rem 0.4rem; background: #222; color: #ddd; border: 1px solid #444; font: inherit; font-size: 0.75rem; padding: 0.15rem 0.35rem; }
  ul { list-style: none; margin: 0; padding: 0; overflow-y: auto; flex: 1; }
  li button { display: flex; width: 100%; gap: 0.3rem; align-items: baseline; border: 0; background: none; color: #bbb; padding: 0.15rem 0.9rem; text-align: left; font-size: 0.72rem; }
  li button:hover { background: #232323; color: #fff; }
  li button.on { background: #ddd; color: #111; }
  .ns { color: #777; }
  .nm { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .n { color: #777; font-size: 0.66rem; }
  section { overflow-y: auto; padding: 0.6rem 0.9rem; min-height: 0; }
  .vals { border-right: 1px solid #000; }
  h4 { font-size: 0.7rem; letter-spacing: 0.1em; text-transform: uppercase; font-weight: normal; color: #ddd; margin: 0 0 0.5rem; }
  h4 span { text-transform: none; letter-spacing: 0; color: #777; margin-left: 0.4rem; }
  .bar { position: relative; display: flex; width: 100%; border: 0; background: none; padding: 0.2rem 0.3rem; margin-bottom: 2px; font-size: 0.72rem; color: #ccc; text-align: left; gap: 0.4rem; }
  .bar:hover { background: #222; }
  .bar.on { outline: 1px solid #ddd; }
  .fill { position: absolute; left: 0; top: 0; bottom: 0; background: #2f3a2f; z-index: 0; }
  .lab { position: relative; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .cnt { position: relative; color: #888; }
  .grid { display: flex; flex-wrap: wrap; gap: 5px; }
  .dim { color: #777; font-size: 0.75rem; padding: 0 0.9rem; }
  .err { color: var(--warn); font-size: 0.75rem; padding: 0 0.9rem; }
</style>
