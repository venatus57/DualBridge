<script lang="ts">
  // A stylized controller seen from the front, with its lightbar glowing in
  // `color`. DualSense models also show the five player LEDs.
  let {
    color = "#0040FF",
    dualsense = true,
    playerLeds = 0,
    width = 260,
  }: { color?: string; dualsense?: boolean; playerLeds?: number; width?: number } = $props();

  const leds = [0, 1, 2, 3, 4];
  // Gradient IDs must be unique when several pads are on the page.
  const uid = Math.random().toString(36).slice(2, 8);
</script>

<svg viewBox="0 0 300 190" {width} class="pad" style="--glow:{color}" aria-hidden="true">
  <defs>
    <radialGradient id="glow-{uid}" cx="50%" cy="50%" r="50%">
      <stop offset="0" stop-color={color} stop-opacity="0.75" />
      <stop offset="1" stop-color={color} stop-opacity="0" />
    </radialGradient>
    <linearGradient id="body-{uid}" x1="0" y1="0" x2="0" y2="1">
      <stop offset="0" stop-color={dualsense ? "#f4f6fa" : "#2b3140"} />
      <stop offset="1" stop-color={dualsense ? "#cfd5e0" : "#1b2029"} />
    </linearGradient>
  </defs>
  <ellipse cx="150" cy="38" rx="120" ry="34" fill="url(#glow-{uid})" />
  <path
    d="M88 40h124c34 0 52 20 60 58l14 52c6 26-12 40-32 40-16 0-26-10-36-26l-12-20H94l-12 20c-10 16-20 26-36 26-20 0-38-14-32-40l14-52c8-38 26-58 60-58z"
    fill="url(#body-{uid})"
    stroke={dualsense ? "#aab3c3" : "#3a4254"}
    stroke-width="1.5"
  />
  <!-- touchpad with lightbar edges -->
  <rect x="104" y="46" width="92" height="52" rx="10" fill={dualsense ? "#e3e7ee" : "#232936"} stroke={dualsense ? "#b8c0cd" : "#3a4254"} />
  {#if dualsense}
    <path d="M100 52 q-4 24 6 44" stroke={color} stroke-width="4" fill="none" stroke-linecap="round" class="bar" />
    <path d="M200 52 q4 24 -6 44" stroke={color} stroke-width="4" fill="none" stroke-linecap="round" class="bar" />
    {#each leds as i (i)}
      <circle cx={138 + i * 6} cy="104" r="1.9" fill={playerLeds & (1 << i) ? "#ffffff" : "#b7bfcc"} class:on={playerLeds & (1 << i)} />
    {/each}
  {:else}
    <rect x="112" y="40" width="76" height="6" rx="3" fill={color} class="bar" />
  {/if}
  <!-- D-pad -->
  <g fill={dualsense ? "#9aa4b5" : "#4a5367"}>
    <rect x="62" y="70" width="10" height="30" rx="2" />
    <rect x="52" y="80" width="30" height="10" rx="2" />
  </g>
  <!-- face buttons -->
  <g fill={dualsense ? "#9aa4b5" : "#4a5367"}>
    <circle cx="238" cy="72" r="6" />
    <circle cx="238" cy="98" r="6" />
    <circle cx="225" cy="85" r="6" />
    <circle cx="251" cy="85" r="6" />
  </g>
  <!-- sticks -->
  <circle cx="112" cy="124" r="15" fill={dualsense ? "#2d3340" : "#11151c"} />
  <circle cx="188" cy="124" r="15" fill={dualsense ? "#2d3340" : "#11151c"} />
</svg>

<style>
  .pad {
    display: block;
    height: auto;
    filter: drop-shadow(0 10px 18px rgba(0, 0, 0, 0.45));
  }
  .bar {
    filter: drop-shadow(0 0 6px var(--glow));
  }
  .on {
    filter: drop-shadow(0 0 3px #fff);
  }
</style>
