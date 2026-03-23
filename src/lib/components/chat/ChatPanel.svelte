<script lang="ts">
	import { onMount, tick } from 'svelte';
	import { getChatStore } from '$lib/stores/chat.svelte';
	import { getTaskStore } from '$lib/stores/tasks.svelte';
	import { getTheme } from '$lib/stores/theme.svelte';
	import ChatMessage from './ChatMessage.svelte';
	import ChatInput from './ChatInput.svelte';
	import AgentAvatar from './AgentAvatar.svelte';

	const chat = getChatStore();
	const taskStore = getTaskStore();
	const theme = getTheme();

	let messagesContainer = $state<HTMLDivElement | null>(null);
	let minimized = $state(false);
	let confirmingTaskId = $state<string | null>(null);

	// Track pending confirmation tasks
	let pendingTasks = $derived(
		taskStore.tasks.filter((t: any) => t.status === 'pending_confirmation')
	);

	// Clear confirming state when pending tasks change (task was confirmed/rejected)
	$effect(() => {
		if (confirmingTaskId && !pendingTasks.some((t: any) => t.id === confirmingTaskId)) {
			confirmingTaskId = null;
		}
	});

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

	function confirmTask(task: any) {
		confirmingTaskId = task.id;
		chat.sendMessage('yes');
	}

	function rejectTask(task: any) {
		confirmingTaskId = task.id;
		chat.sendMessage('no');
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

		<!-- Pending confirmation banner -->
		{#if pendingTasks.length > 0}
			<div style="
				padding: 8px 14px; display: flex; flex-direction: column; gap: 6px;
				border-top: 1px solid {theme.c.borderDefault};
				background: {theme.c.bgSurface};
			">
				{#each pendingTasks as task (task.id)}
					<div style="
						display: flex; align-items: center; gap: 8px;
						padding: 8px 10px; border-radius: 8px;
						background: {theme.c.accentAmber}08;
						border: 1px solid {theme.c.accentAmber}30;
					">
						<div style="flex: 1; min-width: 0;">
							<div style="font-size: 11px; color: {theme.c.textMuted};">Confirm project for:</div>
							<div style="font-size: 12px; font-weight: 600; color: {theme.c.textPrimary}; white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
								{task.title}
								{#if task.project}
									<span style="font-weight: 400; color: {theme.c.accentIndigo};"> on {task.project}</span>
								{/if}
							</div>
						</div>
						<button
							onclick={() => confirmTask(task)}
							style="
								padding: 4px 12px; border-radius: 6px; font-size: 11px; font-weight: 600;
								background: {theme.c.accentGreen}; color: #fff; border: none;
								cursor: pointer; font-family: var(--font-ui);
								transition: all 0.15s;
							"
						disabled={confirmingTaskId !== null || chat.sendingMessage}
						>{confirmingTaskId === task.id ? 'Confirming...' : 'Confirm'}</button>
						<button
							onclick={() => rejectTask(task)}
							disabled={confirmingTaskId !== null || chat.sendingMessage}
							style="
								padding: 4px 10px; border-radius: 6px; font-size: 11px; font-weight: 600;
								background: none; color: {theme.c.accentRed}; border: 1px solid {theme.c.accentRed}40;
								cursor: {confirmingTaskId ? 'wait' : 'pointer'}; font-family: var(--font-ui);
								opacity: {confirmingTaskId ? '0.6' : '1'};
								transition: all 0.15s;
							"
						>Wrong</button>
					</div>
				{/each}
			</div>
		{/if}

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
