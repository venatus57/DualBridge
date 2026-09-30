<script lang="ts">
  // A stylized controller seen from the front, with its lightbar glowing in
  // `color`. DualSense models also show the five player LEDs; the Switch Pro
  // Controller has no lightbar, only four player LEDs (lit by count).
  let {
    color = "#0040FF",
    dualsense = true,
    switchPro = false,
    playerLeds = 0,
    width = 260,
  }: { color?: string; dualsense?: boolean; switchPro?: boolean; playerLeds?: number; width?: number } = $props();

  const leds = [0, 1, 2, 3, 4];
  const proLit = $derived(Math.min(4, [0, 1, 2, 3, 4].filter((i) => playerLeds & (1 << i)).length));
  // Gradient IDs must be unique when several pads are on the page.
  const uid = Math.random().toString(36).slice(2, 8);
</script>

{#if switchPro}
  <svg viewBox="0 0 300 190" {width} class="pad" aria-hidden="true">
    <defs>
      <linearGradient id="pro-{uid}" x1="0" y1="0" x2="0" y2="1">
        <stop offset="0" stop-color="#3a3f48" />
        <stop offset="1" stop-color="#1c1f25" />
      </linearGradient>
    </defs>
    <path
      d="M86 36h128c34 0 54 22 62 60l12 54c6 26-12 40-32 40-16 0-26-10-36-26l-12-20H92l-12 20c-10 16-20 26-36 26-20 0-38-14-32-40l12-54c8-38 28-60 62-60z"
      fill="url(#pro-{uid})"
      stroke="#4a505c"
      stroke-width="1.5"
    />
    <!-- Minus / Plus, Capture / Home -->
    <rect x="118" y="58" width="12" height="3" rx="1.5" fill="#8a93a3" />
    <g fill="#8a93a3"><rect x="170" y="58" width="12" height="3" rx="1.5" /><rect x="174.5" y="53.5" width="3" height="12" rx="1.5" /></g>
    <rect x="128" y="80" width="10" height="10" rx="2" fill="#2a2e36" stroke="#5a6170" />
    <circle cx="170" cy="85" r="6" fill="#2a2e36" stroke="#5a6170" />
    <!-- Player LEDs -->
    {#each [0, 1, 2, 3] as i (i)}
      <rect x={133 + i * 9} y="150" width="6" height="3" rx="1" fill={i < proLit ? "#9dff6a" : "#3a404b"} class:lit={i < proLit} />
    {/each}
    <!-- Left stick high, D-pad low (asymmetric) -->
    <circle cx="80" cy="78" r="17" fill="#15171b" stroke="#4a505c" />
    <circle cx="80" cy="78" r="10" fill="#2a2e36" />
    <g fill="#4a5262">
      <rect x="108" y="104" width="10" height="28" rx="2" />
      <rect x="99" y="113" width="28" height="10" rx="2" />
    </g>
    <!-- Face buttons high, right stick low -->
    <g fill="#4a5262">
      <circle cx="226" cy="64" r="7" />
      <circle cx="226" cy="92" r="7" />
      <circle cx="212" cy="78" r="7" />
      <circle cx="240" cy="78" r="7" />
    </g>
    <circle cx="190" cy="118" r="17" fill="#15171b" stroke="#4a505c" />
    <circle cx="190" cy="118" r="10" fill="#2a2e36" />
  </svg>
{:else}
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
{/if}

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
  .lit {
    filter: drop-shadow(0 0 3px #9dff6a);
  }
</style>
