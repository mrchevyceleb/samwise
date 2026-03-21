<script lang="ts">
	import type { AeMessage } from '$lib/types';
	import { renderMarkdown } from '$lib/utils/markdown';
	import { getTheme } from '$lib/stores/theme.svelte';
	import AgentAvatar from './AgentAvatar.svelte';

	interface Props {
		message: AeMessage;
	}

	let { message }: Props = $props();
	const theme = getTheme();

	let isUser = $derived(message.role === 'user');
	let isAgent = $derived(message.role === 'agent');
	let isSystem = $derived(message.role === 'system');

	let renderedContent = $state(message.content);

	$effect(() => {
		if (isAgent && message.content) {
			renderMarkdown(message.content).then(html => {
				renderedContent = html;
			});
		} else {
			renderedContent = message.content;
		}
	});

	function formatTime(dateStr: string): string {
		const date = new Date(dateStr);
		const now = new Date();
		const diffMs = now.getTime() - date.getTime();
		const diffMin = Math.floor(diffMs / 60000);

		if (diffMin < 1) return 'just now';
		if (diffMin < 60) return `${diffMin}m ago`;

		const diffHr = Math.floor(diffMin / 60);
		if (diffHr < 24) return `${diffHr}h ago`;

		return date.toLocaleDateString([], { month: 'short', day: 'numeric' });
	}
</script>

<div style="
	display: flex; flex-direction: column;
	align-items: {isUser ? 'flex-end' : isSystem ? 'center' : 'flex-start'};
	padding: 4px 0; gap: 2px;
	animation: {isUser ? 'slide-in-right' : isSystem ? 'fade-in' : 'slide-in-left'} 0.3s ease;
">
	{#if isUser}
		<div style="
			max-width: 85%; padding: 10px 14px; border-radius: 14px 14px 4px 14px;
			background: {theme.c.accentPrimary};
			color: #fff; font-size: 14px; line-height: 1.5;
			white-space: pre-wrap; word-break: break-word;
			box-shadow: 0 2px 8px {theme.c.accentGlow};
		">
			{message.content}
		</div>
		<span style="font-size: 10px; color: {theme.c.textMuted}; padding: 0 4px;">
			{formatTime(message.created_at)}
		</span>
	{:else if isAgent}
		<div style="max-width: 92%; display: flex; gap: 8px; align-items: flex-start;">
			<AgentAvatar size={24} />
			<div style="flex: 1; min-width: 0;">
				<div class="agent-msg-content" style="
					padding: 10px 14px; border-radius: 4px 14px 14px 14px;
					background: {theme.c.bgElevated};
					border: 1px solid {theme.c.borderDefault};
					color: {theme.c.textPrimary};
					font-size: 14px; line-height: 1.6;
					word-break: break-word;
				">
					{@html renderedContent}
				</div>
				<span style="font-size: 10px; color: {theme.c.textMuted}; margin-top: 2px; display: inline-block;">
					{formatTime(message.created_at)}
				</span>
			</div>
		</div>
	{:else if isSystem}
		<div style="
			display: inline-flex; align-items: center; gap: 6px;
			padding: 4px 14px; border-radius: 12px;
			background: {theme.c.bgSurface};
			border: 1px solid {theme.c.borderDefault};
			font-size: 11px; color: {theme.c.textMuted}; font-style: italic;
		">
			{message.content}
		</div>
	{/if}
</div>

<style>
	.agent-msg-content :global(p) {
		margin: 0 0 0.5em;
	}
	.agent-msg-content :global(p:last-child) {
		margin-bottom: 0;
	}
	.agent-msg-content :global(pre) {
		background: var(--bg-surface);
		border-radius: 6px;
		padding: 8px 10px;
		overflow-x: auto;
		margin: 6px 0;
		font-size: 12px;
	}
	.agent-msg-content :global(code) {
		font-family: var(--font-mono);
		font-size: 12px;
	}
	.agent-msg-content :global(code:not(pre code)) {
		background: var(--bg-surface);
		padding: 1px 5px;
		border-radius: 3px;
	}
	.agent-msg-content :global(a) {
		color: var(--accent-blue);
		text-decoration: none;
	}
	.agent-msg-content :global(a:hover) {
		text-decoration: underline;
	}
	.agent-msg-content :global(ul), .agent-msg-content :global(ol) {
		padding-left: 1.2em;
		margin: 4px 0;
	}
</style>
