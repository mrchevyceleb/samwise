<script lang="ts">
	let hovered = $state(false);
</script>

<div style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; gap: 20px; padding: 32px; position: relative; overflow: hidden;">
	<!-- Radial glow behind banana -->
	<div style="position: absolute; width: 300px; height: 300px; border-radius: 50%; background: radial-gradient(circle, color-mix(in srgb, var(--accent-primary) 4%, transparent) 0%, transparent 70%); pointer-events: none;"></div>

	<!-- Big banana -->
	<div
		style="font-size: 80px; animation: bob 3s ease-in-out infinite; cursor: default; transition: transform 0.3s ease, filter 0.3s ease; transform: {hovered ? 'scale(1.2) rotate(12deg)' : 'scale(1)'}; filter: drop-shadow(0 0 16px color-mix(in srgb, var(--accent-primary) 20%, transparent)) {hovered ? 'drop-shadow(0 0 30px color-mix(in srgb, var(--accent-primary) 50%, transparent))' : ''}; position: relative; z-index: 1;"
		onmouseenter={() => hovered = true}
		onmouseleave={() => hovered = false}
		role="img"
		aria-label="Banana"
	>
		🍌
	</div>

	<div style="text-align: center; position: relative; z-index: 1;">
		<p style="font-size: 17px; font-weight: 700; color: var(--text-primary); margin-bottom: 8px; letter-spacing: -0.3px;">
			No preview active
		</p>
		<p style="font-size: 13px; color: var(--text-muted); max-width: 280px; line-height: 1.6;">
			Open a project to see your app come to life. The preview will appear here automatically.
		</p>
	</div>

	<!-- Decorative dots -->
	<div style="display: flex; gap: 8px; margin-top: 4px;">
		{#each [0, 1, 2] as i}
			<div style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent-primary); opacity: 0.4; animation: pulse-dot 2s ease-in-out infinite; animation-delay: {i * 0.3}s; box-shadow: 0 0 8px color-mix(in srgb, var(--accent-primary) 30%, transparent);"></div>
		{/each}
	</div>
</div>
