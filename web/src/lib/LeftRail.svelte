<script lang="ts">
  // The left rail: the stores pand holds, and the sets of the one open.
  // One store at a time; a set is a slice of it, newest set first.
  import type { Store, IndexState, Health } from './api'
  import { count, day } from './format'

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
  // 2,000 sets are 2,000 buttons; fine to draw, but a long list is drawn in
  // pieces so opening a store never stalls on it.
  let shown = $state(300)
  $effect(() => {
    void filter
    void index
    shown = 300
  })
</script>

<nav>
  <section class="stores">
    <h2 class="label">stores</h2>
    <ul>
      {#each stores as s (s.id)}
        <li>
          <button class:on={current?.id === s.id} disabled={!s.images} onclick={() => onstore(s)}>
            <span class="name">{s.name}</span>
            <span class="n">{count(s.images)}</span>
          </button>
        </li>
      {/each}
    </ul>
  </section>

  {#if current}
    <section class="sets">
      <h2 class="label">
        sets
        {#if index?.sets}<span class="n">{count(index.sets.length)}</span>{/if}
      </h2>
      {#if index?.ready}
        <input type="search" placeholder="filter sets" bind:value={filter} />
        <ul class="setlist">
          {#if !filter}
            <li>
              <button class:on={set === null} onclick={() => onset(null)}>
                <span class="name">everything, in time order</span>
                <span class="sub">{count(index.count)} · {day(index.first)} → {day(index.last)}</span>
              </button>
            </li>
          {/if}
          {#each sets.slice(0, shown) as s (s.id)}
            <li>
              <button class:on={set === s.id} onclick={() => onset(s.id)} title={s.id}>
                <span class="name">{s.id}</span>
                <span class="sub">{count(s.count)} · {day(s.first)}</span>
              </button>
            </li>
          {/each}
          {#if sets.length > shown}
            <li><button class="more" onclick={() => (shown += 500)}>{count(sets.length - shown)} more</button></li>
          {/if}
          {#if filter && !sets.length}<li class="empty">no set matches</li>{/if}
        </ul>
      {:else if index?.error}
        <p class="warn">{index.error}</p>
      {:else}
        <p class="empty">reading {current.name}'s store from pand…</p>
      {/if}
    </section>
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
  }
  .label {
    font-family: var(--mono);
    font-size: 0.72rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-soft);
    margin: 0.8rem 0.9rem 0.35rem;
    display: flex;
    justify-content: space-between;
  }
  ul { list-style: none; margin: 0; padding: 0; }
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
  .sets { display: flex; flex-direction: column; min-height: 0; flex: 1; border-top: 1px solid var(--rule); margin-top: 0.4rem; }
  .setlist { overflow-y: auto; flex: 1; }
  input {
    margin: 0 0.9rem 0.4rem;
    font: inherit;
    font-size: 0.8rem;
    padding: 0.2rem 0.4rem;
    border: 1px solid var(--rule-strong);
  }
  .more { font-size: 0.75rem; color: var(--ink-soft); text-decoration: underline; }
  .empty { color: var(--ink-faint); font-size: 0.8rem; padding: 0 0.9rem; }
  .warn { color: var(--warn); font-size: 0.8rem; padding: 0 0.9rem; }
  footer { font-size: 0.7rem; color: var(--ink-faint); padding: 0.5rem 0.9rem; border-top: 1px solid var(--rule); }
  footer .warn { padding: 0; }
</style>
