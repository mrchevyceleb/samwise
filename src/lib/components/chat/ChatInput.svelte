<script lang="ts">
	import { getTheme } from '$lib/stores/theme.svelte';
	import { getProjectStore } from '$lib/stores/projects.svelte';
	import type { AeProject } from '$lib/types';

	interface Props {
		disabled?: boolean;
		placeholder?: string;
		onSend: (message: string) => void;
		modelName?: string;
	}

	let { disabled = false, placeholder = 'Message Sam...', onSend, modelName = '' }: Props = $props();
	const theme = getTheme();
	const projectStore = getProjectStore();

	let inputValue = $state('');
	let sendHovered = $state(false);
	let textareaEl = $state<HTMLTextAreaElement | null>(null);

	// @ autocomplete state
	let showAutocomplete = $state(false);
	let autocompleteQuery = $state('');
	let autocompleteIndex = $state(0);
	let mentionStart = $state(-1); // cursor position where @ was typed

	let filteredProjects = $derived.by(() => {
		if (!showAutocomplete) return [];
		const q = autocompleteQuery.toLowerCase();
		return projectStore.projects.filter((p: AeProject) => {
			const name = (p.name || '').toLowerCase();
			const client = (p.client || '').toLowerCase();
			return name.includes(q) || client.includes(q);
		}).slice(0, 8);
	});

	function autoResize() {
		if (!textareaEl) return;
		textareaEl.style.height = 'auto';
		textareaEl.style.height = Math.min(textareaEl.scrollHeight, 200) + 'px';
	}

	function handleKeydown(e: KeyboardEvent) {
		// Autocomplete navigation
		if (showAutocomplete && filteredProjects.length > 0) {
			if (e.key === 'ArrowDown') {
				e.preventDefault();
				autocompleteIndex = (autocompleteIndex + 1) % filteredProjects.length;
				return;
			}
			if (e.key === 'ArrowUp') {
				e.preventDefault();
				autocompleteIndex = (autocompleteIndex - 1 + filteredProjects.length) % filteredProjects.length;
				return;
			}
			if (e.key === 'Enter' || e.key === 'Tab') {
				e.preventDefault();
				selectProject(filteredProjects[autocompleteIndex]);
				return;
			}
			if (e.key === 'Escape') {
				e.preventDefault();
				showAutocomplete = false;
				return;
			}
		}

		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			send();
		}
	}

	function handleInput() {
		autoResize();
		checkForMention();
	}

	function checkForMention() {
		if (!textareaEl) return;
		const pos = textareaEl.selectionStart;
		const text = inputValue.slice(0, pos);

		// Find @ preceded by whitespace, comma, or start of string (avoids triggering on emails)
		const match = text.match(/(^|[\s,])@([\w-]*)$/);
		if (match) {
			mentionStart = pos - match[2].length - 1; // -1 for the @ character
			autocompleteQuery = match[2];
			autocompleteIndex = 0;
			showAutocomplete = true;
		} else {
			showAutocomplete = false;
		}
	}

	function selectProject(project: AeProject) {
		if (!textareaEl) return;
		const before = inputValue.slice(0, mentionStart);
		const after = inputValue.slice(textareaEl.selectionStart);
		inputValue = `${before}@${project.name} ${after}`;
		showAutocomplete = false;

		// Restore cursor position after the inserted mention
		const newPos = mentionStart + project.name.length + 2; // +2 for @ and space
		requestAnimationFrame(() => {
			textareaEl?.setSelectionRange(newPos, newPos);
			textareaEl?.focus();
		});
	}

	function send() {
		const trimmed = inputValue.trim();
		if (!trimmed || disabled) return;
		showAutocomplete = false;
		onSend(trimmed);
		inputValue = '';
		if (textareaEl) textareaEl.style.height = 'auto';
	}

	$effect(() => {
		if (inputValue !== undefined) autoResize();
	});
</script>

