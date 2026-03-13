<script lang="ts">
	const emojis = ['🍌', '🍎', '🍑', '🍒', '🍓', '🥭', '🍊', '🍋', '🫐', '🍇'];
	let particles: Array<{ id: number; x: number; y: number; emoji: string }> = $state([]);
	let nextId = 0;

	function handleClick(e: MouseEvent) {
		const emoji = emojis[Math.floor(Math.random() * emojis.length)];
		const id = nextId++;
		particles = [...particles, { id, x: e.clientX, y: e.clientY, emoji }];

		setTimeout(() => {
			particles = particles.filter(p => p.id !== id);
		}, 1000);
	}
</script>

<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	style="position: fixed; inset: 0; z-index: 9999; pointer-events: none;"
>
	{#each particles as p (p.id)}
		<span
			style="position: absolute; left: {p.x}px; top: {p.y}px; font-size: 24px; pointer-events: none; animation: emoji-rise 1s ease-out forwards; z-index: 10000;"
		>
			{p.emoji}
		</span>
	{/each}
</div>

<!-- Invisible click catcher - sits behind everything interactive but catches clicks on empty space -->
<svelte:window onclick={handleClick} />
