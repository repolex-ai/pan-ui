<script lang="ts">
  // Stars and flag over an image, Lightroom's way: a white flag is a pick, a
  // black flag a reject, no flag is neither. Nothing is drawn for an image
  // nobody has marked, so marks stand out against the many that are not.
  interface Props {
    rating: number
    flag: number
    big?: boolean
  }
  let { rating, flag, big = false }: Props = $props()
</script>

{#if rating > 0 || flag !== 0}
  <span class="badge" class:big>
    {#if flag !== 0}
      <svg class="flag" class:pick={flag > 0} viewBox="0 0 12 14" aria-label={flag > 0 ? 'picked' : 'rejected'}>
        <path d="M2 1v12" />
        <path d="M2 1.5h8l-2 3 2 3H2z" />
      </svg>
    {/if}
    {#if rating > 0}<span class="stars" aria-label="{rating} stars">{'★'.repeat(rating)}</span>{/if}
  </span>
{/if}

<style>
  .badge {
    position: absolute;
    left: 4px;
    bottom: 4px;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 1px 5px;
    background: rgba(0, 0, 0, 0.62);
    color: #f2d36b;
    font-size: 11px;
    line-height: 16px;
    pointer-events: none;
  }
  .badge.big { font-size: 15px; line-height: 22px; padding: 3px 9px; left: 10px; bottom: 10px; gap: 7px; }
  .stars { letter-spacing: 1px; }
  .flag { width: 11px; height: 13px; stroke-width: 1.4; stroke-linejoin: round; }
  .big .flag { width: 15px; height: 18px; }
  .flag path:first-child { stroke: #fff; fill: none; }
  .flag path:last-child { fill: #000; stroke: #fff; }
  .flag.pick path:last-child { fill: #fff; stroke: #fff; }
</style>
