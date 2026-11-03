<script lang="ts">
  // The bar above the grid: show only some flags, and only some ratings.
  // Flags are three switches (none on means all); stars are a comparison
  // and a number of stars, click the same star again to drop it.
  import type { Filter, Op } from './filter'
  import { isOff, NO_FILTER } from './filter'
  import { count } from './format'

  interface Props {
    filter: Filter
    shown: number
    total: number
    onchange: (f: Filter) => void
  }
  let { filter, shown, total, onchange }: Props = $props()

  const OPS: Op[] = ['>=', '<=', '=']
  const OP_TEXT: Record<Op, string> = { '>=': '≥', '<=': '≤', '=': '=' }
  const set = (p: Partial<Filter>) => onchange({ ...filter, ...p })
</script>

<div class="bar">
  <span class="label">show</span>
  <button class:on={filter.picked} onclick={() => set({ picked: !filter.picked })} title="picked (P)">⚑ picked</button>
  <button class:on={filter.unflagged} onclick={() => set({ unflagged: !filter.unflagged })} title="no flag (U)">unflagged</button>
  <button class:on={filter.rejected} onclick={() => set({ rejected: !filter.rejected })} title="rejected (X)">✕ rejected</button>

  <span class="label gap">rating</span>
  <button
    class="op"
    onclick={() => set({ op: OPS[(OPS.indexOf(filter.op) + 1) % OPS.length] })}
    title="at least / at most / exactly"
  >{OP_TEXT[filter.op]}</button>
  <span class="stars">
    <button class="star zero" class:on={filter.stars === 0} onclick={() => set({ stars: filter.stars === 0 ? null : 0 })} title="no stars">{filter.stars === 0 ? '●' : '○'}</button>
    {#each [1, 2, 3, 4, 5] as n (n)}
      <button
        class="star"
        class:lit={filter.stars !== null && n <= filter.stars}
        onclick={() => set({ stars: filter.stars === n ? null : n })}
        title="{OP_TEXT[filter.op]} {n} star{n > 1 ? 's' : ''}"
      >★</button>
    {/each}
  </span>

  {#if !isOff(filter)}
    <button class="clear" onclick={() => onchange({ ...NO_FILTER, op: filter.op })}>clear</button>
    <span class="n">{count(shown)} of {count(total)}</span>
  {/if}
</div>

<style>
  .bar {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    padding: 0.3rem 0.7rem;
    background: var(--paper);
    border-bottom: 1px solid var(--rule-strong);
    font-size: 0.72rem;
    white-space: nowrap;
    overflow: hidden;
  }
  .label { color: var(--ink-faint); margin-right: 0.15rem; }
  .gap { margin-left: 0.8rem; }
  button { font-size: 0.72rem; padding: 0.05rem 0.45rem; }
  button.on { background: var(--ink); color: var(--paper); }
  .op { min-width: 1.6rem; }
  .stars { display: inline-flex; }
  .star { border: 0; background: none; padding: 0 0.12rem; font-size: 0.95rem; color: var(--rule-strong); }
  .star:hover { background: none; color: var(--ink-soft); }
  .star.lit { color: #b8901c; }
  .star.zero { font-size: 0.8rem; margin-right: 0.2rem; }
  .star.zero.on { background: none; color: var(--ink); }
  .clear { margin-left: 0.6rem; border: 0; background: none; text-decoration: underline; color: var(--ink-soft); }
  .n { margin-left: auto; color: var(--ink-soft); }
</style>