<div style="padding: 10px 12px; position: relative;">
	<!-- @ Autocomplete dropdown -->
	{#if showAutocomplete && filteredProjects.length > 0}
		<div
			role="listbox"
			id="project-autocomplete"
			aria-label="Project suggestions"
			style="
			position: absolute; bottom: 100%; left: 12px; right: 12px;
			background: {theme.c.bgElevated}; border: 1px solid {theme.c.borderDefault};
			border-radius: 10px; overflow: hidden; z-index: 50;
			box-shadow: 0 -4px 16px rgba(0,0,0,0.3);
			max-height: 240px; overflow-y: auto;
			margin-bottom: 4px;
		">
			{#each filteredProjects as project, i (project.id)}
				<button
					role="option"
					id="project-option-{i}"
					aria-selected={i === autocompleteIndex}
					onmousedown={(e) => { e.preventDefault(); selectProject(project); }}
					onmouseenter={() => autocompleteIndex = i}
					style="
						width: 100%; display: flex; align-items: center; gap: 8px;
						padding: 8px 12px; border: none; text-align: left;
						background: {i === autocompleteIndex ? theme.c.accentIndigo + '18' : 'transparent'};
						{i === autocompleteIndex ? 'outline: 2px solid ' + theme.c.accentIndigo + '40; outline-offset: -2px;' : ''}
						color: {theme.c.textPrimary}; cursor: pointer;
						font-family: var(--font-ui); transition: background 0.1s;
					"
				>
					<span style="font-size: 14px; opacity: 0.5;">@</span>
					<div style="flex: 1; min-width: 0;">
						<div style="display: flex; align-items: center; gap: 6px;">
							<span style="font-size: 12px; font-weight: 600; color: {theme.c.textPrimary};">{project.name}</span>
							{#if project.client}
								<span style="
									font-size: 9px; padding: 1px 5px; border-radius: 3px;
									background: {theme.c.accentIndigo}15; color: {theme.c.accentIndigo};
									font-weight: 600;
								">{project.client}</span>
							{/if}
						</div>
						{#if project.repo_path}
							<div style="font-size: 9px; color: {theme.c.textMuted}; font-family: var(--font-mono); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
								{project.repo_path}
							</div>
						{/if}
					</div>
				</button>
			{/each}
		</div>
	{/if}

	<div
		style="
			display: flex; flex-direction: column; gap: 0;
			background: {theme.c.bgElevated};
			border: 1px solid {theme.c.borderDefault};
			border-radius: 14px; overflow: hidden;
			transition: border-color 0.2s ease, box-shadow 0.2s ease;
			box-shadow: {theme.c.shadowSm};
			{disabled ? 'opacity: 0.6;' : ''}
		"
	>
		<div style="padding: 8px 10px 4px;">
			<textarea
				bind:this={textareaEl}
				aria-expanded={showAutocomplete && filteredProjects.length > 0}
				aria-controls={showAutocomplete ? 'project-autocomplete' : undefined}
				aria-activedescendant={showAutocomplete && filteredProjects.length > 0 ? `project-option-${autocompleteIndex}` : undefined}
				bind:value={inputValue}
				{placeholder}
				rows={2}
				{disabled}
				style="
					width: 100%; background: none; border: none; outline: none;
					color: {theme.c.textPrimary}; font-family: var(--font-ui);
					font-size: 14px; resize: none; min-height: 36px;
					max-height: 200px; line-height: 1.5;
				"
				oninput={handleInput}
				onkeydown={handleKeydown}
				onfocus={(e) => {
					const p = (e.currentTarget as HTMLElement).closest('[style*="border-radius: 14px"]') as HTMLElement | null;
					if (p) {
						p.style.borderColor = theme.c.accentPrimary + '50';
						p.style.boxShadow = theme.c.shadowSm;
					}
				}}
				onblur={(e) => {
					const p = (e.currentTarget as HTMLElement).closest('[style*="border-radius: 14px"]') as HTMLElement | null;
					if (p) {
						p.style.borderColor = theme.c.borderDefault;
						p.style.boxShadow = theme.c.shadowSm;
					}
					// Delay hiding autocomplete so clicks on it register
					setTimeout(() => { showAutocomplete = false; }, 200);
				}}
			></textarea>
		</div>

		<div style="display: flex; align-items: center; gap: 4px; padding: 4px 8px 6px;">
			<div style="flex: 1;"></div>

			{#if modelName}
				<span style="
					padding: 2px 8px; border-radius: 10px; font-size: 10px;
					background: {theme.c.accentGlow}; color: {theme.c.textMuted};
					font-family: var(--font-mono); white-space: nowrap;
				">
					{modelName}
				</span>
			{/if}

			<button
				style="
					width: 30px; height: 30px; display: flex; align-items: center;
					justify-content: center;
					background: {disabled ? theme.c.textMuted : sendHovered ? theme.c.accentHover : theme.c.accentPrimary};
					border: none; border-radius: 10px;
					cursor: {disabled ? 'not-allowed' : 'pointer'};
					flex-shrink: 0; transition: all 0.15s ease;
					transform: {sendHovered && !disabled ? 'scale(1.1) rotate(-5deg)' : 'scale(1)'};
					box-shadow: {disabled ? 'none' : '0 2px 8px ' + theme.c.accentGlow};
				"
				onmouseenter={() => sendHovered = true}
				onmouseleave={() => sendHovered = false}
				onclick={send}
				{disabled}
				aria-label="Send message"
			>
				<svg width="14" height="14" viewBox="0 0 16 16" fill="white">
					<path d="M1.724 1.053a.5.5 0 0 1 .546-.065l13 6.5a.5.5 0 0 1 0 .894l-13 6.5a.5.5 0 0 1-.7-.58L3.39 8.5H8a.5.5 0 0 0 0-1H3.39L1.57 1.618a.5.5 0 0 1 .154-.565z"/>
				</svg>
			</button>
		</div>
	</div>

	<div style="display: flex; justify-content: space-between; padding: 4px 4px 0; font-size: 10px; color: {theme.c.textMuted};">
		<span>Enter to send, Shift+Enter for new line, @ to tag a project</span>
	</div>
</div>
