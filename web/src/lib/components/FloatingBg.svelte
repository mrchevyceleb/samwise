<script lang="ts">
  const blobs = Array.from({ length: 10 }, (_, i) => ({
    left: Math.random() * 100,
    top: Math.random() * 100,
    size: 80 + Math.random() * 160,
    delay: -Math.random() * 14,
    hue: 190 + Math.random() * 80
  }));
</script>

<div class="pointer-events-none fixed inset-0 -z-10 overflow-hidden">
  {#each blobs as b}
    <div
      class="drift absolute rounded-full opacity-20 blur-3xl"
      style="left:{b.left}%;top:{b.top}%;width:{b.size}px;height:{b.size}px;background:hsl({b.hue} 80% 55%);animation-delay:{b.delay}s"
    ></div>
  {/each}
</div>
