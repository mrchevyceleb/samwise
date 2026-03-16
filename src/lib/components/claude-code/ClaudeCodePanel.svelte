<script lang="ts">
	import { tick } from 'svelte';
	import { getClaudeCodeStore } from '$lib/stores/claude-code.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';
	import { getSettingsStore } from '$lib/stores/settings.svelte';
	import CCMessage from './ClaudeCodeMessage.svelte';
	import ClaudeCodeInput from './ClaudeCodeInput.svelte';

	const store = getClaudeCodeStore();
	const workspace = getWorkspace();
	const settingsStore = getSettingsStore();

	let messagesContainer = $state<HTMLDivElement | undefined>(undefined);
	let shouldAutoScroll = $state(true);
	const AUTO_SCROLL_THRESHOLD_PX = 96;

	// Model switcher
	let modelDropdownOpen = $state(false);
	const MODEL_OPTIONS = [
		{ id: '', label: 'Default', desc: 'Use CLI default' },
		{ id: 'opus', label: 'Opus', desc: 'Most capable' },
		{ id: 'sonnet', label: 'Sonnet', desc: 'Fast + capable' },
		{ id: 'haiku', label: 'Haiku', desc: 'Fastest' },
	];

	// Prevent auto-relaunch after user deliberately closes
	let userClosed = $state(false);

	let session = $derived(store.getActiveSession());

	// Auto-launch session when panel mounts with a workspace
	$effect(() => {
		if (!session && workspace.path && !userClosed) {
			store.launchSession(workspace.path);
		}
	});

	// Reset userClosed when workspace changes
	$effect(() => {
		if (workspace.path) {
			userClosed = false;
		}
	});

	// Close dropdown on outside clicks (document-level)
	$effect(() => {
		if (!modelDropdownOpen) return;
		function onDocClick(e: MouseEvent) {
			const target = e.target as HTMLElement;
			if (!target.closest('.cc-dropdown')) {
				modelDropdownOpen = false;
			}
		}
		// Defer to avoid catching the click that opened it
		setTimeout(() => document.addEventListener('click', onDocClick), 0);
		return () => document.removeEventListener('click', onDocClick);
	});

	// Filter messages for display
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

	let modelDisplay = $derived((() => {
		const selected = session?.selectedModel;
		const detected = session?.model?.replace(/\[.*\]/, '');
		if (selected) return selected;
		return detected || '';
	})());

	// Context usage
	let contextUsage = $derived(session?.contextUsage);
	let contextPercent = $derived((() => {
		if (!contextUsage || !contextUsage.contextWindow) return 0;
		const total = contextUsage.inputTokens + contextUsage.outputTokens + contextUsage.cacheCreationTokens;
		return Math.min(100, Math.round((total / contextUsage.contextWindow) * 100));
	})());
	let contextColor = $derived(
		contextPercent >= 80 ? 'rgb(239, 68, 68)' :
		contextPercent >= 50 ? 'rgb(234, 179, 8)' :
		'rgb(34, 197, 94)'
	);

	let slashCommands = $derived(session?.slashCommands ?? []);

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

	// Track visibleMessages content changes for auto-scroll (not just message count)
	$effect(() => {
		if (visibleMessages.length) {
			// Access the last message's raw to track streaming updates
			const last = visibleMessages[visibleMessages.length - 1];
			void last.raw;
			scrollToBottom();
		}
	});

	function handleSend(message: string) {
		if (!session) {
			// Launch a session if none exists
			if (workspace.path) {
				userClosed = false;
				const id = store.launchSession(workspace.path);
				store.sendMessage(id, message);
			}
			return;
		}
		store.sendMessage(session.id, message);
	}

	function handleStop() {
		if (session) store.stopSession(session.id);
	}

	function handleSteer(steer: string) {
		if (!session) return;
		const sessionId = session.id;
		store.stopSession(sessionId).then(() => {
			setTimeout(() => {
				store.sendMessage(sessionId, steer);
			}, 200);
		});
	}

	function handleClose() {
		if (!session) return;
		const sessionId = session.id;
		userClosed = true;
		if (isRunning) {
			store.stopSession(sessionId).then(() => {
				store.removeSession(sessionId);
			});
		} else {
			store.removeSession(sessionId);
		}
	}

	function selectModel(modelId: string) {
		if (session) store.setModel(session.id, modelId);
		modelDropdownOpen = false;
	}

	function formatTokens(n: number): string {
		if (n >= 1_000_000) return `${(n / 1_000_000).toFixed(1)}M`;
		if (n >= 1_000) return `${(n / 1_000).toFixed(1)}K`;
		return String(n);
	}

	// Click easter egg
	function handlePanelClick(e: MouseEvent) {
		const target = e.target as HTMLElement;
		if (target.closest('button, textarea, input, select, label, a, pre, code, .cc-prose, .cc-dropdown')) return;
		modelDropdownOpen = false;
		const symbols = ['>', '_', '/', '\\', '|', '{', '}', '$', '#', '~'];
		const symbol = symbols[Math.floor(Math.random() * symbols.length)];
		const el = document.createElement('span');
		el.textContent = symbol;
		el.style.cssText = `
			position: fixed;
			left: ${e.clientX}px;
			top: ${e.clientY}px;
			pointer-events: none;
			font-family: monospace;
			font-size: 18px;
			color: color-mix(in srgb, var(--accent-primary) 70%, transparent);
			z-index: 9999;
			animation: cc-float 1s ease-out forwards;
		`;
		document.body.appendChild(el);
		setTimeout(() => el.remove(), 1000);
	}
