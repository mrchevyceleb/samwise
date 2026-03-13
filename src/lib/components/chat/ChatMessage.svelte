<script lang="ts">
	import type { AgentMessage } from '$lib/stores/agents';
	import ThinkingBlock from './ThinkingBlock.svelte';
	import ToolCallBlock from './ToolCallBlock.svelte';

	interface Props {
		message: AgentMessage;
	}

	let { message }: Props = $props();

	let isUser = $derived(message.role === 'user');
	let isAssistant = $derived(message.role === 'assistant');
	let isSystem = $derived(message.role === 'system');
</script>

<div style="
	display: flex; flex-direction: column;
	align-items: {isUser ? 'flex-end' : 'flex-start'};
	padding: 4px 0; gap: 2px;
">
	{#if isUser}
		<!-- User message -->
		<div style="
			max-width: 85%; padding: 8px 12px; border-radius: 12px 12px 2px 12px;
			background: rgba(255, 214, 10, 0.1); border: 1px solid rgba(255, 214, 10, 0.15);
			color: var(--text-primary); font-size: 13px; line-height: 1.5;
			white-space: pre-wrap; word-break: break-word;
		">
			{message.content}
		</div>
		<span style="font-size: 10px; color: var(--text-muted); padding: 0 4px;">
			{new Date(message.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
		</span>
	{:else if isAssistant}
		<!-- Assistant message -->
		<div style="max-width: 92%; display: flex; gap: 8px; align-items: flex-start;">
			<div style="
				width: 22px; height: 22px; border-radius: 6px; flex-shrink: 0;
				background: rgba(255, 214, 10, 0.15); display: flex;
				align-items: center; justify-content: center; font-size: 12px;
				margin-top: 2px;
			">
				A
			</div>
			<div style="flex: 1; min-width: 0;">
				{#if message.thinking}
					<ThinkingBlock content={message.thinking} />
				{/if}
				{#if message.content}
					<div style="
						padding: 6px 0; color: var(--text-primary);
						font-size: 13px; line-height: 1.6; white-space: pre-wrap;
						word-break: break-word;
					">
						{@html message.content}
					</div>
				{/if}
				{#if message.toolCalls && message.toolCalls.length > 0}
					<div style="display: flex; flex-direction: column; gap: 3px; margin-top: 4px;">
						{#each message.toolCalls as tc (tc.id)}
							<ToolCallBlock toolCall={tc} />
						{/each}
					</div>
				{/if}
				<span style="font-size: 10px; color: var(--text-muted);">
					{new Date(message.timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
				</span>
			</div>
		</div>
	{:else if isSystem}
		<!-- System message -->
		<div style="
			width: 100%; text-align: center; padding: 4px 12px;
			font-size: 11px; color: var(--text-muted); font-style: italic;
		">
			{message.content}
		</div>
	{:else}
		<!-- Tool message -->
		<div style="
			max-width: 90%; padding: 4px 10px; border-radius: 6px;
			background: rgba(88, 166, 255, 0.05); border: 1px solid rgba(88, 166, 255, 0.1);
			font-size: 11px; color: var(--text-secondary); font-family: var(--font-mono);
			white-space: pre-wrap; word-break: break-word;
		">
			{message.content}
		</div>
	{/if}
</div>
