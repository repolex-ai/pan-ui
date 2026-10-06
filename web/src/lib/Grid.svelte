<script lang="ts">
  // The grid: every image of the set as a thumbnail, in time order.
  //
  // A set can be 200,000 images, so only the rows on screen (and a few either
  // side) exist as elements; the rest is one tall empty box that gives the
  // scrollbar its length. Scrolling moves which rows exist, nothing else.
  import type { Images } from './api'
  import { thumbUrl } from './api'
  import Badge from './Badge.svelte'

  interface Props {
    images: Images
    cursor: number
    size: number
    onpick: (i: number) => void
    onopen: (i: number) => void
    /** Columns, reported up so the arrow keys can move by rows. */
    cols?: number
  }
  let { images, cursor, size, onpick, onopen, cols = $bindable(1) }: Props = $props()

  let box = $state<HTMLDivElement | null>(null)
  let width = $state(0)
  let height = $state(0)
  let top = $state(0)

  const GAP = 6
  const OVERSCAN = 3
  const n = $derived(images.ids.length)
  $effect(() => {
    cols = Math.max(1, Math.floor((width - GAP) / (size + GAP)))
  })
  const cell = $derived(cols > 0 ? (width - GAP) / cols - GAP : size)
  const pitch = $derived(cell + GAP)
  const rows = $derived(Math.ceil(n / cols))
  const first = $derived(Math.max(0, Math.floor(top / pitch) - OVERSCAN))
  const last = $derived(Math.min(rows - 1, Math.ceil((top + height) / pitch) + OVERSCAN))
  const visible = $derived.by(() => {
    const out: number[] = []
    for (let r = first; r <= last; r++) {
      for (let c = 0; c < cols; c++) {
        const i = r * cols + c
        if (i < n) out.push(i)
      }
    }
    return out
  })

  // Keep the cursor on screen: when it moves off the top or bottom, scroll
  // just far enough to bring its row back, so the grid does not jump.
  $effect(() => {
    if (!box || cursor < 0) return
    const y = Math.floor(cursor / cols) * pitch
    if (y < box.scrollTop) box.scrollTop = y
    else if (y + pitch > box.scrollTop + height) box.scrollTop = y + pitch - height + GAP
  })
</script>

<div
  class="grid"
  bind:this={box}
  bind:clientWidth={width}
  bind:clientHeight={height}
  onscroll={() => (top = box?.scrollTop ?? 0)}
>
  <div class="spacer" style="height:{rows * pitch + GAP}px">
    {#each visible as i (images.ids[i])}
      {@const r = Math.floor(i / cols)}
      {@const c = i % cols}
      <button
        class="cell"
        class:on={i === cursor}
        class:rejected={images.flag[i] < 0}
        style="left:{GAP + c * pitch}px;top:{GAP + r * pitch}px;width:{cell}px;height:{cell}px"
        onclick={() => onpick(i)}
        ondblclick={() => onopen(i)}
        tabindex="-1"
      >
        <img src={thumbUrl(images.ids[i])} alt="" decoding="async" draggable="false" />
        <Badge rating={images.rating[i]} flag={images.flag[i]} />
      </button>
    {/each}
  </div>
</div>

<style>
  .grid {
    position: absolute;
    inset: 0;
    overflow-y: auto;
    background: var(--stage);
  }
  .spacer { position: relative; }
  .cell {
    position: absolute;
    padding: 0;
    border: 0;
    background: var(--stage-cell);
    display: block;
    outline: 2px solid transparent;
    outline-offset: 1px;
  }
  .cell:hover { background: var(--stage-cell); }
  .cell.on { outline-color: var(--stage-ink); }
  .cell img {
    width: 100%;
    height: 100%;
    object-fit: contain;
    display: block;
  }
  .cell.rejected img { opacity: 0.28; }
</style>
