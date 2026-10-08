<script lang="ts">
  // pand's config, as the text files they are: config.yml (stores, port,
  // and every model pass: which model, its endpoint, its prompt, what is sent
  // with each request) and the caption prompts. Plain text on purpose: the
  // comments in config.yml carry the reasons behind each setting, and a form
  // would lose them.
  import { onMount } from 'svelte'
  import { getConfig, saveConfig } from './api'
  import type { ConfigFile } from './api'

  let files = $state<ConfigFile[]>([])
  let dir = $state('')
  let loadError = $state<string | null>(null)
  let current = $state<string>('config.yml')
  /** Edited text per file name; a file absent here is unedited. */
  let drafts = $state<Record<string, string>>({})
  let message = $state<{ ok: boolean; text: string } | null>(null)
  let saving = $state(false)
  let copyName = $state('')

  const file = $derived(files.find((f) => f.name === current) ?? null)
  const text = $derived(file ? (drafts[file.name] ?? file.text) : '')
  const dirty = (f: ConfigFile) => drafts[f.name] !== undefined && drafts[f.name] !== f.text

  async function load(keep = true) {
    try {
      const r = await getConfig()
      files = r.files
      dir = r.dir
      loadError = null
      if (!keep) drafts = {}
      if (!files.some((f) => f.name === current)) current = 'config.yml'
    } catch (e) {
      loadError = (e as Error).message
    }
  }

  function edit(v: string) {
    if (!file) return
    drafts = { ...drafts, [file.name]: v }
    message = null
  }

  async function save(asName: string | null = null) {
    if (!file || saving) return
    saving = true
    const name = asName ?? file.name
    try {
      const r = await saveConfig(name, text, asName ? null : file.version)
      const { [file.name]: _, ...rest } = drafts
      drafts = rest
      await load()
      current = name
      copyName = ''
      message = {
        ok: true,
        text:
          `Saved ${r.saved.path}.` +
          (r.saved.backup ? ` The previous version is in ${r.saved.backup}.` : '') +
          (r.saved.checked ? ` pand checked it: ${r.saved.checked}.` : '') +
          ` pand reads it ${r.takes_effect}.`,
      }
    } catch (e) {
      message = { ok: false, text: (e as Error).message }
    } finally {
      saving = false
    }
  }

  function revert() {
    if (!file) return
    const { [file.name]: _, ...rest } = drafts
    drafts = rest
    message = null
  }

  function onkey(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === 's') {
      e.preventDefault()
      if (!file?.shipped) save()
    }
  }

  onMount(() => {
    load()
    const warn = (e: BeforeUnloadEvent) => {
      if (files.some(dirty)) e.preventDefault()
    }
    addEventListener('beforeunload', warn)
    return () => removeEventListener('beforeunload', warn)
  })
</script>

<div class="config">
  {#if loadError}
    <p class="warn pad">{loadError}</p>
  {:else}
    <div class="tabs">
      {#each files as f (f.name)}
        <button class:on={f.name === current} onclick={() => ((current = f.name), (message = null))}>
          {f.name}{dirty(f) ? ' •' : ''}
        </button>
      {/each}
    </div>
    {#if file}
      <p class="about">
        <code>{file.path}</code>
        {#if file.kind === 'prompt'}
          · {file.used_by.length ? `used by the ${file.used_by.join(', ')} pass` : 'not used by any pass'}
        {/if}
      </p>
      {#if file.shipped}
        <p class="warn about">
          This prompt ships with Pan, and pand rewrites it when Pan ships a new one, so it cannot be saved here.
          Save your version as a new prompt, then point the pass's <code>prompt:</code> line in config.yml at it.
        </p>
      {/if}
      <textarea
        spellcheck="false"
        value={text}
        oninput={(e) => edit((e.target as HTMLTextAreaElement).value)}
        onkeydown={onkey}
      ></textarea>
      <div class="bar">
        <button onclick={() => save()} disabled={!dirty(file) || saving || file.shipped}>save</button>
        <button onclick={revert} disabled={!dirty(file) || saving}>revert</button>
        {#if file.kind === 'prompt'}
          <span class="copy">
            <input placeholder="my-caption.md" bind:value={copyName} />
            <button
              onclick={() => save(`prompts/${copyName.trim()}`)}
              disabled={!/^[A-Za-z0-9][A-Za-z0-9._-]*\.md$/.test(copyName.trim()) || saving}>save as new prompt</button
            >
          </span>
        {/if}
        <span class="hint">⌘S saves. pand reads these files when it starts: restart pand for a change to take effect.</span>
      </div>
      {#if message}
        <p class="msg" class:warn={!message.ok}>{message.text}</p>
      {/if}
    {/if}
    <p class="about dir">{dir}</p>
  {/if}
</div>

<style>
  .config {
    position: absolute;
    inset: 0;
    display: flex;
    flex-direction: column;
    background: var(--paper);
    padding: 0.6rem 0.9rem;
    gap: 0.4rem;
  }
  .pad { padding: 1rem; }
  .tabs { display: flex; flex-wrap: wrap; gap: 0.3rem; }
  .tabs button { font-size: 0.78rem; padding: 0.15rem 0.6rem; border-color: var(--rule-strong); }
  .tabs button.on { background: var(--ink); color: var(--paper); }
  .about { margin: 0; font-size: 0.74rem; color: var(--ink-soft); }
  .dir { color: var(--ink-faint); }
  textarea {
    flex: 1;
    min-height: 0;
    width: 100%;
    resize: none;
    font-family: var(--mono);
    font-size: 0.8rem;
    line-height: 1.45;
    padding: 0.6rem 0.7rem;
    border: 1px solid var(--rule-strong);
    tab-size: 2;
  }
  .bar { display: flex; align-items: center; gap: 0.5rem; flex-wrap: wrap; }
  .bar button { font-size: 0.8rem; }
  .copy { display: flex; gap: 0.3rem; }
  .copy input { font: inherit; font-size: 0.78rem; padding: 0.15rem 0.4rem; border: 1px solid var(--rule-strong); width: 11rem; }
  .hint { font-size: 0.72rem; color: var(--ink-faint); margin-left: auto; }
  .msg { margin: 0; font-size: 0.78rem; color: var(--live); white-space: pre-wrap; }
  .warn { color: var(--warn); }
</style>
