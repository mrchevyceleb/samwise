<script lang="ts">
	import { getConversations, type ConversationRef } from '$lib/stores/conversations.svelte';
	import { getAgentStore } from '$lib/stores/agents.svelte';
	import { getClaudeCodeStore } from '$lib/stores/claude-code.svelte';
	import { getLayout } from '$lib/stores/layout.svelte';
	import { getWorkspace } from '$lib/stores/workspace.svelte';
	import { formatTimeAgo } from '$lib/utils/relative-time';

	const convos = getConversations();
	const agents = getAgentStore();
	const cc = getClaudeCodeStore();
	const layout = getLayout();
	const workspace = getWorkspace();

	let searchQuery = $state('');
	let searchFocused = $state(false);
	let archivedOpen = $state(false);
	let contextMenu = $state<{ x: number; y: number; ref: ConversationRef } | null>(null);
	let renamingId = $state<string | null>(null);
	let renameValue = $state('');
	let newDropdownOpen = $state(false);

	let filtered = $derived(convos.filtered(searchQuery));

	let collapsed = $derived(layout.sidebarCollapsed);

	function focusConversation(ref: ConversationRef) {
		layout.focusedConversation = { id: ref.id, type: ref.type };
	}

	function isFocused(ref: ConversationRef): boolean {
		return layout.focusedConversation?.id === ref.id;
	}

	function handleNewAgent() {
		const id = agents.addAgent();
		layout.focusedConversation = { id, type: 'agent' };
		newDropdownOpen = false;
	}

	function handleNewClaudeCode() {
		const cwd = workspace.path || '.';
		const id = cc.launchSession(cwd);
		layout.focusedConversation = { id, type: 'claude-code' };
		newDropdownOpen = false;
	}

	function handleContextMenu(e: MouseEvent, ref: ConversationRef) {
		e.preventDefault();
		contextMenu = { x: e.clientX, y: e.clientY, ref };
	}

	function closeContextMenu() { contextMenu = null; }

	function startRename(ref: ConversationRef) {
		renamingId = ref.id;
		renameValue = ref.title;
		closeContextMenu();
	}

	function finishRename() {
		if (renamingId && renameValue.trim()) {
			const ref = [...filtered.active, ...filtered.archived].find(c => c.id === renamingId);
			if (ref) convos.rename(ref, renameValue.trim());
		}
		renamingId = null;
	}

	function autofocusAction(node: HTMLInputElement) {
		requestAnimationFrame(() => { node.focus(); node.select(); });
	}

	// Close context menu / dropdown on outside clicks
	$effect(() => {
		if (!contextMenu && !newDropdownOpen) return;
		function onClick() { contextMenu = null; newDropdownOpen = false; }
		setTimeout(() => document.addEventListener('click', onClick), 0);
		return () => document.removeEventListener('click', onClick);
	});

	function typeIcon(type: string): string {
		return type === 'claude-code' ? 'CC' : 'A';
	}

	function typeColor(type: string): string {
		return type === 'claude-code' ? 'rgba(139, 92, 246, 0.8)' : 'var(--banana-yellow)';
	}
</script>

<svelte:head>
	<style>
		@keyframes sidebar-slide-in {
			from { opacity: 0; transform: translateX(-8px); }
			to { opacity: 1; transform: translateX(0); }
		}
	</style>
</svelte:head>

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div style="
	width: {collapsed ? '44px' : '220px'}; display: flex; flex-direction: column;
	height: 100%; border-right: 1px solid var(--border-default);
	background: var(--bg-surface); flex-shrink: 0;
	transition: width 0.2s ease; overflow: hidden;
