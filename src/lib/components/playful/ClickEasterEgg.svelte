<script lang="ts">
	// Work-themed click easter eggs
	const emojis = ['--', '{;}', '()', '=>', '**', '[]', '<>', '//', '&&', '||', '!=', '::'];
	let particles: Array<{ id: number; x: number; y: number; emoji: string; rotation: number }> = $state([]);
	let nextId = 0;

	function handleClick(e: MouseEvent) {
		const emoji = emojis[Math.floor(Math.random() * emojis.length)];
		const id = nextId++;
		const rotation = Math.random() * 60 - 30;
		particles = [...particles, { id, x: e.clientX, y: e.clientY, emoji, rotation }];

		setTimeout(() => {
			particles = particles.filter(p => p.id !== id);
		}, 1000);
	}
</script>

<div
	style="position: fixed; inset: 0; z-index: 9999; pointer-events: none;"
>
	{#each particles as p (p.id)}
		<span
			style="
				position: absolute;
				left: {p.x}px;
				top: {p.y}px;
				font-size: 14px;
				font-family: var(--font-mono);
				font-weight: 700;
				color: var(--accent-indigo);
				pointer-events: none;
				animation: emoji-rise 1s ease-out forwards;
				z-index: 10000;
				opacity: 0.7;
				text-shadow: 0 0 8px rgba(99, 102, 241, 0.4);
			"
		>
			{p.emoji}
		</span>
	{/each}
</div>

<svelte:window onclick={handleClick} />
