<script lang="ts">
  // What a key just did to the image, at the foot of the stage, so the
  // result reads at a glance without looking away to find the badge. Only
  // the mark that changed: five star places for a rating, the flag itself
  // (white pick, black reject, empty outline for none) for a flag. Brief:
  // it only says the mark took.
  interface Props {
    kind: 'rating' | 'flag'
    rating: number
    flag: number
    /** Bumped on every mark, so the same result shown twice still replays. */
    n: number
  }
  let { kind, rating, flag, n }: Props = $props()
</script>

{#key n}
  <div class="toast" role="status">
    {#if kind === 'rating'}
      <span class="stars" aria-label="{rating} stars">
        {#each [1, 2, 3, 4, 5] as k (k)}<span class:on={k <= rating}>★</span>{/each}
      </span>
    {:else}
      <svg class:pick={flag > 0} class:none={flag === 0} viewBox="0 0 12 14" aria-label={flag > 0 ? 'picked' : flag < 0 ? 'rejected' : 'no flag'}>
        <path d="M2 1v12" /><path d="M2 1.5h8l-2 3 2 3H2z" />
      </svg>
    {/if}
  </div>
{/key}

<style>
  .toast {
    position: absolute;
    left: 50%;
    bottom: 28px;
    transform: translateX(-50%);
    display: flex;
    align-items: center;
    padding: 6px 20px;
    background: rgba(0, 0, 0, 0.72);
    pointer-events: none;
    animation: fade 0.9s ease-in forwards;
    z-index: 5;
  }
  @keyframes fade {
    0%, 40% { opacity: 1; }
    100% { opacity: 0; }
  }
  .stars { font-size: 34px; line-height: 40px; letter-spacing: 5px; color: #555; }
  .stars .on { color: #f2d36b; }
  svg { width: 34px; height: 40px; stroke-width: 1.1; stroke-linejoin: round; }
  path:first-child { stroke: #fff; fill: none; }
  path:last-child { fill: #000; stroke: #fff; }
  .pick path:last-child { fill: #fff; }
  .none path { stroke: #999; }
  .none path:last-child { fill: none; }
</style>
