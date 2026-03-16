<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { getChatStore } from '$lib/stores/chat.svelte';
	import ChatMessage from './ChatMessage.svelte';
	import ChatInput from './ChatInput.svelte';
	import AgentAvatar from './AgentAvatar.svelte';

	const chat = getChatStore();

	let messagesContainer = $state<HTMLDivElement | null>(null);
	let minimized = $state(false);

	// Auto-scroll to bottom when messages change
	$effect(() => {
		// Touch messages array to create dependency
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
		background: rgba(22, 27, 34, 0.8);
		border-bottom: 1px solid var(--border-default);
		flex-shrink: 0;
	">
		<AgentAvatar size={22} busy={chat.sendingMessage} />
		<div style="flex: 1; min-width: 0;">
			<div style="
				font-size: 13px; font-weight: 600; color: var(--text-primary);
				font-family: var(--font-ui); letter-spacing: 0.3px;
			">
				Chat with Agent
			</div>
			<div style="font-size: 10px; color: var(--text-muted);">
				{#if chat.sendingMessage}
					<span style="color: var(--accent-primary);">Thinking...</span>
				{:else}
					Online
				{/if}
			</div>
		</div>
		<!-- Minimize button -->
		<button
			style="
				width: 28px; height: 28px; display: flex; align-items: center; justify-content: center;
				background: none; border: 1px solid var(--border-default); border-radius: 6px;
				color: var(--text-muted); cursor: pointer;
				transition: all 0.15s ease;
			"
			onmouseenter={(e) => {
				const el = e.currentTarget as HTMLElement;
				el.style.background = 'rgba(99, 102, 241, 0.1)';
				el.style.borderColor = 'rgba(99, 102, 241, 0.3)';
				el.style.color = 'var(--accent-primary)';
			}}
			onmouseleave={(e) => {
				const el = e.currentTarget as HTMLElement;
				el.style.background = 'none';
				el.style.borderColor = 'var(--border-default)';
				el.style.color = 'var(--text-muted)';
			}}
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
		<!-- Message list -->
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
				<!-- Empty state -->
				<div style="
					display: flex; flex-direction: column; align-items: center; justify-content: center;
					flex: 1; gap: 12px; padding: 40px 20px; text-align: center;
					animation: fade-in 0.5s ease;
				">
					<AgentAvatar size={48} />
					<div style="font-size: 15px; font-weight: 600; color: var(--text-primary);">
						Hey there.
					</div>
					<div style="font-size: 12px; color: var(--text-muted); max-width: 260px; line-height: 1.6;">
						I'm your AI agent. Send me a message to get started. I can create tasks, run automations, or just chat.
					</div>
				</div>
			{:else}
				{#each chat.sortedMessages as msg (msg.id)}
					<ChatMessage message={msg} />
				{/each}
			{/if}

			<!-- Typing indicator -->
			{#if chat.sendingMessage}
				<div style="
					display: flex; gap: 8px; align-items: flex-start;
					padding: 4px 0;
					animation: slide-in-left 0.3s ease;
				">
					<AgentAvatar size={24} busy={true} />
					<div style="
						padding: 10px 14px; border-radius: 4px 14px 14px 14px;
						background: #1c2128;
						border: 1px solid var(--border-default);
						display: flex; gap: 4px; align-items: center;
					">
						<div class="typing-dot" style="animation-delay: 0ms;"></div>
						<div class="typing-dot" style="animation-delay: 150ms;"></div>
						<div class="typing-dot" style="animation-delay: 300ms;"></div>
					</div>
				</div>
			{/if}
		</div>

		<!-- Input area -->
		<ChatInput
			disabled={chat.sendingMessage}
			placeholder="Message your agent..."
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
