<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { getChatStore } from '$lib/stores/chat.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';
	import ChatMessage from './ChatMessage.svelte';
	import ChatInput from './ChatInput.svelte';
	import AgentAvatar from './AgentAvatar.svelte';

	const chat = getChatStore();
	const theme = getTheme();

	let messagesContainer = $state<HTMLDivElement | null>(null);
	let minimized = $state(false);

	$effect(() => {
		const _len = chat.sortedMessages.length;
		void _len;
		tick().then(() => {
			if (messagesContainer) {
				messagesContainer.scrollTop = messagesContainer.scrollHeight;
			}
		});
	});

	onMount(() => {
		chat.fetchMessages();
	});

	function handleSend(content: string) {
		chat.sendMessage(content);
	}
</script>

<div style="
	display: flex; flex-direction: column; height: 100%;
	{minimized ? 'max-height: 48px; overflow: hidden;' : ''}
">
	<!-- Header -->
	<div style="
		display: flex; align-items: center; gap: 10px;
		padding: 10px 14px;
		background: {theme.c.bgSurface};
		border-bottom: 1px solid {theme.c.borderDefault};
		flex-shrink: 0;
	">
		<AgentAvatar size={22} busy={chat.sendingMessage} />
		<div style="flex: 1; min-width: 0;">
			<div style="
				font-size: 14px; font-weight: 600; color: {theme.c.textPrimary};
				font-family: var(--font-ui); letter-spacing: 0.3px;
			">
				Chat with Sam
			</div>
			<div style="font-size: 12px; color: {theme.c.textMuted};">
				{#if chat.sendingMessage}
					<span style="color: {theme.c.accentPrimary};">Thinking...</span>
				{:else if chat.waitingForSam}
					<span style="color: {theme.c.accentIndigo};">Waiting for Sam...</span>
				{:else}
					Online
				{/if}
			</div>
		</div>
		<button
			style="
				width: 28px; height: 28px; display: flex; align-items: center; justify-content: center;
				background: none; border: 1px solid {theme.c.borderDefault}; border-radius: 6px;
				color: {theme.c.textMuted}; cursor: pointer;
				transition: all 0.15s ease;
			"
			onclick={() => minimized = !minimized}
			aria-label={minimized ? 'Expand chat' : 'Minimize chat'}
		>
			<svg width="14" height="14" viewBox="0 0 16 16" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
				{#if minimized}
					<path d="M4 6l4 4 4-4"/>
				{:else}
					<path d="M4 10l4-4 4 4"/>
				{/if}
			</svg>
		</button>
	</div>

	{#if !minimized}
		<div
			bind:this={messagesContainer}
			style="
				flex: 1; overflow-y: auto; overflow-x: hidden;
				padding: 12px 14px;
				display: flex; flex-direction: column; gap: 4px;
			"
		>
			{#if chat.loading}
				<div style="display: flex; justify-content: center; padding: 40px 0;">
					<div style="display: flex; gap: 6px; align-items: center;">
						<div class="typing-dot" style="animation-delay: 0ms;"></div>
						<div class="typing-dot" style="animation-delay: 150ms;"></div>
						<div class="typing-dot" style="animation-delay: 300ms;"></div>
					</div>
				</div>
			{:else if chat.sortedMessages.length === 0}
				<div style="
					display: flex; flex-direction: column; align-items: center; justify-content: center;
					flex: 1; gap: 12px; padding: 40px 20px; text-align: center;
					animation: fade-in 0.5s ease;
				">
					<AgentAvatar size={48} />
					<div style="font-size: 16px; font-weight: 600; color: {theme.c.textPrimary};">
						Hey.
					</div>
					<div style="font-size: 13px; color: {theme.c.textMuted}; max-width: 260px; line-height: 1.6;">
						Hey, I'm Sam. Drop me a task or just say what you need. I'll handle the code, the PRs, the whole nine yards.
					</div>
				</div>
			{:else}
				{#each chat.sortedMessages as msg (msg.id)}
					<ChatMessage message={msg} />
				{/each}
			{/if}

			{#if chat.sendingMessage || chat.waitingForSam}
				<div style="
					display: flex; gap: 8px; align-items: flex-start;
					padding: 4px 0;
					animation: slide-in-left 0.3s ease;
				">
					<AgentAvatar size={24} busy={true} />
					<div style="
						padding: 10px 14px; border-radius: 4px 14px 14px 14px;
						background: {theme.c.bgElevated};
						border: 1px solid {theme.c.borderDefault};
						display: flex; gap: 4px; align-items: center;
					">
						<div class="typing-dot" style="animation-delay: 0ms;"></div>
						<div class="typing-dot" style="animation-delay: 150ms;"></div>
						<div class="typing-dot" style="animation-delay: 300ms;"></div>
					</div>
				</div>
			{/if}
		</div>

		<ChatInput
			disabled={chat.sendingMessage || chat.waitingForSam}
			placeholder="Message Sam..."
			onSend={handleSend}
		/>
	{/if}
</div>

<style>
	.typing-dot {
		width: 6px;
		height: 6px;
		border-radius: 50%;
		background: var(--text-muted);
		animation: typing-bounce 1.2s ease-in-out infinite;
	}
</style>
