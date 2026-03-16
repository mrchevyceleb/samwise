<script lang="ts">
	import { tick } from 'svelte';
	import { getClaudeCodeStore } from '$lib/stores/claude-code.svelte';
	import { getSettingsStore } from '$lib/stores/settings.svelte';
	import CCMessage from './ClaudeCodeMessage.svelte';

	interface Props {
		sessionId: string;
	}

	let { sessionId }: Props = $props();

	const store = getClaudeCodeStore();
	const settingsStore = getSettingsStore();

	let messagesContainer = $state<HTMLDivElement | undefined>(undefined);
	let shouldAutoScroll = $state(true);
	const AUTO_SCROLL_THRESHOLD_PX = 96;

	let session = $derived(store.getSession(sessionId));

	let visibleMessages = $derived((() => {
		if (!session) return [];
		let seenInit = false;
		return session.messages.filter((m) => {
			if (m.type === 'user' || m.type === 'assistant' || m.type === 'result' || m.type === 'error' || m.type === 'stderr') return true;
			if (m.type === 'system' && m.raw?.subtype === 'init' && !seenInit) {
				seenInit = true;
				return true;
			}
			return false;
		});
	})());

	let isRunning = $derived(session?.status === 'running');

	function handleScroll() {
		if (!messagesContainer) return;
		const { scrollTop, scrollHeight, clientHeight } = messagesContainer;
		shouldAutoScroll = scrollHeight - scrollTop - clientHeight < AUTO_SCROLL_THRESHOLD_PX;
	}

	async function scrollToBottom() {
		await tick();
		if (messagesContainer && shouldAutoScroll) {
			messagesContainer.scrollTop = messagesContainer.scrollHeight;
		}
	}

	$effect(() => {
		if (visibleMessages.length) {
			const last = visibleMessages[visibleMessages.length - 1];
			void last.raw;
			scrollToBottom();
		}
	});
</script>

<div
	bind:this={messagesContainer}
	onscroll={handleScroll}
	style="height: 100%; overflow-y: auto; padding: 16px; user-select: text;"
>
	{#if !session}
		<div style="display: flex; align-items: center; justify-content: center; height: 100%; color: var(--text-muted); font-size: 13px;">
			Session not found.
		</div>
	{:else if visibleMessages.length === 0}
		<div style="display: flex; flex-direction: column; align-items: center; justify-content: center; height: 100%; color: var(--text-muted); gap: 8px;">
			{#if session.status === 'error'}
				<span style="color: rgb(239, 68, 68); font-size: 13px;">{session.error || 'Failed to start Claude Code'}</span>
				<span style="font-size: 11px;">Make sure the claude CLI is installed and on your PATH</span>
			{:else}
				<span style="font-size: 13px;">Send a message to get started.</span>
				<span style="font-size: 11px;">Claude Code runs in your workspace with full tool access.</span>
			{/if}
		</div>
	{:else}
		<div style="display: flex; flex-direction: column; gap: 16px;">
			{#each visibleMessages as msg (msg.seq)}
				<CCMessage
					message={msg}
					fontSize={settingsStore.value.aiChatFontSize}
					fontFamily={settingsStore.value.aiChatFontFamily === 'system' ? 'var(--font-ui)' : settingsStore.value.aiChatFontFamily === 'mono' ? 'var(--font-mono)' : settingsStore.value.aiChatFontFamily}
				/>
			{/each}

			<!-- Thinking indicator -->
			{#if isRunning && visibleMessages.length > 0 && visibleMessages[visibleMessages.length - 1].type !== 'result'}
				<div style="display: flex; align-items: center; gap: 8px; padding: 10px 14px;">
					<div style="display: flex; align-items: center; gap: 4px;">
						<div style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent-primary); opacity: 0.6; animation: cc-bounce 1.4s ease-in-out infinite;"></div>
						<div style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent-primary); opacity: 0.6; animation: cc-bounce 1.4s ease-in-out 0.2s infinite;"></div>
						<div style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent-primary); opacity: 0.6; animation: cc-bounce 1.4s ease-in-out 0.4s infinite;"></div>
					</div>
					<span style="font-size: 12px; color: var(--text-muted);">Thinking...</span>
				</div>
			{/if}
		</div>
	{/if}
</div>

<svelte:head>
	<style>
		@keyframes cc-bounce {
			0%, 80%, 100% { transform: translateY(0); opacity: 0.4; }
			40% { transform: translateY(-8px); opacity: 1; }
		}
	</style>
</svelte:head>