</script>

<svelte:head>
	<style>
		@keyframes cc-float {
			0% { opacity: 1; transform: translateY(0) scale(1); }
			100% { opacity: 0; transform: translateY(-40px) scale(0.5) rotate(20deg); }
		}
		@keyframes cc-panel-pulse {
			0%, 100% { opacity: 0.4; }
			50% { opacity: 0.8; }
		}
		@keyframes cc-bounce {
			0%, 80%, 100% { transform: translateY(0); opacity: 0.4; }
			40% { transform: translateY(-8px); opacity: 1; }
		}
		@keyframes cc-drift {
			0% { transform: translateY(0) translateX(0); }
			50% { transform: translateY(-15px) translateX(8px); }
			100% { transform: translateY(0) translateX(0); }
		}
	</style>
</svelte:head>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	style="height: 100%; display: flex; flex-direction: column; position: relative; overflow: hidden; background: var(--bg-surface);"
	onclick={handlePanelClick}
>
	<!-- Floating background symbols -->
	<div style="position: absolute; inset: 0; pointer-events: none; overflow: hidden; z-index: 0;">
		{#each Array(6) as _, i}
			<span
				style="
					position: absolute;
					left: {10 + i * 15}%;
					top: {5 + (i * 17) % 80}%;
					font-size: {10 + (i % 3) * 4}px;
					color: color-mix(in srgb, var(--accent-primary) 6%, transparent);
					font-family: monospace;
					user-select: none;
					animation: cc-drift {4 + i * 1.3}s ease-in-out infinite;
					animation-delay: {i * 0.7}s;
				"
			>
				{['>', '$', '~', '_', '|', '#'][i]}
			</span>
		{/each}
	</div>

	<!-- Header -->
	<div
		style="
			display: flex; align-items: center; gap: 8px;
			height: 40px; padding: 0 12px;
			border-bottom: 1px solid var(--border-default);
			flex-shrink: 0; position: relative; z-index: 2;
		"
	>
		<!-- CC icon -->
		<div
			style="
				width: 26px; height: 26px; display: flex; align-items: center; justify-content: center;
				border-radius: 6px; flex-shrink: 0;
				background: linear-gradient(135deg, color-mix(in srgb, var(--accent-primary) 15%, transparent), color-mix(in srgb, var(--accent-primary) 5%, transparent));
				border: 1px solid color-mix(in srgb, var(--accent-primary) 20%, transparent);
			"
		>
			<span style="font-size: 11px; font-weight: 700; color: var(--accent-primary); font-family: var(--font-mono);">CC</span>
		</div>

		<!-- Model switcher -->
		<div style="position: relative;" class="cc-dropdown">
			<button
				style="
					display: flex; align-items: center; gap: 4px; padding: 4px 8px;
					border: none; background: none; cursor: pointer; border-radius: 6px;
					color: var(--text-primary); font-size: 13px; font-weight: 600; font-family: var(--font-ui);
					transition: background 0.1s ease;
				"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-elevated)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
				onclick={() => modelDropdownOpen = !modelDropdownOpen}
			>
				<span style="max-width: 140px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
					{modelDisplay || 'Claude Code'}
				</span>
				<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="opacity: 0.5; flex-shrink: 0;">
					<polyline points="6 9 12 15 18 9"/>
				</svg>
			</button>

			{#if modelDropdownOpen}
				<div
					style="
						position: absolute; left: 0; top: 100%; margin-top: 4px;
						width: 200px; z-index: 60;
						background: var(--bg-elevated); border: 1px solid var(--border-default);
						border-radius: 8px; box-shadow: 0 8px 24px rgba(0,0,0,0.4); overflow: hidden;
					"
				>
					{#each MODEL_OPTIONS as opt}
						<button
							style="
								width: 100%; display: flex; align-items: center; justify-content: space-between;
								padding: 8px 12px; border: none; cursor: pointer; text-align: left;
								font-family: var(--font-ui); font-size: 12px;
								background: {(session?.selectedModel || '') === opt.id ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'transparent'};
								color: {(session?.selectedModel || '') === opt.id ? 'var(--accent-primary)' : 'var(--text-secondary)'};
								transition: background 0.1s ease;
							"
							onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.05)'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = (session?.selectedModel || '') === opt.id ? 'color-mix(in srgb, var(--accent-primary) 10%, transparent)' : 'transparent'; }}
							onclick={() => selectModel(opt.id)}
						>
							<div>
								<div style="font-weight: 500;">{opt.label}</div>
								<div style="font-size: 10px; color: var(--text-muted);">{opt.desc}</div>
							</div>
							{#if (session?.selectedModel || '') === opt.id}
								<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--accent-primary)" stroke-width="3">
									<polyline points="20 6 9 17 4 12"/>
								</svg>
							{/if}
						</button>
					{/each}
				</div>
			{/if}
		</div>

		<div style="flex: 1;"></div>

		<!-- Context usage bar -->
		{#if contextUsage && contextUsage.inputTokens > 0}
			<div style="display: flex; align-items: center; gap: 6px;" title="{formatTokens(contextUsage.inputTokens + contextUsage.outputTokens)} / {formatTokens(contextUsage.contextWindow)} tokens ({contextPercent}%)">
				<div style="width: 80px; height: 6px; border-radius: 3px; overflow: hidden; background: var(--bg-primary);">
					<div style="height: 100%; border-radius: 3px; transition: width 0.3s ease; width: {contextPercent}%; background: {contextColor};"></div>
				</div>
				<span style="font-size: 10px; color: var(--text-muted); font-family: var(--font-mono);">{contextPercent}%</span>
			</div>
		{/if}

		<!-- Status -->
		{#if isRunning}
			<div style="display: flex; align-items: center; gap: 4px;">
				<div style="width: 6px; height: 6px; border-radius: 50%; background: var(--accent-primary); animation: cc-panel-pulse 1.5s ease-in-out infinite;"></div>
				<span style="font-size: 11px; color: var(--accent-primary); font-weight: 500;">Running</span>
			</div>
		{:else if session?.status === 'error'}
			<span style="font-size: 11px; color: rgb(239, 68, 68); font-weight: 500;">Error</span>
		{/if}

		<!-- Slash commands hint -->
		{#if slashCommands.length > 0 && !isRunning}
			<span style="font-size: 10px; color: var(--text-muted); opacity: 0.6; font-family: var(--font-mono);" title={slashCommands.map((c) => `/${c}`).join(', ')}>
				/{slashCommands.length} cmds
			</span>
		{/if}

		<!-- Stop button (header) -->
		{#if isRunning}
			<button
				style="
					padding: 4px 8px; border: 1px solid rgba(239, 68, 68, 0.3); background: rgba(239, 68, 68, 0.1);
					border-radius: 6px; cursor: pointer; font-size: 10px; font-family: var(--font-ui);
					color: rgb(239, 68, 68); font-weight: 600; transition: all 0.12s ease;
				"
				onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.background = 'rgba(239, 68, 68, 0.2)'; }}
				onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.background = 'rgba(239, 68, 68, 0.1)'; }}
				onclick={handleStop}
				title="Stop Claude Code"
			>
				Stop
			</button>
		{/if}

		<!-- Close session button -->
		{#if session}
			<button
				style="
					padding: 4px; border: none; background: none; cursor: pointer;
					color: var(--text-muted); border-radius: 4px;
					transition: all 0.1s ease;
				"
				onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'rgb(239, 68, 68)'; t.style.background = 'rgba(239, 68, 68, 0.1)'; }}
				onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'var(--text-muted)'; t.style.background = 'none'; }}
				onclick={handleClose}
				title="Close session"
			>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
				</svg>
			</button>
		{/if}

		<!-- Settings -->
		<button
			style="
				padding: 4px; border: none; background: none; cursor: pointer;
				color: var(--text-muted); border-radius: 4px;
				transition: all 0.1s ease;
			"
			onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'var(--text-primary)'; t.style.background = 'var(--bg-elevated)'; }}
			onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.color = 'var(--text-muted)'; t.style.background = 'none'; }}
			onclick={() => settingsStore.settingsVisible = true}
			title="Settings"
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
				<circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83-2.83l.06-.06A1.65 1.65 0 0 0 4.68 15a1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 2.83-2.83l.06.06A1.65 1.65 0 0 0 9 4.68a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 2.83l-.06.06A1.65 1.65 0 0 0 19.4 9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z"/>
			</svg>
		</button>
	</div>

	<!-- Messages area -->
	<div
		bind:this={messagesContainer}
		onscroll={handleScroll}
		style="flex: 1; overflow-y: auto; position: relative; z-index: 1; padding: 16px; user-select: text;"
	>
		{#if !session}
			<div style="display: flex; align-items: center; justify-content: center; height: 100%; color: var(--text-muted); font-size: 13px;">
				No workspace open. Open a folder to start.
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
					<CCMessage message={msg} fontSize={settingsStore.value.aiChatFontSize} fontFamily={settingsStore.value.aiChatFontFamily === 'system' ? 'var(--font-ui)' : settingsStore.value.aiChatFontFamily === 'mono' ? 'var(--font-mono)' : settingsStore.value.aiChatFontFamily} />
				{/each}

				<!-- Thinking indicator (show when running and last message is user or an assistant with tool use) -->
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

	<!-- Input area -->
	<div style="flex-shrink: 0; position: relative; z-index: 1;">
		<ClaudeCodeInput
			onSend={handleSend}
			onStop={handleStop}
			onSteer={handleSteer}
			isRunning={isRunning ?? false}
		/>
	</div>
</div>
