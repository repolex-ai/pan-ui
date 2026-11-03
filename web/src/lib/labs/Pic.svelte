<script lang="ts">
  // One image in a Lab view: a thumbnail that can be clicked, with an
  // optional line under it. Double-click opens it in the Library.
  import { thumbUrl } from '../api'

  interface Props {
    id: string
    note?: string | null
    size?: number
    on?: boolean
    onclick?: (id: string, e: MouseEvent) => void
    onopen?: (id: string) => void
  }
  let { id, note = null, size = 140, on = false, onclick, onopen }: Props = $props()
</script>

<button
  class="pic"
  class:on
  style="width:{size}px"
  onclick={(e) => onclick?.(id, e)}
  ondblclick={() => onopen?.(id)}
  title="{id} · click to use, double-click to open in the Library"
>
  <span class="frame" style="height:{size}px"><img src={thumbUrl(id)} alt="" loading="lazy" decoding="async" draggable="false" /></span>
  {#if note}<span class="note">{note}</span>{/if}
</button>

<style>
  .pic {
    display: inline-flex;
    flex-direction: column;
    gap: 0.15rem;
    padding: 0;
    border: 0;
    background: none;
    color: var(--stage-soft, #999);
    font-size: 0.66rem;
    text-align: left;
  }
  .pic:hover { background: none; }
  .frame { display: block; background: var(--stage-cell, #222); outline: 2px solid transparent; outline-offset: 1px; }
  .pic:hover .frame { outline-color: #666; }
  .pic.on .frame { outline-color: var(--stage-ink, #eee); }
  img { width: 100%; height: 100%; object-fit: contain; display: block; }
  .note { white-space: nowrap; overflow: hidden; text-overflow: ellipsis; }
</style>
