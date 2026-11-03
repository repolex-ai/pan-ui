<script lang="ts">
  // The Lab module: development views that take the whole screen, chosen
  // from the list in labs/index.ts.
  import { labs } from './labs'
  import type { Store } from './api'
  import { status, type LabStatus } from './labs/data'
  import { count } from './format'

  interface Props {
    store: Store | null
    image: string | null
    onopen: (id: string) => void
    onset: (set: string) => void
  }
  let { store, image, onopen, onset }: Props = $props()

  let current = $state<string | null>(readLab())
  function readLab(): string | null {
    try {
      return localStorage.getItem('pan-ui.lab')
    } catch {
      return null
    }
  }
  function pick(id: string) {
    current = id
    try {
      localStorage.setItem('pan-ui.lab', id)
    } catch {}
  }
  const lab = $derived(labs.find((l) => l.id === current) ?? labs[0] ?? null)

  // How old the Lab's copy is. The views read a snapshot built by
  // tools/lab_build.py, and Pan keeps embedding new images after it.
  let st = $state<LabStatus | null>(null)
  $effect(() => {
    const id = store?.id
    st = null
    if (id) status(id).then((s) => (st = s), () => (st = null))
  })
</script>

<div class="lab">
  {#if labs.length > 1}
    <nav>
      {#each labs as l (l.id)}
        <button class:on={l.id === lab?.id} onclick={() => pick(l.id)} title={l.about}>{l.title}</button>
      {/each}
      {#if st}
        <span class="fresh" title={st.rebuild}>
          {#if st.ready}
            snapshot {st.built?.replace('T', ' ').slice(0, 16)} · {count(st.n)} images
            {#if st.embeddings_now != null && st.n != null && st.embeddings_now > st.n}· pand has {count(st.embeddings_now - st.n)} newer{/if}
          {:else}
            no Lab data for this store yet · hover for the command
          {/if}
        </span>
      {/if}
    </nav>
  {/if}
  <div class="stage">
    {#if lab}
      <lab.view {store} {image} {onopen} {onset} />
    {:else}
      <div class="empty">
        <h2>Lab</h2>
        <p>
          Whole-screen views for trying things out, outside the Library layout: a store's graph, a new way to
          browse, a measurement.
        </p>
        <p>
          There are none yet. A lab view is a Svelte component in <code>web/src/lib/labs/</code>, added to the list
          in <code>labs/index.ts</code>. It is given the open store
          {#if store}(<b>{store.name}</b>){/if} and the image under the cursor
          {#if image}(<code>{image}</code>){/if}, and the whole screen.
        </p>
      </div>
    {/if}
  </div>
</div>

<style>
  .lab { position: absolute; inset: 0; display: flex; flex-direction: column; background: var(--paper); }
  nav { display: flex; gap: 0.3rem; padding: 0.4rem 0.9rem; border-bottom: 1px solid var(--rule); }
  nav button { font-size: 0.76rem; padding: 0.15rem 0.6rem; }
  nav button.on { background: var(--ink); color: var(--paper); }
  nav { align-items: center; }
  .fresh { margin-left: auto; font-size: 0.68rem; color: var(--ink-faint); white-space: nowrap; }
  .stage { position: relative; flex: 1; min-height: 0; }
  .empty { max-width: 38rem; margin: 4rem auto; padding: 0 1rem; font-size: 0.85rem; color: var(--ink-soft); }
  .empty h2 { font-size: 1.4rem; color: var(--ink); margin-bottom: 0.6rem; }
</style>
