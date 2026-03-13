<script lang="ts">
	interface Props {
		content: string;
		isActive?: boolean;
	}

	let { content, isActive = false }: Props = $props();
	let expanded = $state(false);
	let hovered = $state(false);
</script>

<div
	style="
		margin: 4px 0; border-radius: 6px;
		border: 1px solid rgba(255, 214, 10, 0.1);
		background: rgba(255, 214, 10, 0.03);
		overflow: hidden; transition: all 0.15s ease;
	"
>
	<button
		style="
			display: flex; align-items: center; gap: 6px; width: 100%;
			padding: 5px 8px; background: {hovered ? 'rgba(255, 214, 10, 0.06)' : 'transparent'};
			border: none; cursor: pointer; font-family: var(--font-ui);
			font-size: 11px; color: var(--banana-yellow-dim);
			transition: background 0.1s ease;
		"
		onclick={() => expanded = !expanded}
		onmouseenter={() => hovered = true}
		onmouseleave={() => hovered = false}
	>
		<span style="font-size: 10px; transition: transform 0.15s ease; transform: rotate({expanded ? '90deg' : '0deg'}); display: inline-block;">
			&#9654;
		</span>
		<span style="font-style: italic;">
			{#if isActive}
				Thinking<span class="thinking-anim">...</span>
			{:else}
				Thought process
			{/if}
		</span>
	</button>
	{#if expanded}
		<div style="padding: 6px 10px 8px; font-size: 12px; color: var(--text-muted); font-style: italic; line-height: 1.5; white-space: pre-wrap; font-family: var(--font-mono); border-top: 1px solid rgba(255, 214, 10, 0.08);">
			{content}
		</div>
	{/if}
</div>

<style>
	.thinking-anim {
		display: inline-block;
		animation: thinking-dots 1.4s steps(4, end) infinite;
		width: 1.2em;
		overflow: hidden;
		vertical-align: bottom;
	}
	@keyframes thinking-dots {
		0% { width: 0; }
		100% { width: 1.2em; }
	}
</style>
