<script lang="ts">
  // The left rail: the stores pand holds, and the sets of the one open.
  // One store at a time; a set is a slice of it, newest set first, folded
  // into months and days by the date of the set's first image.
  import type { Store, IndexState, Health, SetInfo } from './api'
  import { count, day } from './format'
  import Section from './Section.svelte'

  interface Props {
    stores: Store[]
    current: Store | null
    index: IndexState | null
    set: string | null
    health: Health | null
    onstore: (s: Store) => void
    onset: (set: string | null) => void
  }
  let { stores, current, index, set, health, onstore, onset }: Props = $props()

  let filter = $state('')
  const sets = $derived.by(() => {
    const all = index?.sets ?? []
    const f = filter.trim().toLowerCase()
    return f ? all.filter((s) => s.id.toLowerCase().includes(f)) : all
  })
  // 2,000 sets are 2,000 buttons; fine to draw, but a long filtered list is
  // drawn in pieces so typing never stalls on it.
  let shown = $state(300)
  $effect(() => {
    void filter
    void index
    shown = 300
  })

  // Months, then days, newest first. The sets already come newest first, so
  // grouping is one pass.
  interface Day { key: string; title: string; sets: SetInfo[] }
  interface Month { key: string; title: string; days: Day[]; n: number }
  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec']
  const tree = $derived.by(() => {
    const out: Month[] = []
    for (const s of index?.sets ?? []) {
      const d = new Date(s.first * 1000)
      const mk = `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}`
      const dk = day(s.first)
      let m = out.at(-1)
      if (m?.key !== mk) out.push((m = { key: mk, title: `${MONTHS[d.getMonth()]} ${d.getFullYear()}`, days: [], n: 0 }))
      let dd = m.days.at(-1)
      if (dd?.key !== dk) m.days.push((dd = { key: dk, title: `${d.getDate()} ${MONTHS[d.getMonth()]}`, sets: [] }))
      dd.sets.push(s)
      m.n++
    }
    return out
  })

  // Which months and days are open. Unless you have said otherwise, the
  // newest month and day are open, and so are the ones holding the open set.
  let opened = $state<Record<string, boolean>>(readOpened())
  function readOpened(): Record<string, boolean> {
    try {
      return JSON.parse(localStorage.getItem('pan-ui.setdays') ?? '{}') ?? {}
    } catch {
      return {}
    }
  }
  const setDay = $derived.by(() => {
    const s = set ? index?.sets?.find((x) => x.id === set) : null
    return s ? day(s.first) : null
  })
  function isOpen(key: string, newest: boolean): boolean {
    return opened[key] ?? (newest || (!!setDay && setDay.startsWith(key)))
  }
  function toggle(key: string, newest: boolean) {
    opened = { ...opened, [key]: !isOpen(key, newest) }
    try {
      localStorage.setItem('pan-ui.setdays', JSON.stringify(opened))
    } catch {}
  }
</script>

