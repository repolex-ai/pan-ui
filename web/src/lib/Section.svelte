<script lang="ts" module>
  // Which sections are folded, remembered across reloads. One record for the
  // whole page, keyed by the section's name, so a section folded once stays
  // folded wherever it is drawn.
  let folded = $state<Record<string, boolean>>(read())
  function read(): Record<string, boolean> {
    try {
      return JSON.parse(localStorage.getItem('pan-ui.folded') ?? '{}') ?? {}
    } catch {
      return {}
    }
  }
  function toggle(key: string, closedByDefault: boolean) {
    folded = { ...folded, [key]: !(folded[key] ?? closedByDefault) }
    try {
      localStorage.setItem('pan-ui.folded', JSON.stringify(folded))
    } catch {}
  }
</script>

<script lang="ts">
  // One foldable section of a side panel, Lightroom's way: the title is the
  // button, and a folded section is one line tall.
  import type { Snippet } from 'svelte'

  interface Props {
    /** Remembers the fold; also the title unless one is given. */
    key: string
    title?: string
    /** Shown at the right of the title line, e.g. a count. */
    aside?: string | null
    closed?: boolean
    /** Let the open section take the panel's remaining height and scroll. */
    grow?: boolean
    children: Snippet
  }
  let { key, title, aside = null, closed = false, grow = false, children }: Props = $props()
  const open = $derived(!(folded[key] ?? closed))
</script>

<section class:grow={grow && open}>
  <button class="head" onclick={() => toggle(key, closed)} aria-expanded={open}>
    <span class="tri">{open ? '▾' : '▸'}</span>
    <span class="title">{title ?? key}</span>
    {#if aside}<span class="aside">{aside}</span>{/if}
  </button>
  {#if open}
    <div class="body">{@render children()}</div>
  {/if}
</section>

<style>
  /* A section keeps its height; the panel scrolls. Only a growing section
     (the left rail's long list of sets) shrinks to fit and scrolls inside. */
  section { border-bottom: 1px solid var(--rule); display: flex; flex-direction: column; flex-shrink: 0; }
  section.grow { flex: 1 1 0; min-height: 8rem; }
  .head {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    width: 100%;
    border: 0;
    background: none;
    padding: 0.45rem 0.9rem;
    text-align: left;
    font-family: var(--mono);
    font-size: 0.72rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-soft);
  }
  .head:hover:not(:disabled) { background: var(--paper-tint); color: var(--ink); }
  .tri { width: 0.8rem; color: var(--ink-faint); }
  .title { flex: 1; }
  .aside { letter-spacing: 0; text-transform: none; color: var(--ink-faint); }
  .body { padding: 0 0.9rem 0.7rem; min-height: 0; display: flex; flex-direction: column; }
  section.grow .body { flex: 1; }
</style>
