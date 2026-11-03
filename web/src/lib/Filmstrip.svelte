<script lang="ts">
  // The filmstrip: the whole set as one row of thumbnails under the large
  // image, so you can see where you are in it and jump with one click.
  //
  // Like the grid, only the cells on screen exist; a set of 200,000 is one
  // wide empty box. The current image is kept in the middle of the strip.
  import type { Images } from './api'
  import { thumbUrl } from './api'
  import Badge from './Badge.svelte'

  interface Props {
    images: Images
    cursor: number
    onpick: (i: number) => void
  }
  let { images, cursor, onpick }: Props = $props()

  let box = $state<HTMLDivElement | null>(null)
  let width = $state(0)
  let height = $state(0)
  let left = $state(0)

  const GAP = 6
  const OVERSCAN = 4
  const n = $derived(images.ids.length)
  const cell = $derived(Math.max(40, height - 2 * GAP))
  const pitch = $derived(cell + GAP)
  const first = $derived(Math.max(0, Math.floor(left / pitch) - OVERSCAN))
  const last = $derived(Math.min(n - 1, Math.ceil((left + width) / pitch) + OVERSCAN))
  const visible = $derived.by(() => {
    const out: number[] = []
    for (let i = first; i <= last; i++) out.push(i)
    return out
  })

  $effect(() => {
    if (!box || cursor < 0) return
    box.scrollLeft = GAP + cursor * pitch + cell / 2 - width / 2
  })

  // A mouse wheel scrolls up and down; the strip runs sideways.
  function wheel(e: WheelEvent) {
    if (!box || Math.abs(e.deltaX) > Math.abs(e.deltaY)) return
    e.preventDefault()
    box.scrollLeft += e.deltaY
  }
</script>

<div
  class="strip"
  bind:this={box}
  bind:clientWidth={width}
  bind:clientHeight={height}
  onscroll={() => (left = box?.scrollLeft ?? 0)}
  onwheel={wheel}
>
  <div class="spacer" style="width:{n * pitch + GAP}px">
    {#each visible as i (images.ids[i])}
      <button
        class="cell"
        class:on={i === cursor}
        class:rejected={images.flag[i] < 0}
        style="left:{GAP + i * pitch}px;top:{GAP}px;width:{cell}px;height:{cell}px"
        onclick={() => onpick(i)}
        tabindex="-1"
      >
        <img src={thumbUrl(images.ids[i])} alt="" decoding="async" draggable="false" />
        <Badge rating={images.rating[i]} flag={images.flag[i]} />
      </button>
    {/each}
  </div>
</div>

<style>
  .strip {
    position: absolute;
    inset: 0;
    overflow-x: auto;
    overflow-y: hidden;
    background: var(--stage);
    border-top: 1px solid #000;
  }
  .spacer { position: relative; height: 100%; }
  .cell {
    position: absolute;
    padding: 0;
    border: 0;
    background: var(--stage-cell);
    outline: 2px solid transparent;
    outline-offset: 1px;
  }
  .cell:hover { background: var(--stage-cell); }
  .cell.on { outline-color: var(--stage-ink); }
  .cell img { width: 100%; height: 100%; object-fit: contain; display: block; }
  .cell.rejected img { opacity: 0.28; }
</style>
