<script lang="ts">
	import { onMount } from 'svelte';

	// Floating particles
	let particles = $state<Array<{
		id: number;
		x: number;
		y: number;
		size: number;
		duration: number;
		delay: number;
		emoji: string;
	}>>([]);

	const emojis = ['✨', '⭐', '🍌', '💛', '🌟', '⚡'];

	onMount(() => {
		// Generate floating particles
		particles = Array.from({ length: 12 }, (_, i) => ({
			id: i,
			x: Math.random() * 100,
			y: Math.random() * 100,
			size: 8 + Math.random() * 14,
			duration: 4 + Math.random() * 6,
			delay: Math.random() * -8,
			emoji: emojis[Math.floor(Math.random() * emojis.length)],
		}));
	});
</script>

<div class="loading-container">
	<!-- Floating background particles -->
	{#each particles as p (p.id)}
		<div
			class="particle"
			style="
				left: {p.x}%;
				top: {p.y}%;
				font-size: {p.size}px;
				animation-duration: {p.duration}s;
				animation-delay: {p.delay}s;
			"
		>
			{p.emoji}
		</div>
	{/each}

	<!-- Main banana animation -->
	<div class="banana-stage">
		<!-- Glow ring -->
		<div class="glow-ring"></div>

		<!-- Banana -->
		<div class="banana-bounce">
			<span class="banana-emoji">&#x1F34C;</span>
		</div>

		<!-- Shadow -->
		<div class="banana-shadow"></div>
	</div>

	<!-- Loading dots -->
	<div class="loading-dots">
		{#each [0, 1, 2] as i}
			<div
				class="dot"
				style="animation-delay: {i * 0.2}s;"
			></div>
		{/each}
	</div>
</div>

<style>
	.loading-container {
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		height: 100%;
		position: relative;
		overflow: hidden;
		background: radial-gradient(
			ellipse at center,
			rgba(255, 214, 10, 0.03) 0%,
			transparent 70%
		);
	}

	/* Floating particles */
	.particle {
		position: absolute;
		opacity: 0.15;
		animation: float-particle linear infinite;
		pointer-events: none;
		user-select: none;
	}

	@keyframes float-particle {
		0% {
			transform: translateY(0) rotate(0deg) scale(1);
			opacity: 0;
		}
		10% {
			opacity: 0.15;
		}
		90% {
			opacity: 0.15;
		}
		100% {
			transform: translateY(-120px) rotate(360deg) scale(0.5);
			opacity: 0;
		}
	}

	/* Banana stage */
	.banana-stage {
		position: relative;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 120px;
		height: 120px;
	}

	/* Glow ring */
	.glow-ring {
		position: absolute;
		width: 100px;
		height: 100px;
		border-radius: 50%;
		border: 2px solid rgba(255, 214, 10, 0.1);
		animation: ring-pulse 2.5s ease-in-out infinite;
	}

	@keyframes ring-pulse {
		0%, 100% {
			transform: scale(0.8);
			opacity: 0;
			border-color: rgba(255, 214, 10, 0.05);
		}
		50% {
			transform: scale(1.3);
			opacity: 1;
			border-color: rgba(255, 214, 10, 0.15);
		}
	}

	/* Banana bounce */
	.banana-bounce {
		animation: banana-hop 1.8s cubic-bezier(0.36, 0, 0.66, -0.56) infinite alternate;
		z-index: 2;
	}

	.banana-emoji {
		font-size: 64px;
		display: block;
		filter: drop-shadow(0 4px 20px rgba(255, 214, 10, 0.3));
		animation: banana-wiggle 1.8s ease-in-out infinite alternate;
	}

	@keyframes banana-hop {
		0% {
			transform: translateY(0);
		}
		100% {
			transform: translateY(-18px);
		}
	}

	@keyframes banana-wiggle {
		0% {
			transform: rotate(-8deg) scale(1);
		}
		50% {
			transform: rotate(0deg) scale(1.05);
		}
		100% {
			transform: rotate(8deg) scale(1);
		}
	}

	/* Shadow under banana */
	.banana-shadow {
		position: absolute;
		bottom: 8px;
		width: 50px;
		height: 10px;
		border-radius: 50%;
		background: rgba(255, 214, 10, 0.12);
		filter: blur(6px);
		animation: shadow-squish 1.8s cubic-bezier(0.36, 0, 0.66, -0.56) infinite alternate;
	}

	@keyframes shadow-squish {
		0% {
			transform: scaleX(1);
			opacity: 0.6;
		}
		100% {
			transform: scaleX(0.7);
			opacity: 0.3;
		}
	}

	/* Loading dots */
	.loading-dots {
		display: flex;
		gap: 6px;
		margin-top: 24px;
	}

	.dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--banana-yellow, #FFD60A);
		animation: dot-bounce 1.2s ease-in-out infinite;
	}

	@keyframes dot-bounce {
		0%, 80%, 100% {
			transform: scale(0.6);
			opacity: 0.3;
		}
		40% {
			transform: scale(1);
			opacity: 1;
		}
	}
</style>
