<script module lang="ts">
  export type Shape = "cross" | "circle" | "square" | "triangle";
</script>

<script lang="ts">
  // A PlayStation face-button symbol, drawn as a stroke.
  let { shape, size = 16, stroke = 2.2 }: { shape: Shape; size?: number; stroke?: number } = $props();
</script>

<svg
  width={size}
  height={size}
  viewBox="0 0 24 24"
  fill="none"
  stroke="currentColor"
  stroke-width={stroke}
  stroke-linecap="round"
  stroke-linejoin="round"
  aria-hidden="true"
>
  {#if shape === "cross"}
    <path d="M5 5l14 14M19 5L5 19" />
  {:else if shape === "circle"}
    <circle cx="12" cy="12" r="8" />
  {:else if shape === "square"}
    <rect x="4.5" y="4.5" width="15" height="15" />
  {:else}
    <path d="M12 4l9 15.5H3z" />
  {/if}
</svg>