">
	<!-- Collapse toggle -->
	<div style="display: flex; align-items: center; height: 36px; padding: 0 {collapsed ? '10px' : '10px'}; border-bottom: 1px solid var(--border-default); flex-shrink: 0;">
		<button
			style="
				border: none; background: none; cursor: pointer; padding: 4px;
				color: var(--text-muted); border-radius: 4px; transition: all 0.12s ease;
				display: flex; align-items: center; justify-content: center;
			"
			onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color = 'var(--text-primary)'; (e.currentTarget as HTMLElement).style.background = 'var(--bg-elevated)'; }}
			onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'; (e.currentTarget as HTMLElement).style.background = 'none'; }}
			onclick={() => layout.toggleSidebar()}
			title={collapsed ? 'Expand sidebar' : 'Collapse sidebar'}
		>
			<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="transition: transform 0.2s; transform: {collapsed ? 'rotate(180deg)' : 'rotate(0)'};">
				<polyline points="15 18 9 12 15 6"/>
			</svg>
		</button>
		{#if !collapsed}
			<span style="flex: 1; font-size: 11px; font-weight: 600; color: var(--text-muted); text-align: right; padding-right: 2px;">Chats</span>
		{/if}
	</div>

	{#if !collapsed}
		<!-- Search -->
		<div style="padding: 8px 10px 4px; flex-shrink: 0;">
			<div style="
				display: flex; align-items: center; gap: 6px; padding: 6px 8px;
				background: var(--bg-primary); border: 1px solid {searchFocused ? 'var(--banana-yellow)' : 'var(--border-default)'};
				border-radius: 6px; transition: border-color 0.15s ease;
			">
				<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--text-muted)" stroke-width="2" style="flex-shrink: 0;">
					<circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/>
				</svg>
				<input
					type="text"
					placeholder="Search..."
					bind:value={searchQuery}
					onfocus={() => searchFocused = true}
					onblur={() => searchFocused = false}
					style="flex: 1; border: none; background: none; outline: none; color: var(--text-primary); font-size: 11px; font-family: var(--font-ui);"
				/>
			</div>
		</div>

		<!-- New conversation buttons -->
		<div style="padding: 4px 10px 8px; display: flex; gap: 4px; flex-shrink: 0;">
			<button
				style="
					flex: 1; padding: 7px 0; border: 1px solid var(--border-default); border-radius: 8px;
					cursor: pointer; font-size: 11px; font-family: var(--font-ui); font-weight: 600;
					transition: all 0.15s ease; background: var(--bg-primary); color: var(--text-secondary);
				"
				onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--banana-yellow)'; t.style.color = 'var(--banana-yellow)'; t.style.background = 'rgba(255,214,10,0.08)'; }}
				onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--border-default)'; t.style.color = 'var(--text-secondary)'; t.style.background = 'var(--bg-primary)'; }}
				onclick={handleNewAgent}
			>
				+ Agent
			</button>
			<button
				style="
					flex: 1; padding: 7px 0; border: 1px solid var(--border-default); border-radius: 8px;
					cursor: pointer; font-size: 11px; font-family: var(--font-ui); font-weight: 600;
					transition: all 0.15s ease; background: var(--bg-primary); color: var(--text-secondary);
				"
				onmouseenter={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'rgba(139,92,246,0.6)'; t.style.color = 'rgba(139,92,246,0.9)'; t.style.background = 'rgba(139,92,246,0.08)'; }}
				onmouseleave={(e) => { const t = e.currentTarget as HTMLElement; t.style.borderColor = 'var(--border-default)'; t.style.color = 'var(--text-secondary)'; t.style.background = 'var(--bg-primary)'; }}
				onclick={handleNewClaudeCode}
			>
				+ Claude Code
			</button>
		</div>

		<!-- Conversation list -->
		<div style="flex: 1; overflow-y: auto; padding: 0 6px;">
			{#each filtered.active as ref, i (ref.id)}
				<!-- svelte-ignore a11y_no_static_element_interactions -->
				<div
					style="
						padding: 8px 10px; margin: 2px 0; border-radius: 6px; cursor: pointer;
						transition: all 0.12s ease;
						background: {isFocused(ref) ? 'rgba(255, 214, 10, 0.1)' : 'transparent'};
						border-left: 2px solid {isFocused(ref) ? 'var(--banana-yellow)' : 'transparent'};
						animation: sidebar-slide-in 0.2s ease-out both;
						animation-delay: {Math.min(i * 0.03, 0.15)}s;
					"
					onmouseenter={(e) => { if (!isFocused(ref)) (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.04)'; }}
					onmouseleave={(e) => { if (!isFocused(ref)) (e.currentTarget as HTMLElement).style.background = 'transparent'; }}
					onclick={() => focusConversation(ref)}
					oncontextmenu={(e) => handleContextMenu(e, ref)}
					role="button"
					tabindex={0}
					onkeydown={(e) => { if (e.key === 'Enter') focusConversation(ref); }}
				>
					<div style="display: flex; align-items: center; gap: 6px;">
						<!-- Type badge -->
						<span style="
							font-size: 9px; font-weight: 700; padding: 1px 4px; border-radius: 3px;
							background: {ref.type === 'claude-code' ? 'rgba(139,92,246,0.15)' : 'rgba(255,214,10,0.12)'};
							color: {typeColor(ref.type)}; font-family: var(--font-mono); flex-shrink: 0;
						">
							{typeIcon(ref.type)}
						</span>
						<div style="flex: 1; min-width: 0;">
							{#if renamingId === ref.id}
								<input
									type="text"
									bind:value={renameValue}
									onblur={finishRename}
									onkeydown={(e) => { if (e.key === 'Enter') finishRename(); if (e.key === 'Escape') { renamingId = null; } }}
									use:autofocusAction
									style="
										width: 100%; border: 1px solid var(--banana-yellow); background: var(--bg-primary);
										border-radius: 4px; padding: 2px 4px; color: var(--text-primary);
										font-size: 12px; font-family: var(--font-ui); font-weight: 600; outline: none;
									"
								/>
							{:else}
								<div style="font-size: 12px; font-weight: 600; color: var(--text-primary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">
									{ref.title}
								</div>
							{/if}
						</div>
						<span style="font-size: 10px; color: var(--text-muted); flex-shrink: 0;">
							{formatTimeAgo(ref.lastMessageAt)}
						</span>
					</div>
					{#if ref.lastActivity}
						<div style="font-size: 11px; color: var(--text-muted); margin-top: 2px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding-left: 24px;">
							{ref.lastActivity}
						</div>
					{/if}
				</div>
			{/each}

			{#if filtered.active.length === 0 && !searchQuery}
				<div style="padding: 20px 10px; text-align: center; color: var(--text-muted); font-size: 12px;">
					No conversations yet.
				</div>
			{/if}

			{#if filtered.active.length === 0 && searchQuery}
				<div style="padding: 20px 10px; text-align: center; color: var(--text-muted); font-size: 12px;">
					No matches.
				</div>
			{/if}

			<!-- Archived -->
			{#if filtered.archived.length > 0}
				<button
					style="
						display: flex; align-items: center; gap: 4px; width: 100%;
						padding: 8px 4px; margin-top: 8px; border: none; background: none;
						cursor: pointer; font-size: 11px; font-weight: 600; color: var(--text-muted);
						font-family: var(--font-ui); transition: color 0.12s ease;
					"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.color = 'var(--text-secondary)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.color = 'var(--text-muted)'; }}
					onclick={() => archivedOpen = !archivedOpen}
				>
					<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="transition: transform 0.15s; transform: {archivedOpen ? 'rotate(90deg)' : 'rotate(0)'};">
						<polyline points="9 18 15 12 9 6"/>
					</svg>
					Archived ({filtered.archived.length})
				</button>

				{#if archivedOpen}
					{#each filtered.archived as ref (ref.id)}
						<!-- svelte-ignore a11y_no_static_element_interactions -->
						<div
							style="padding: 6px 10px; margin: 1px 0; border-radius: 6px; cursor: pointer; opacity: 0.6; transition: all 0.12s ease;"
							onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.04)'; (e.currentTarget as HTMLElement).style.opacity = '0.8'; }}
							onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'transparent'; (e.currentTarget as HTMLElement).style.opacity = '0.6'; }}
							onclick={() => { convos.unarchive(ref); focusConversation({ ...ref, archived: false }); }}
							oncontextmenu={(e) => handleContextMenu(e, ref)}
							role="button"
							tabindex={0}
							onkeydown={(e) => { if (e.key === 'Enter') { convos.unarchive(ref); focusConversation({ ...ref, archived: false }); } }}
						>
							<div style="display: flex; align-items: center; gap: 6px;">
								<span style="font-size: 9px; font-weight: 700; padding: 1px 4px; border-radius: 3px; background: rgba(255,255,255,0.06); color: var(--text-muted); font-family: var(--font-mono); flex-shrink: 0;">
									{typeIcon(ref.type)}
								</span>
								<div style="font-size: 11px; color: var(--text-muted); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; flex: 1;">
									{ref.title}
								</div>
								<span style="font-size: 10px; color: var(--text-muted); flex-shrink: 0;">
									{formatTimeAgo(ref.lastMessageAt)}
								</span>
							</div>
						</div>
					{/each}
				{/if}
			{/if}
		</div>
	{:else}
		<!-- Collapsed: icon-only rail -->
		<div style="flex: 1; overflow-y: auto; padding: 4px 0;">
			{#each filtered.active as ref (ref.id)}
				<button
					style="
						width: 100%; padding: 8px 0; border: none; cursor: pointer;
						display: flex; align-items: center; justify-content: center;
						background: {isFocused(ref) ? 'rgba(255, 214, 10, 0.1)' : 'transparent'};
						border-left: 2px solid {isFocused(ref) ? 'var(--banana-yellow)' : 'transparent'};
						transition: all 0.12s ease;
					"
					onmouseenter={(e) => { if (!isFocused(ref)) (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.04)'; }}
					onmouseleave={(e) => { if (!isFocused(ref)) (e.currentTarget as HTMLElement).style.background = isFocused(ref) ? 'rgba(255, 214, 10, 0.1)' : 'transparent'; }}
					onclick={() => focusConversation(ref)}
					title={ref.title}
				>
					<span style="
						font-size: 9px; font-weight: 700; padding: 2px 5px; border-radius: 4px;
						background: {ref.type === 'claude-code' ? 'rgba(139,92,246,0.15)' : 'rgba(255,214,10,0.12)'};
						color: {typeColor(ref.type)}; font-family: var(--font-mono);
					">
						{typeIcon(ref.type)}
					</span>
				</button>
			{/each}
		</div>

		<!-- Collapsed: new buttons -->
		<div style="padding: 4px 0 8px; display: flex; flex-direction: column; align-items: center; gap: 4px; flex-shrink: 0;">
			<button
				style="border: none; background: none; cursor: pointer; color: var(--banana-yellow); padding: 4px; border-radius: 4px; transition: background 0.12s;"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,214,10,0.1)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
				onclick={handleNewAgent}
				title="New Agent"
			>
				<svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<line x1="12" y1="5" x2="12" y2="19"/><line x1="5" y1="12" x2="19" y2="12"/>
				</svg>
			</button>
		</div>
	{/if}

	<!-- Context menu -->
	{#if contextMenu}
		<div style="
			position: fixed; left: {contextMenu.x}px; top: {contextMenu.y}px; z-index: 100;
			background: var(--bg-elevated); border: 1px solid var(--border-default);
			border-radius: 8px; box-shadow: 0 8px 24px rgba(0,0,0,0.4);
			padding: 4px; min-width: 140px;
		">
			{#if !contextMenu.ref.archived}
				<button
					style="width: 100%; padding: 6px 10px; border: none; background: none; cursor: pointer; text-align: left; font-size: 12px; font-family: var(--font-ui); color: var(--text-secondary); border-radius: 4px; transition: background 0.1s;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.06)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
					onclick={() => startRename(contextMenu!.ref)}
				>
					Rename
				</button>
				<button
					style="width: 100%; padding: 6px 10px; border: none; background: none; cursor: pointer; text-align: left; font-size: 12px; font-family: var(--font-ui); color: var(--text-secondary); border-radius: 4px; transition: background 0.1s;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.06)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
					onclick={() => { const ref = contextMenu!.ref; if (layout.focusedConversation?.id === ref.id) layout.focusedConversation = null; convos.archive(ref); closeContextMenu(); }}
				>
					Archive
				</button>
			{:else}
				<button
					style="width: 100%; padding: 6px 10px; border: none; background: none; cursor: pointer; text-align: left; font-size: 12px; font-family: var(--font-ui); color: var(--text-secondary); border-radius: 4px; transition: background 0.1s;"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.06)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
					onclick={() => { convos.unarchive(contextMenu!.ref); closeContextMenu(); }}
				>
					Unarchive
				</button>
			{/if}
			<button
				style="width: 100%; padding: 6px 10px; border: none; background: none; cursor: pointer; text-align: left; font-size: 12px; font-family: var(--font-ui); color: rgb(239, 68, 68); border-radius: 4px; transition: background 0.1s;"
				onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(239, 68, 68, 0.1)'; }}
				onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
				onclick={() => { const ref = contextMenu!.ref; if (layout.focusedConversation?.id === ref.id) layout.focusedConversation = null; convos.remove(ref); closeContextMenu(); }}
			>
				Delete
			</button>
		</div>
	{/if}
</div>