{#snippet setButton(s: SetInfo)}
  <li>
    <button class="set" class:on={set === s.id} onclick={() => onset(s.id)} title={s.id}>
      <span class="name">{s.id}</span>
      <span class="sub">{count(s.count)} · {day(s.first)}</span>
    </button>
  </li>
{/snippet}

<nav>
  <Section key="stores" title="stores" aside={count(stores.length)}>
    <ul class="stores">
      {#each stores as s (s.id)}
        <li>
          <button class:on={current?.id === s.id} disabled={!s.images} onclick={() => onstore(s)}>
            <span class="name">{s.name}</span>
            <span class="n">{count(s.images)}</span>
          </button>
        </li>
      {/each}
    </ul>
  </Section>

  {#if current}
    <Section key="sets" title="sets" aside={index?.sets ? count(index.sets.length) : null} grow>
      {#if index?.ready}
        <input type="search" placeholder="filter sets" bind:value={filter} />
        <ul class="setlist" class:tree={!filter}>
          {#if !filter}
            <li>
              <button class:on={set === null} onclick={() => onset(null)}>
                <span class="name">everything, newest first</span>
                <span class="sub">{count(index.count)} · {day(index.first)} → {day(index.last)}</span>
              </button>
            </li>
          {/if}
          {#if filter}
            {#each sets.slice(0, shown) as s (s.id)}{@render setButton(s)}{/each}
          {:else}
            {#each tree as m, mi (m.key)}
              {@const mOpen = isOpen(m.key, mi === 0)}
              <li>
                <button class="fold month" onclick={() => toggle(m.key, mi === 0)} aria-expanded={mOpen}>
                  <span class="tri">{mOpen ? '▾' : '▸'}</span><span class="t">{m.title}</span><span class="c">{count(m.n)}</span>
                </button>
              </li>
              {#if mOpen}
                {#each m.days as d, di (d.key)}
                  {@const dOpen = isOpen(d.key, mi === 0 && di === 0)}
                  <li>
                    <button class="fold dayrow" onclick={() => toggle(d.key, mi === 0 && di === 0)} aria-expanded={dOpen}>
                      <span class="tri">{dOpen ? '▾' : '▸'}</span><span class="t">{d.title}</span><span class="c">{count(d.sets.length)}</span>
                    </button>
                  </li>
                  {#if dOpen}
                    {#each d.sets as s (s.id)}{@render setButton(s)}{/each}
                  {/if}
                {/each}
              {/if}
            {/each}
          {/if}
          {#if filter && sets.length > shown}
            <li><button class="more" onclick={() => (shown += 500)}>{count(sets.length - shown)} more</button></li>
          {/if}
          {#if filter && !sets.length}<li class="empty">no set matches</li>{/if}
        </ul>
      {:else if index?.error}
        <p class="warn">{index.error}</p>
      {:else}
        <p class="empty">reading {current.name}'s store from pand…</p>
      {/if}
    </Section>
  {/if}

  <footer>
    {#if health?.pand}
      pand {health.pand.version} · {health.pand_url.replace('http://', '')}
    {:else if health}
      <span class="warn">{health.error ?? 'pand is not answering'}</span>
    {/if}
  </footer>
</nav>

<style>
  nav {
    border-right: 1px solid var(--rule);
    display: flex;
    flex-direction: column;
    min-height: 0;
    height: 100%;
  }
  ul { list-style: none; margin: 0 -0.9rem; padding: 0; }
  li button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    text-align: left;
    border: 0;
    padding: 0.3rem 0.9rem;
    background: none;
  }
  .stores li button { flex-direction: row; justify-content: space-between; align-items: baseline; }
  li button:hover:not(:disabled) { background: var(--paper-tint); color: var(--ink); }
  li button.on { background: var(--ink); color: var(--paper); }
  li button.on .sub, li button.on .n { color: var(--paper); }
  .name { font-family: var(--display); font-size: 0.95rem; }
  .setlist .name { font-family: var(--mono); font-size: 0.74rem; overflow-wrap: anywhere; }
  .n, .sub { font-size: 0.7rem; color: var(--ink-faint); }
  .setlist { overflow-y: auto; flex: 1; min-height: 0; }
  input {
    margin: 0 0 0.4rem;
    font: inherit;
    font-size: 0.8rem;
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--rule-strong);
  }
  /* Month and day rows: one line each, so a closed month costs one line. */
  li button.fold { flex-direction: row; align-items: baseline; gap: 0.35rem; padding: 0.22rem 0.9rem; }
  .fold .tri { width: 0.7rem; color: var(--ink-faint); font-size: 0.7rem; }
  .fold .t { flex: 1; }
  .fold .c { font-size: 0.7rem; color: var(--ink-faint); }
  .month { font-family: var(--display); font-size: 0.9rem; border-top: 1px solid var(--rule) !important; }
  .dayrow { padding-left: 1.6rem !important; font-size: 0.78rem; color: var(--ink-soft); }
  .tree li button.set { padding-left: 2.6rem; }
  .more { font-size: 0.75rem; color: var(--ink-soft); text-decoration: underline; }
  .empty { color: var(--ink-faint); font-size: 0.8rem; margin: 0; }
  .warn { color: var(--warn); font-size: 0.8rem; margin: 0; }
  footer { margin-top: auto; font-size: 0.7rem; color: var(--ink-faint); padding: 0.5rem 0.9rem; border-top: 1px solid var(--rule); }
</style>
