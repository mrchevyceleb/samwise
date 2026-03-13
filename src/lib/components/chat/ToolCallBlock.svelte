<script lang="ts">
	import type { ToolCall } from '$lib/stores/agents';

	interface Props {
		toolCall: ToolCall;
	}

	let { toolCall }: Props = $props();
	let expanded = $state(false);
	let hovered = $state(false);

	const borderColors: Record<ToolCall['status'], string> = {
		pending: 'var(--text-muted)',
		running: 'var(--banana-yellow)',
		complete: 'var(--accent-green)',
		error: 'var(--accent-red)'
	};

	const bgColors: Record<ToolCall['status'], string> = {
		pending: 'rgba(110, 118, 129, 0.05)',
		running: 'rgba(255, 214, 10, 0.05)',
		complete: 'rgba(63, 185, 80, 0.05)',
		error: 'rgba(248, 81, 73, 0.05)'
	};

	const statusIcons: Record<ToolCall['status'], string> = {
		pending: '...',
		running: '>>',
		complete: 'ok',
		error: '!!'
	};

	let duration = $derived(
		toolCall.startedAt && toolCall.completedAt
			? `${((toolCall.completedAt - toolCall.startedAt) / 1000).toFixed(1)}s`
			: toolCall.startedAt ? 'running...' : ''
	);
</script>

<div
	style="
		margin: 4px 0; border-radius: 6px;
		border: 1px solid {borderColors[toolCall.status]};
		background: {bgColors[toolCall.status]};
		overflow: hidden; transition: all 0.15s ease;
	"
>
	<button
		style="
			display: flex; align-items: center; gap: 8px; width: 100%;
			padding: 5px 8px; background: {hovered ? 'rgba(255,255,255,0.03)' : 'transparent'};
			border: none; cursor: pointer; font-family: var(--font-mono);
			font-size: 11px; color: var(--text-secondary);
			transition: background 0.1s ease;
		"
		onclick={() => expanded = !expanded}
		onmouseenter={() => hovered = true}
		onmouseleave={() => hovered = false}
	>
		<span style="font-size: 10px; transition: transform 0.15s ease; transform: rotate({expanded ? '90deg' : '0deg'}); display: inline-block; color: var(--text-muted);">
			&#9654;
		</span>
		<span style="
			font-size: 9px; font-weight: 700; padding: 1px 5px; border-radius: 3px;
			background: {borderColors[toolCall.status]}22;
			color: {borderColors[toolCall.status]};
			{toolCall.status === 'running' ? 'animation: pulse-dot 1s ease-in-out infinite;' : ''}
		">
			{statusIcons[toolCall.status]}
		</span>
		<span style="font-weight: 600; color: var(--text-primary);">{toolCall.name}</span>
		{#if duration}
			<span style="margin-left: auto; font-size: 10px; color: var(--text-muted);">{duration}</span>
		{/if}
	</button>
	{#if expanded}
		<div style="padding: 6px 10px 8px; border-top: 1px solid rgba(255,255,255,0.05); font-size: 11px; font-family: var(--font-mono);">
			{#if Object.keys(toolCall.args).length > 0}
				<div style="margin-bottom: 6px;">
					<div style="font-size: 10px; color: var(--text-muted); margin-bottom: 3px; text-transform: uppercase; letter-spacing: 0.5px;">Arguments</div>
					<pre style="color: var(--text-secondary); white-space: pre-wrap; word-break: break-all; line-height: 1.4; margin: 0;">{JSON.stringify(toolCall.args, null, 2)}</pre>
				</div>
			{/if}
			{#if toolCall.result !== undefined}
				<div>
					<div style="font-size: 10px; color: var(--text-muted); margin-bottom: 3px; text-transform: uppercase; letter-spacing: 0.5px;">Result</div>
					<pre style="color: {toolCall.status === 'error' ? 'var(--accent-red)' : 'var(--text-secondary)'}; white-space: pre-wrap; word-break: break-all; line-height: 1.4; margin: 0; max-height: 200px; overflow-y: auto;">{toolCall.result}</pre>
				</div>
			{/if}
		</div>
	{/if}
</div>
