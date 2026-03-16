<script lang="ts">
	// Floating code symbols - subtle drifting background elements
	const symbols = [
		{ text: '{', size: 24, x: 8, y: 15, opacity: 0.03, duration: 28, driftX: 35, driftY: -50, delay: 0 },
		{ text: '/>', size: 20, x: 75, y: 30, opacity: 0.025, duration: 32, driftX: -25, driftY: -40, delay: 4 },
		{ text: '=>', size: 18, x: 45, y: 70, opacity: 0.035, duration: 24, driftX: 30, driftY: -35, delay: 8 },
		{ text: '}', size: 26, x: 88, y: 80, opacity: 0.025, duration: 30, driftX: -40, driftY: -60, delay: 2 },
		{ text: '</', size: 16, x: 20, y: 55, opacity: 0.03, duration: 36, driftX: 28, driftY: -45, delay: 11 },
		{ text: '()', size: 22, x: 60, y: 10, opacity: 0.025, duration: 34, driftX: -18, driftY: -30, delay: 6 },
		{ text: '[]', size: 14, x: 35, y: 90, opacity: 0.03, duration: 27, driftX: 38, driftY: -48, delay: 9 },
		{ text: '&&', size: 18, x: 92, y: 45, opacity: 0.02, duration: 31, driftX: -32, driftY: -38, delay: 14 },
		{ text: '**', size: 16, x: 5, y: 85, opacity: 0.025, duration: 29, driftX: 22, driftY: -42, delay: 7 },
		{ text: '::',  size: 20, x: 50, y: 40, opacity: 0.02, duration: 33, driftX: -15, driftY: -25, delay: 12 },
	];
</script>

<div style="position: fixed; inset: 0; z-index: 0; pointer-events: none; overflow: hidden;">
	{#each symbols as s, i}
		<span
			style="
				position: absolute;
				left: {s.x}%;
				top: {s.y}%;
				font-size: {s.size}px;
				opacity: {s.opacity};
				--float-opacity: {s.opacity};
				--drift-x: {s.driftX}px;
				--drift-y: {s.driftY}px;
				animation: float-drift {s.duration}s ease-in-out infinite;
				animation-delay: {s.delay}s;
				pointer-events: none;
				user-select: none;
				font-family: var(--font-mono);
				color: var(--accent-indigo);
				font-weight: 700;
			"
		>
			{s.text}
		</span>
	{/each}
</div>
