<script lang="ts">
  const emojis = ['✨', '🍌', '🌱', '💫', '🚀', '🪄', '🍀', '🌀', '🎈', '🔮'];
  type Spark = { id: number; x: number; y: number; emoji: string };
  let sparks = $state<Spark[]>([]);
  let nextId = 0;

  function handleClick(e: MouseEvent) {
    const target = e.target as HTMLElement;
    if (target.closest('a, button, input, textarea, [data-no-egg]')) return;
    const id = ++nextId;
    const emoji = emojis[Math.floor(Math.random() * emojis.length)];
    sparks = [...sparks, { id, x: e.clientX, y: e.clientY, emoji }];
    setTimeout(() => {
      sparks = sparks.filter((s) => s.id !== id);
    }, 900);
  }
</script>

<svelte:window on:click={handleClick} />

<div class="pointer-events-none fixed inset-0 z-50">
  {#each sparks as s (s.id)}
    <span
      class="absolute text-2xl"
      style="left:{s.x}px;top:{s.y}px;animation:pop-in 900ms ease-out forwards;transform:translate(-50%, -50%)"
    >{s.emoji}</span>
  {/each}
</div>
