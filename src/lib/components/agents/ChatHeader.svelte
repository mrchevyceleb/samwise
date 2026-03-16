<script lang="ts">
	import { getAgentStore } from '$lib/stores/agents.svelte';
	import { getClaudeCodeStore } from '$lib/stores/claude-code.svelte';
	import { getConversations } from '$lib/stores/conversations.svelte';
	import { getSettingsStore } from '$lib/stores/settings.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';
	import AgentStatusBadge from './AgentStatusBadge.svelte';

	const agents = getAgentStore();
	const cc = getClaudeCodeStore();
	const convos = getConversations();
	const settingsStore = getSettingsStore();
	const layout = getLayout();
	const workspace = getWorkspace();

	let menuOpen = $state(false);
	let newChatHovered = $state(false);
	let modelDropdownOpen = $state(false);

	const MODEL_OPTIONS = [
		{ id: '', label: 'Default', desc: 'Use CLI default' },
		{ id: 'opus', label: 'Opus', desc: 'Most capable' },
		{ id: 'sonnet', label: 'Sonnet', desc: 'Fast + capable' },
		{ id: 'haiku', label: 'Haiku', desc: 'Fastest' },
	];

	let focused = $derived(layout.focusedConversation);
	let agent = $derived(focused?.type === 'agent' ? agents.getAgent(focused.id) : undefined);
	let ccSession = $derived(focused?.type === 'claude-code' ? cc.getSession(focused.id) : undefined);

	let title = $derived(agent?.title || ccSession?.title || 'New Chat');
	let isAgent = $derived(focused?.type === 'agent');
	let isCC = $derived(focused?.type === 'claude-code');
	let isRunning = $derived(
		(agent && agent.status !== 'idle' && agent.status !== 'done' && agent.status !== 'error') ||
		(ccSession?.status === 'running')
	);

	// CC context usage
	let contextUsage = $derived(ccSession?.contextUsage);
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

	let modelDisplay = $derived((() => {
		if (isAgent && agent) return agent.model.split('/').pop() || agent.model;
		if (isCC && ccSession) {
			const sel = ccSession.selectedModel;
			const det = ccSession.model?.replace(/\[.*\]/, '');
			return sel || det || '';
		}
		return '';
	})());

	function handleNewAgent() {
		const id = agents.addAgent();
		layout.focusedConversation = { id, type: 'agent' };
	}

	function handleStop() {
		if (isCC && ccSession) cc.stopSession(ccSession.id);
		if (isAgent && agent) agents.abortAgent(agent.id);
	}

	function selectCCModel(modelId: string) {
		if (ccSession) cc.setModel(ccSession.id, modelId);
		modelDropdownOpen = false;
	}

	// Close menus on outside click
	$effect(() => {
		if (!menuOpen && !modelDropdownOpen) return;
		function onClick() { menuOpen = false; modelDropdownOpen = false; }
		setTimeout(() => document.addEventListener('click', onClick), 0);
		return () => document.removeEventListener('click', onClick);
	});
</script>

<div style="
	display: flex; align-items: center; gap: 8px;
	height: 40px; padding: 0 12px;
	box-shadow: 0 2px 8px rgba(0, 0, 0, 0.2), inset 0 -1px 0 rgba(255, 255, 255, 0.03);
	flex-shrink: 0; background: linear-gradient(180deg, rgba(25, 31, 40, 0.6) 0%, rgba(18, 23, 31, 0.4) 100%); position: relative; z-index: 2;
