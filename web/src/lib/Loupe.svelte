<script lang="ts">
  // One image, as large as the stage allows.
  //
  // Moving to the next image swaps in an element that is already decoded
  // (see pixels.ts), so there is nothing to wait for. If the arrow key is
  // held down faster than images can arrive, the thumbnail is shown in the
  // meantime, scaled up, and the full image replaces it the moment it is
  // ready, unless the cursor has already moved on.
  import type { Images } from './api'
  import { thumbUrl } from './api'
  import * as pixels from './pixels'
  import Badge from './Badge.svelte'

  interface Props {
    images: Images
    cursor: number
    dir: number
    onclose: () => void
  }
  let { images, cursor, dir, onclose }: Props = $props()

  let frame = $state<HTMLDivElement | null>(null)
  let sharp = $state(false)
  const thumb = new Image()
  thumb.className = 'thumb'
  thumb.draggable = false

  function show(el: HTMLImageElement) {
    if (!frame) return
    if (frame.firstChild !== el) frame.replaceChildren(el)
  }

  $effect(() => {
    const ids = images.ids
    const i = cursor
    if (!frame || i < 0 || i >= ids.length) return
    const id = ids[i]
    const ready = pixels.ready(id)
    if (ready) {
      show(ready)
      sharp = true
    } else {
      thumb.src = thumbUrl(id)
      show(thumb)
      sharp = false
      pixels.get(id).ready.then(
        (img) => {
          if (images.ids[cursor] === id) {
            show(img)
            sharp = true
          }
        },
        () => {},
      )
    }
    // Warm what comes next only after this one is asked for, so the image
    // you are looking at is first in line.
    pixels.around(ids, i, dir)
  })
</script>

<div class="loupe" ondblclick={onclose} role="presentation">
  <div class="frame" class:soft={!sharp} bind:this={frame}></div>
  {#if cursor >= 0 && cursor < images.ids.length}
    <Badge rating={images.rating[cursor]} flag={images.flag[cursor]} big />
  {/if}
</div>

<style>
  .loupe {
    position: absolute;
    inset: 0;
    background: var(--stage);
  }
  .frame {
    position: absolute;
    inset: 12px;
    display: flex;
    align-items: center;
    justify-content: center;
  }
  .frame :global(img) {
    max-width: 100%;
    max-height: 100%;
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
    user-select: none;
  }
  /* The stand-in thumbnail is a third of the size: say so by softening it a
     touch rather than pretending it is the real image. */
  .frame.soft :global(img) { filter: blur(0.6px); }
</style>
