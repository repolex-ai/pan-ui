<script lang="ts">
  // The right-hand panel: what Pan knows about the image under the cursor,
  // and the keys. Facts are asked for only once the cursor has rested for a
  // moment, so holding an arrow key down does not send a request per image.
  import type { Facts, Images } from './api'
  import { getFacts } from './api'
  import { when, localName, prefixOf } from './format'

  interface Props {
    images: Images | null
    cursor: number
  }
  let { images, cursor }: Props = $props()

  const id = $derived(images && cursor >= 0 && cursor < images.ids.length ? images.ids[cursor] : null)
  let facts = $state<Facts | null>(null)
  let error = $state<string | null>(null)
  const cache = new Map<string, Facts>()

  $effect(() => {
    const want = id
    error = null
    if (!want) {
      facts = null
      return
    }
    const hit = cache.get(want)
    if (hit) {
      facts = hit
      return
    }
    facts = null
    const t = setTimeout(() => {
      getFacts(want).then(
        (f) => {
          if (cache.size > 500) cache.clear()
          cache.set(want, f)
          if (id === want) facts = f
        },
        (e) => {
          if (id === want) error = (e as Error).message
        },
      )
    }, 140)
    return () => clearTimeout(t)
  })

  /** The first value of the first of these local names that is present. */
  function pickFact(f: Facts | null, ...names: string[]): string | null {
    if (!f) return null
    for (const n of names) {
      for (const [k, v] of Object.entries(f.facts)) if (localName(k) === n && v.length) return v[0]
    }
    return null
  }
  const caption = $derived(pickFact(facts, 'caption', 'longCaption', 'shortCaption', 'qwen35vl9bCaption'))
  const prompt = $derived(pickFact(facts, 'renderPrompt', 'prompt'))
  const model = $derived(pickFact(facts, 'genModel'))
  const seed = $derived(pickFact(facts, 'seed'))
  const size = $derived.by(() => {
    const w = pickFact(facts, 'width')
    const h = pickFact(facts, 'height')
    return w && h ? `${w} × ${h}` : null
  })
  const all = $derived(
    facts
      ? Object.entries(facts.facts)
          .map(([k, v]) => ({ k, name: localName(k), ns: prefixOf(k), v }))
          .sort((a, b) => a.ns.localeCompare(b.ns) || a.name.localeCompare(b.name))
      : [],
  )
  let promptOpen = $state(false)
</script>

<aside>
  {#if id && images}
    <header>
      <code class="id">{id}</code>
      <p class="date">{when(images.t[cursor])}</p>
      <p class="marks">
        <span class="stars">{images.rating[cursor] ? '★'.repeat(images.rating[cursor]) : 'no stars'}</span>
        ·
        <span>{images.flag[cursor] > 0 ? 'picked' : images.flag[cursor] < 0 ? 'rejected' : 'no flag'}</span>
      </p>
    </header>
    {#if error}
      <p class="warn">{error}</p>
    {:else if !facts}
      <p class="empty">…</p>
    {:else}
      <dl>
        {#if size}<dt>size</dt><dd>{size}</dd>{/if}
        {#if model}<dt>model</dt><dd>{model}</dd>{/if}
        {#if seed}<dt>seed</dt><dd>{seed}</dd>{/if}
      </dl>
      {#if caption}
        <h3 class="label">caption</h3>
        <p class="text">{caption}</p>
      {/if}
      {#if prompt}
        <h3 class="label">prompt</h3>
        <p class="text prompt" class:clamped={!promptOpen}>{prompt}</p>
        {#if prompt.length > 400}
          <button class="more" onclick={() => (promptOpen = !promptOpen)}>{promptOpen ? 'less' : 'more'}</button>
        {/if}
      {/if}
      <details>
        <summary class="label">all {all.length} facts</summary>
        <table>
          <tbody>
            {#each all as f (f.k)}
              <tr>
                <td class="k" title={f.k}><span class="ns">{f.ns}:</span>{f.name}</td>
                <td class="v">{f.v.join(' · ')}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      </details>
    {/if}
  {:else}
    <p class="empty">pick a store and a set</p>
  {/if}

  <section class="keys">
    <h3 class="label">keys</h3>
    <table>
      <tbody>
        <tr><td><kbd>←</kbd> <kbd>→</kbd> <kbd>↑</kbd> <kbd>↓</kbd></td><td>move</td></tr>
        <tr><td><kbd>Enter</kbd> <kbd>Space</kbd></td><td>full size / back</td></tr>
        <tr><td><kbd>G</kbd> <kbd>Esc</kbd></td><td>grid</td></tr>
        <tr><td><kbd>1</kbd>–<kbd>5</kbd> <kbd>0</kbd></td><td>stars / none</td></tr>
        <tr><td><kbd>P</kbd> <kbd>X</kbd> <kbd>U</kbd></td><td>pick / reject / no flag</td></tr>
        <tr><td><kbd>⇧</kbd> + any mark</td><td>mark, then next</td></tr>
        <tr><td><kbd>=</kbd> <kbd>-</kbd></td><td>bigger / smaller thumbnails</td></tr>
      </tbody>
    </table>
  </section>
</aside>

<style>
  aside {
    border-left: 1px solid var(--rule);
    overflow-y: auto;
    min-height: 0;
    padding: 0.8rem 0.9rem 1rem;
    display: flex;
    flex-direction: column;
  }
  .label {
    font-family: var(--mono);
    font-size: 0.72rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-soft);
    margin: 1rem 0 0.35rem;
  }
  .id { font-size: 0.8rem; }
  .date { margin: 0.15rem 0 0; font-family: var(--display); font-size: 1.05rem; }
  .marks { margin: 0.15rem 0 0; font-size: 0.8rem; color: var(--ink-soft); }
  .stars { color: #b8901c; }
  dl { display: grid; grid-template-columns: auto 1fr; gap: 0.1rem 0.8rem; margin: 0.8rem 0 0; font-size: 0.78rem; }
  dt { color: var(--ink-faint); }
  dd { margin: 0; overflow-wrap: anywhere; }
  .text { margin: 0; font-size: 0.8rem; line-height: 1.45; white-space: pre-wrap; overflow-wrap: anywhere; }
  .prompt.clamped { max-height: 9rem; overflow: hidden; mask-image: linear-gradient(#000 70%, transparent); }
  .more { border: 0; padding: 0; font-size: 0.72rem; text-decoration: underline; background: none; }
  .more:hover { background: none; color: var(--ink); }
  details { margin-top: 0.4rem; }
  summary { cursor: pointer; }
  details table { width: 100%; border-collapse: collapse; font-size: 0.7rem; }
  details td { border-bottom: 1px solid var(--rule); padding: 0.15rem 0.3rem 0.15rem 0; vertical-align: top; }
  .k { white-space: nowrap; color: var(--ink-soft); }
  .ns { color: var(--ink-faint); }
  .v { overflow-wrap: anywhere; max-width: 0; width: 100%; }
  .empty { color: var(--ink-faint); font-size: 0.8rem; }
  .warn { color: var(--warn); font-size: 0.8rem; }
  .keys { margin-top: auto; padding-top: 1rem; }
  .keys table { font-size: 0.74rem; border-collapse: collapse; }
  .keys td { padding: 0.12rem 0.6rem 0.12rem 0; color: var(--ink-soft); }
  kbd {
    font-family: var(--mono);
    font-size: 0.68rem;
    border: 1px solid var(--ink-faint);
    padding: 0 0.25rem;
    color: var(--ink);
  }
</style>
