<script lang="ts">
	let hovered = $state(false);
</script>

<div style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; gap: 16px; padding: 32px;">
	<!-- Big banana -->
	<div
		style="font-size: 72px; animation: bob 3s ease-in-out infinite; cursor: default; transition: transform 0.2s ease; transform: {hovered ? 'scale(1.15) rotate(10deg)' : 'scale(1)'}; filter: {hovered ? 'drop-shadow(0 0 20px rgba(255, 214, 10, 0.4))' : 'none'};"
		onmouseenter={() => hovered = true}
		onmouseleave={() => hovered = false}
		role="img"
		aria-label="Banana"
	>
		🍌
	</div>

	<div style="text-align: center;">
		<p style="font-size: 16px; font-weight: 600; color: var(--text-primary); margin-bottom: 6px;">
			No preview active
		</p>
		<p style="font-size: 13px; color: var(--text-muted); max-width: 280px; line-height: 1.5;">
			Open a project to see your app come to life. The preview will appear here automatically.
		</p>
	</div>

	<!-- Decorative dots -->
	<div style="display: flex; gap: 6px; margin-top: 8px;">
		{#each [0, 1, 2] as i}
			<div style="width: 6px; height: 6px; border-radius: 50%; background: var(--banana-yellow); opacity: 0.3; animation: pulse-dot 2s ease-in-out infinite; animation-delay: {i * 0.3}s;"></div>
		{/each}
	</div>
</div>
