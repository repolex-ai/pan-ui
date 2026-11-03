<script lang="ts">
  // Words, then looks. Type a few words: the left column is every image
  // whose caption says them, newest first. The right column is what a word
  // search never finds: images that look like those (the average of their
  // vectors, seeded evenly across time) but whose captions do not say it.
  import type { LabProps } from './index'
  import { hybrid, type Hit } from './data'
  import { getFacts } from '../api'
  import { count } from '../format'
  import Pic from './Pic.svelte'

  let { store, onopen }: LabProps = $props()

  let q = $state('')
  let said = $state<string[]>([])
  let looks = $state<Hit[]>([])
  let total = $state<number | null>(null)
  let seeds = $state(0)
  let busy = $state(false)
  let error = $state<string | null>(null)
  let chosen = $state<string | null>(null)
  let caption = $state<string | null>(null)

  async function run() {
    if (!store || !q.trim()) return
    busy = true
    error = null
    try {
      const r = await hybrid(store.id, q.trim(), 90)
      said = r.said
      looks = r.looks
      total = r.total
      seeds = r.seeds
    } catch (e) {
      error = (e as Error).message
    } finally {
      busy = false
    }
  }

  async function choose(id: string) {
    chosen = id
    caption = null
    try {
      const f = await getFacts(id)
      let best = ''
      for (const [k, v] of Object.entries(f.facts)) if (/caption$/i.test(k) && v[0] && v[0].length > best.length) best = v[0]
      if (chosen === id) caption = best || '(no caption)'
    } catch {
      if (chosen === id) caption = '(could not read the facts)'
    }
  }

  const EXAMPLES = ['harp', 'moss', 'rain window', 'red dress', 'forest -night', 'smiling', 'from above']
</script>

<div class="words">
  <header>
    <b>Words &amp; Looks</b>
    <form onsubmit={(e) => (e.preventDefault(), run())}>
      <input type="search" placeholder="words in the caption; -word to leave out" bind:value={q} />
      <button type="submit">find</button>
    </form>
    <span class="ex">try:
      {#each EXAMPLES as x (x)}<button class="link" onclick={() => ((q = x), run())}>{x}</button>{/each}
    </span>
    {#if busy}<span>…</span>{/if}
  </header>
  {#if error}<p class="err">{error}</p>{/if}
  <div class="cols">
    <section>
      <h4>captions say it {#if total !== null}<span>{count(total)} images, newest first</span>{/if}</h4>
      <div class="grid">
        {#each said as id (id)}<Pic {id} size={96} on={chosen === id} onclick={choose} {onopen} />{/each}
      </div>
    </section>
    <section>
      <h4>looks like it, never says it {#if total !== null}<span>by look, from {seeds} of the matches spread across time</span>{/if}</h4>
      <div class="grid">
        {#each looks as h (h.id)}<Pic id={h.id} size={96} note={h.sim.toFixed(2)} on={chosen === h.id} onclick={choose} {onopen} />{/each}
      </div>
    </section>
    <aside>
      {#if chosen}
        <Pic id={chosen} size={300} {onopen} />
        <p class="cap">{caption ?? '…'}</p>
        <button onclick={() => onopen(chosen!)}>open in the Library</button>
      {:else}
        <p class="dim">The left column is a word search. The right column is the point: pictures that look like the matches, found by their vectors, whose captions never use your words. Click one to read what its caption does say.</p>
      {/if}
    </aside>
  </div>
</div>

<style>
  .words { position: absolute; inset: 0; display: flex; flex-direction: column; background: var(--stage); color: var(--stage-soft); }
  header { display: flex; gap: 1rem; align-items: center; padding: 0.45rem 0.9rem; border-bottom: 1px solid #000; font-size: 0.72rem; flex-wrap: wrap; }
  header b { color: var(--stage-ink); font-family: var(--display); font-size: 0.95rem; font-weight: normal; }
  form { display: flex; gap: 0.3rem; }
  input { width: 22rem; background: #222; color: #ddd; border: 1px solid #444; font: inherit; font-size: 0.8rem; padding: 0.2rem 0.4rem; }
  header button, aside button { font-size: 0.72rem; background: #222; color: #ddd; border: 1px solid #555; padding: 0.1rem 0.5rem; }
  .ex { color: #777; }
  .link { border: 0 !important; background: none !important; text-decoration: underline; color: #aaa !important; padding: 0 0.25rem !important; }
  .err { color: var(--warn); padding: 0.5rem 0.9rem; margin: 0; }
  .cols { flex: 1; min-height: 0; display: grid; grid-template-columns: 1fr 1fr 20rem; }
  section { overflow-y: auto; padding: 0.6rem; border-right: 1px solid #000; }
  h4 { font-size: 0.7rem; letter-spacing: 0.1em; text-transform: uppercase; font-weight: normal; color: #ddd; margin: 0 0 0.5rem; }
  h4 span { text-transform: none; letter-spacing: 0; color: #777; margin-left: 0.4rem; }
  .grid { display: flex; flex-wrap: wrap; gap: 5px; }
  aside { overflow-y: auto; padding: 0.8rem; background: #181818; }
  .cap { font-size: 0.75rem; line-height: 1.45; color: #bbb; white-space: pre-wrap; }
  .dim { color: #777; font-size: 0.75rem; line-height: 1.5; }
</style>
