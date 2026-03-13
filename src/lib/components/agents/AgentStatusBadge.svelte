<script lang="ts">
	import type { Agent } from '$lib/stores/agents';

	interface Props {
		status: Agent['status'];
		size?: 'sm' | 'md';
	}

	let { status, size = 'sm' }: Props = $props();

	const colors: Record<Agent['status'], string> = {
		idle: '#6E7681',
		thinking: '#FFD60A',
		writing: '#58A6FF',
		running_tool: '#D29922',
		done: '#3FB950',
		error: '#F85149'
	};

	const labels: Record<Agent['status'], string> = {
		idle: 'Idle',
		thinking: 'Thinking',
		writing: 'Writing',
		running_tool: 'Running',
		done: 'Done',
		error: 'Error'
	};

	let dotSize = $derived(size === 'sm' ? 7 : 9);
	let fontSize = $derived(size === 'sm' ? 10 : 11);
</script>

<div style="display: inline-flex; align-items: center; gap: {size === 'sm' ? 4 : 6}px;">
	<span
		style="
			width: {dotSize}px;
			height: {dotSize}px;
			border-radius: 50%;
			background: {colors[status]};
			display: inline-block;
			flex-shrink: 0;
			{status === 'thinking' ? 'animation: pulse-dot 1.2s ease-in-out infinite;' : ''}
			{status === 'writing' || status === 'running_tool' ? 'animation: pulse-dot 0.8s ease-in-out infinite;' : ''}
		"
	></span>
	{#if size === 'md'}
		<span style="font-size: {fontSize}px; color: {colors[status]}; font-weight: 500;">
			{labels[status]}
			{#if status === 'thinking'}
				<span class="thinking-dots">...</span>
			{/if}
			{#if status === 'writing'}
				<span class="writing-dots">...</span>
			{/if}
		</span>
	{/if}
</div>

<style>
	.thinking-dots {
		display: inline-block;
		animation: thinking-ellipsis 1.4s steps(4, end) infinite;
		width: 1em;
		overflow: hidden;
		vertical-align: bottom;
	}

	.writing-dots {
		display: inline-block;
		animation: thinking-ellipsis 0.9s steps(4, end) infinite;
		width: 1em;
		overflow: hidden;
		vertical-align: bottom;
	}

	@keyframes thinking-ellipsis {
		0% { width: 0; }
		100% { width: 1.2em; }
	}
</style>
