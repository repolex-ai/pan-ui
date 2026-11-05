<script lang="ts">
  // The right-hand panel: what Pan knows about the image under the cursor,
  // in sections that fold. Facts are asked for only once the cursor has
  // rested for a moment, so holding an arrow key down does not send a
  // request per image.
  //
  // Pan stores anything, so most of this must make sense for a store of
  // screenshots as much as for generated images. File, rating, sets and all
  // facts are there for every image. Description and Generation appear only
  // when the image has facts of that kind.
  import type { Facts, Images, SetInfo } from './api'
  import { getFacts, getSetsOf } from './api'
  import { when, day, count, localName, namespaceOf, prefixOf } from './format'
  import Section from './Section.svelte'

  interface Props {
    store: string | null
    images: Images | null
    cursor: number
    set: string | null
    onset: (set: string | null) => void
    /** The image "more like this" was asked for, while that list is up. */
    like: string | null
    onlike: () => void
  }
  let { store, images, cursor, set, onset, like, onlike }: Props = $props()

  const id = $derived(images && cursor >= 0 && cursor < images.ids.length ? images.ids[cursor] : null)
  let facts = $state<Facts | null>(null)
  let sets = $state<SetInfo[] | null>(null)
  let error = $state<string | null>(null)
  const cache = new Map<string, { facts: Facts; sets: SetInfo[] | null }>()

  $effect(() => {
    const want = id
    const st = store
    error = null
    if (!want || !st) {
      facts = null
      sets = null
      return
    }
    const key = `${st}/${want}`
    const hit = cache.get(key)
    if (hit) {
      facts = hit.facts
      sets = hit.sets
      return
    }
    facts = null
    sets = null
    const t = setTimeout(() => {
      Promise.all([getFacts(want), getSetsOf(st, want).then((r) => r.sets, () => null)]).then(
        ([f, s]) => {
          if (cache.size > 500) cache.clear()
          cache.set(key, { facts: f, sets: s })
          if (id === want) {
            facts = f
            sets = s
          }
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
      for (const [k, v] of Object.entries(f.facts)) if (localName(k) === n && v.length && v[0] !== '') return v[0]
    }
    return null
  }
  /** Any caption Pan holds, whatever wrote it: `caption`, or a name ending `Caption`. */
  function pickCaption(f: Facts | null): string | null {
    const first = pickFact(f, 'caption', 'longCaption', 'shortCaption')
    if (first || !f) return first
    for (const [k, v] of Object.entries(f.facts)) if (/Caption$/.test(localName(k)) && v[0]) return v[0]
    return null
  }

  const size = $derived.by(() => {
    const w = pickFact(facts, 'width')
    const h = pickFact(facts, 'height')
    return w && h ? `${w} × ${h}` : null
  })
  const type = $derived(pickFact(facts, 'mediaType'))
  const path = $derived(pickFact(facts, 'mediaPath', 'sourceFile'))
  const added = $derived(pickFact(facts, 'createdDate'))
  const caption = $derived(pickCaption(facts))
  const gen = $derived.by(() => {
    const rows = [
      ['model', pickFact(facts, 'genModel', 'model')],
      ['seed', pickFact(facts, 'seed', 'genSeed')],
      ['steps', pickFact(facts, 'genSteps', 'steps')],
      ['engine', pickFact(facts, 'GenEngine', 'genEngine')],
    ].filter(([, v]) => v) as [string, string][]
    return { rows, prompt: pickFact(facts, 'renderPrompt', 'prompt') }
  })
  const hasGen = $derived(gen.rows.length > 0 || !!gen.prompt)
  const all = $derived(
    facts
      ? Object.entries(facts.facts)
          .map(([k, v]) => ({ k, name: localName(k), ns: namespaceOf(k), label: prefixOf(k), v }))
          .sort((a, b) => a.label.localeCompare(b.label) || a.ns.localeCompare(b.ns) || a.name.localeCompare(b.name))
      : [],
  )
  // One group per vocabulary, so its name is said once, not on every line.
  // Grouped by the full namespace, so two that share a short name stay apart.
  const groups = $derived.by(() => {
    const out: { ns: string; label: string; rows: typeof all }[] = []
    for (const f of all) {
      if (out.at(-1)?.ns !== f.ns) out.push({ ns: f.ns, label: f.label, rows: [] })
      out.at(-1)!.rows.push(f)
    }
    return out
  })
  let promptOpen = $state(false)
</script>

<aside>
  {#if id && images}
    <Section key="file" title="file">
      <code class="id">{id}</code>
      <p class="date">{when(images.t[cursor])}</p>
      <dl>
        {#if size}<dt>size</dt><dd>{size}</dd>{/if}
        {#if type}<dt>type</dt><dd>{type}</dd>{/if}
        {#if added}<dt>added</dt><dd>{added.slice(0, 10)}</dd>{/if}
        {#if path}<dt>path</dt><dd class="path">{path}</dd>{/if}
      </dl>
    </Section>

    <Section key="rating" title="rating">
      <p class="marks">
        <span class="stars">{images.rating[cursor] ? '★'.repeat(images.rating[cursor]) : 'no stars'}</span>
        ·
        <span>{images.flag[cursor] > 0 ? 'picked' : images.flag[cursor] < 0 ? 'rejected' : 'no flag'}</span>
      </p>
    </Section>

    <Section key="like" title="looks like">
      <p class="marks">
        <button onclick={onlike} title="the images that look most like this one, by Pan's embeddings (S)">more like this</button>
        {#if like && images.score}
          {#if id === like}<span class="sub">the image the list is for</span>
          {:else}<span class="sub">{images.score[cursor]?.toFixed(2)} alike {like}</span>{/if}
        {/if}
      </p>
    </Section>

    <Section key="sets" title="sets" aside={sets ? count(sets.length) : null}>
      {#if sets === null}
        <p class="empty">…</p>
      {:else}
        <ul class="sets">
          <li>
            <button class:on={set === null} onclick={() => onset(null)}>everything, newest first</button>
          </li>
          {#each sets as s (s.id)}
            <li>
              <button class:on={set === s.id} onclick={() => onset(s.id)} title={s.id}>
                <span class="name">{s.id}</span>
                <span class="sub">{count(s.count)} · {day(s.first)}</span>
              </button>
            </li>
          {/each}
        </ul>
        {#if !sets.length}<p class="empty">in no set</p>{/if}
      {/if}
    </Section>

    {#if error}
      <p class="warn pad">{error}</p>
    {:else if !facts}
      <p class="empty pad">…</p>
    {:else}
      {#if caption}
        <Section key="description" title="description">
          <p class="text">{caption}</p>
        </Section>
      {/if}
      {#if hasGen}
        <Section key="generation" title="generation">
          {#if gen.rows.length}
            <dl>
              {#each gen.rows as [k, v] (k)}<dt>{k}</dt><dd>{v}</dd>{/each}
            </dl>
          {/if}
          {#if gen.prompt}
            <h3 class="label">prompt</h3>
            <p class="text prompt" class:clamped={!promptOpen}>{gen.prompt}</p>
            {#if gen.prompt.length > 400}
              <button class="more" onclick={() => (promptOpen = !promptOpen)}>{promptOpen ? 'less' : 'more'}</button>
            {/if}
          {/if}
        </Section>
      {/if}
      <Section key="facts" title="all facts" aside={count(all.length)} closed>
        {#each groups as g (g.ns)}
          <h3 class="label" title={g.ns}>{g.label || 'other'}</h3>
          <dl class="facts">
            {#each g.rows as f (f.k)}
              <dt title={f.k}>{f.name}</dt>
              <dd>{f.v.join(' · ')}</dd>
            {/each}
          </dl>
        {/each}
      </Section>
    {/if}
  {:else}
    <p class="empty pad">pick a store and a set</p>
  {/if}

  <div class="keys">
    <Section key="keys" title="keys" closed>
      <table>
        <tbody>
          <tr><td><kbd>←</kbd> <kbd>→</kbd> <kbd>↑</kbd> <kbd>↓</kbd></td><td>move</td></tr>
          <tr><td><kbd>Enter</kbd> <kbd>Space</kbd></td><td>full size / back</td></tr>
          <tr><td><kbd>G</kbd> <kbd>Esc</kbd></td><td>grid</td></tr>
          <tr><td><kbd>R</kbd></td><td>a random image from the set</td></tr>
          <tr><td><kbd>⇧</kbd> <kbd>R</kbd></td><td>back to the image before the last R</td></tr>
          <tr><td><kbd>S</kbd></td><td>more like this; <kbd>⌫</kbd> back to the set</td></tr>
          <tr><td><kbd>1</kbd>–<kbd>5</kbd> <kbd>0</kbd></td><td>stars / none</td></tr>
          <tr><td><kbd>P</kbd> <kbd>X</kbd> <kbd>U</kbd></td><td>pick / reject / no flag</td></tr>
          <tr><td><kbd>⇧</kbd> <kbd>↑</kbd> <kbd>↓</kbd></td><td>flag one step toward pick / reject</td></tr>
          <tr><td><kbd>⇧</kbd> + any mark</td><td>mark, then next</td></tr>
          <tr><td><kbd>=</kbd> <kbd>-</kbd></td><td>bigger / smaller thumbnails</td></tr>
          <tr><td><kbd>Tab</kbd></td><td>hide / show side panels</td></tr>
          <tr><td><kbd>⇧</kbd> <kbd>Tab</kbd></td><td>hide / show all panels</td></tr>
        </tbody>
      </table>
    </Section>
  </div>
</aside>

<style>
  aside {
    border-left: 1px solid var(--rule);
    overflow-y: auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .pad { padding: 0.6rem 0.9rem; margin: 0; }
  .label {
    font-family: var(--mono);
    font-size: 0.68rem;
    letter-spacing: 0.12em;
    text-transform: uppercase;
    color: var(--ink-faint);
    margin: 0.7rem 0 0.3rem;
  }
  .id { font-size: 0.8rem; }
  .date { margin: 0.15rem 0 0; font-family: var(--display); font-size: 1.05rem; }
  .marks { margin: 0; font-size: 0.8rem; color: var(--ink-soft); }
  .stars { color: #b8901c; }
  dl { display: grid; grid-template-columns: auto 1fr; gap: 0.1rem 0.8rem; margin: 0.5rem 0 0; font-size: 0.78rem; }
  dt { color: var(--ink-faint); }
  dd { margin: 0; overflow-wrap: anywhere; }
  .path { font-size: 0.7rem; }
  .text { margin: 0; font-size: 0.8rem; line-height: 1.45; white-space: pre-wrap; overflow-wrap: anywhere; }
  .prompt.clamped { max-height: 9rem; overflow: hidden; mask-image: linear-gradient(#000 70%, transparent); }
  .more { border: 0; padding: 0; font-size: 0.72rem; text-decoration: underline; background: none; align-self: flex-start; }
  .more:hover { background: none; color: var(--ink); }
  .sets { list-style: none; margin: 0; padding: 0; max-height: 14rem; overflow-y: auto; }
  .sets button {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    width: 100%;
    border: 0;
    background: none;
    padding: 0.2rem 0.4rem;
    text-align: left;
    font-size: 0.74rem;
  }
  .sets button:hover { background: var(--paper-tint); color: var(--ink); }
  .sets button.on { background: var(--ink); color: var(--paper); }
  .sets button.on .sub { color: var(--paper); }
  .sets .name { overflow-wrap: anywhere; }
  .sets .sub { font-size: 0.68rem; color: var(--ink-faint); }
  table { width: 100%; border-collapse: collapse; font-size: 0.7rem; }
  td { border-bottom: 1px solid var(--rule); padding: 0.15rem 0.3rem 0.15rem 0; vertical-align: top; }
  /* Name above value, so a long name never squeezes the value into a sliver. */
  dl.facts { display: block; margin: 0; font-size: 0.72rem; }
  dl.facts dt { color: var(--ink-faint); margin-top: 0.35rem; overflow-wrap: anywhere; }
  dl.facts dd { padding-bottom: 0.3rem; border-bottom: 1px solid var(--rule); white-space: pre-wrap; }
  .empty { color: var(--ink-faint); font-size: 0.8rem; margin: 0; }
  .warn { color: var(--warn); font-size: 0.8rem; }
  .keys { margin-top: auto; border-top: 1px solid var(--rule); }
  .keys td { border: 0; font-size: 0.74rem; padding: 0.12rem 0.6rem 0.12rem 0; color: var(--ink-soft); }
  kbd {
    font-family: var(--mono);
    font-size: 0.68rem;
    border: 1px solid var(--ink-faint);
    padding: 0 0.25rem;
    color: var(--ink);
  }
</style>
