<script lang="ts">
	const bananas = [
		{ emoji: '🍌', size: 28, x: 8, y: 15, opacity: 0.04, duration: 25, driftX: 40, driftY: -60, delay: 0 },
		{ emoji: '🍌', size: 22, x: 75, y: 30, opacity: 0.03, duration: 30, driftX: -30, driftY: -45, delay: 3 },
		{ emoji: '🍌', size: 18, x: 45, y: 70, opacity: 0.05, duration: 22, driftX: 25, driftY: -35, delay: 7 },
		{ emoji: '🍌', size: 32, x: 90, y: 80, opacity: 0.03, duration: 28, driftX: -50, driftY: -70, delay: 2 },
		{ emoji: '🍌', size: 20, x: 20, y: 55, opacity: 0.04, duration: 35, driftX: 35, driftY: -50, delay: 10 },
		{ emoji: '🍌', size: 24, x: 60, y: 10, opacity: 0.03, duration: 32, driftX: -20, driftY: -30, delay: 5 },
		{ emoji: '🍌', size: 16, x: 35, y: 90, opacity: 0.04, duration: 26, driftX: 45, driftY: -55, delay: 8 },
	];
</script>

<div style="position: fixed; inset: 0; z-index: 0; pointer-events: none; overflow: hidden;">
	{#each bananas as b, i}
		<span
			style="
				position: absolute;
				left: {b.x}%;
				top: {b.y}%;
				font-size: {b.size}px;
				opacity: {b.opacity};
				--float-opacity: {b.opacity};
				--drift-x: {b.driftX}px;
				--drift-y: {b.driftY}px;
				animation: float-drift {b.duration}s ease-in-out infinite;
				animation-delay: {b.delay}s;
				pointer-events: none;
				user-select: none;
			"
		>
			{b.emoji}
		</span>
	{/each}
</div>