">
	{#if focused}
		<!-- Type badge -->
		<span style="
			font-size: 9px; font-weight: 700; padding: 2px 5px; border-radius: 4px;
			background: {isCC ? 'rgba(139,92,246,0.15)' : 'color-mix(in srgb, var(--accent-primary) 12%, transparent)'};
			color: {isCC ? 'rgba(139,92,246,0.8)' : 'var(--accent-primary)'};
			font-family: var(--font-mono); flex-shrink: 0;
		">
			{isCC ? 'CC' : '🐒'}
		</span>

		<!-- Title -->
		<span style="font-size: 13px; font-weight: 600; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 180px;">
			{title}
		</span>

		<!-- Status -->
		{#if isRunning}
			{#if isAgent && agent}
				<AgentStatusBadge status={agent.status} size="md" />
			{:else}
				<div style="width: 6px; height: 6px; border-radius: 50%; background: rgba(139,92,246,0.8); animation: cc-header-pulse 1.5s ease-in-out infinite;"></div>
			{/if}
		{/if}
	{:else}
		<span style="font-size: 13px; font-weight: 700; color: var(--text-primary); letter-spacing: -0.2px;">New Chat</span>
	{/if}

	<div style="flex: 1;"></div>

	<!-- CC context usage bar -->
	{#if isCC && contextUsage && contextUsage.inputTokens > 0}
		<div style="display: flex; align-items: center; gap: 4px;" title="{contextPercent}% context used">
			<div style="width: 60px; height: 5px; border-radius: 3px; overflow: hidden; background: var(--bg-primary);">
				<div style="height: 100%; border-radius: 3px; transition: width 0.3s ease; width: {contextPercent}%; background: {contextColor};"></div>
			</div>
			<span style="font-size: 9px; color: var(--text-muted); font-family: var(--font-mono);">{contextPercent}%</span>
		</div>
	{/if}

	<!-- CC model switcher -->
	{#if isCC}
		<div style="position: relative;" class="cc-dropdown">
			<button
				style="
					display: flex; align-items: center; gap: 3px; padding: 3px 6px;
					border: none; background: none; cursor: pointer; border-radius: 4px;
					color: var(--text-muted); font-size: 10px; font-family: var(--font-mono);
					transition: background 0.1s;
				"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-elevated)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
				onclick={() => modelDropdownOpen = !modelDropdownOpen}
			>
				{modelDisplay || 'Model'}
				<svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="opacity: 0.5;">
					<polyline points="6 9 12 15 18 9"/>
				</svg>
			</button>

			{#if modelDropdownOpen}
				<div style="
					position: absolute; right: 0; top: 100%; margin-top: 4px; width: 180px; z-index: 60;
					background: var(--bg-elevated); border: 1px solid var(--border-default);
					border-radius: 8px; box-shadow: 0 8px 24px rgba(0,0,0,0.4); overflow: hidden;
				">
					{#each MODEL_OPTIONS as opt}
						<button
							style="
								width: 100%; display: flex; align-items: center; justify-content: space-between;
								padding: 6px 10px; border: none; cursor: pointer; text-align: left;
								font-family: var(--font-ui); font-size: 11px;
								background: {(ccSession?.selectedModel || '') === opt.id ? 'rgba(139,92,246,0.1)' : 'transparent'};
								color: {(ccSession?.selectedModel || '') === opt.id ? 'rgba(139,92,246,0.9)' : 'var(--text-secondary)'};
								transition: background 0.1s;
							"
							onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.05)'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = (ccSession?.selectedModel || '') === opt.id ? 'rgba(139,92,246,0.1)' : 'transparent'; }}
							onclick={() => selectCCModel(opt.id)}
						>
							<div>
								<div style="font-weight: 500;">{opt.label}</div>
								<div style="font-size: 9px; color: var(--text-muted);">{opt.desc}</div>
							</div>
						</button>
					{/each}
				</div>
			{/if}
		</div>
	{/if}

	<!-- Agent model pill -->
	{#if isAgent && modelDisplay}
		<span style="
			padding: 2px 8px; border-radius: 10px; font-size: 10px;
			background: color-mix(in srgb, var(--accent-primary) 8%, transparent); color: var(--accent-dim);
			font-family: var(--font-mono); border: 1px solid color-mix(in srgb, var(--accent-primary) 10%, transparent);
		">
			{modelDisplay}
		</span>
	{/if}

	<!-- Stop button -->
	{#if isRunning}
		<button
			style="
				padding: 3px 8px; border: 1px solid rgba(239, 68, 68, 0.3); background: rgba(239, 68, 68, 0.1);
				border-radius: 6px; cursor: pointer; font-size: 10px; font-family: var(--font-ui);
				color: rgb(239, 68, 68); font-weight: 600; transition: all 0.12s ease;
			"
			onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(239, 68, 68, 0.2)'; }}
			onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(239, 68, 68, 0.1)'; }}
			onclick={handleStop}
			title="Stop"
		>
			Stop
		</button>
	{/if}

	<!-- + button -->
	<button
		style="
			width: 24px; height: 24px; display: flex; align-items: center; justify-content: center;
			border: 1px solid {newChatHovered ? 'var(--accent-primary)' : 'var(--border-default)'};
			border-radius: 5px; cursor: pointer; transition: all 0.12s ease;
			background: {newChatHovered ? 'color-mix(in srgb, var(--accent-primary) 12%, transparent)' : 'transparent'};
			color: {newChatHovered ? 'var(--accent-primary)' : 'var(--text-muted)'};
		"
		onmouseenter={() => newChatHovered = true}
		onmouseleave={() => newChatHovered = false}
		onclick={handleNewAgent}
		title="New chat"
	>
		<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
			<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
		</svg>
	</button>

	<!-- Collapse panel -->
	<button
		style="
			width: 24px; height: 24px; display: flex; align-items: center; justify-content: center;
			border: none; border-radius: 5px; cursor: pointer; transition: all 0.12s ease;
			background: none; color: var(--text-muted);
		"
		onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-elevated)'; (e.currentTarget as HTMLElement).style.color = 'var(--text-primary)'; }}
		onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'; }}
		onclick={() => layout.toggleLeftPanel()}
		title="Collapse panel (Ctrl+B)"
	>
		<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
			<polyline points="11 17 6 12 11 7"/><line x1="6" y1="12" x2="20" y2="12"/>
		</svg>
	</button>

	<!-- ... menu -->
	{#if focused}
		<div style="position: relative;">
			<button
				style="
					width: 24px; height: 24px; display: flex; align-items: center; justify-content: center;
					border: none; border-radius: 5px; cursor: pointer; transition: all 0.12s ease;
					background: none; color: var(--text-muted);
				"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'var(--bg-elevated)'; (e.currentTarget as HTMLElement).style.color = 'var(--text-primary)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'; }}
				onclick={() => menuOpen = !menuOpen}
				title="More"
			>
				<svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
					<circle cx="12" cy="5" r="1.5"/><circle cx="12" cy="12" r="1.5"/><circle cx="12" cy="19" r="1.5"/>
				</svg>
			</button>

			{#if menuOpen}
				<div style="
					position: absolute; right: 0; top: 100%; margin-top: 4px; z-index: 60;
					background: var(--bg-elevated); border: 1px solid var(--border-default);
					border-radius: 8px; box-shadow: 0 8px 24px rgba(0,0,0,0.4);
					padding: 4px; min-width: 140px;
				">
					<button
						style="width: 100%; padding: 6px 10px; border: none; background: none; cursor: pointer; text-align: left; font-size: 12px; font-family: var(--font-ui); color: var(--text-secondary); border-radius: 4px; transition: background 0.1s;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.06)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
						onclick={() => {
							if (isAgent && agent) agents.clearMessages(agent.id);
							if (isCC && ccSession) cc.clearMessages(ccSession.id);
							menuOpen = false;
						}}
					>
						Clear messages
					</button>
					<button
						style="width: 100%; padding: 6px 10px; border: none; background: none; cursor: pointer; text-align: left; font-size: 12px; font-family: var(--font-ui); color: var(--text-secondary); border-radius: 4px; transition: background 0.1s;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.06)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
						onclick={() => {
							if (focused) { convos.archive(focused); layout.focusedConversation = null; }
							menuOpen = false;
						}}
					>
						Archive
					</button>
					<button
						style="width: 100%; padding: 6px 10px; border: none; background: none; cursor: pointer; text-align: left; font-size: 12px; font-family: var(--font-ui); color: rgb(239, 68, 68); border-radius: 4px; transition: background 0.1s;"
						onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(239, 68, 68, 0.1)'; }}
						onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
						onclick={() => {
							if (focused) convos.remove(focused);
							layout.focusedConversation = null;
							menuOpen = false;
						}}
					>
						Delete
					</button>
				</div>
			{/if}
		</div>
	{/if}
</div>

<svelte:head>
	<style>
		@keyframes cc-header-pulse {
			0%, 100% { opacity: 0.4; }
			50% { opacity: 1; }
		}
	</style>
</svelte:head>
