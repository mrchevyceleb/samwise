<script lang="ts">
	interface Props {
		disabled?: boolean;
		placeholder?: string;
		onSend: (message: string) => void;
		modelName?: string;
	}

	let { disabled = false, placeholder = 'Plan, @ for context, / for commands', onSend, modelName = '' }: Props = $props();

	let inputValue = $state('');
	let sendHovered = $state(false);
	let imgHovered = $state(false);
	let micHovered = $state(false);
	let textareaEl = $state<HTMLTextAreaElement | null>(null);
	let showAtHint = $state(false);
	let showSlashHint = $state(false);

	function autoResize() {
		if (!textareaEl) return;
		textareaEl.style.height = 'auto';
		textareaEl.style.height = Math.min(textareaEl.scrollHeight, 200) + 'px';
	}

	function handleKeydown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			send();
		}
	}

	function handleInput() {
		autoResize();
		// Show hints based on input
		const val = inputValue;
		showAtHint = val.endsWith('@');
		showSlashHint = val === '/';
	}

	function send() {
		const trimmed = inputValue.trim();
		if (!trimmed || disabled) return;
		onSend(trimmed);
		inputValue = '';
		showAtHint = false;
		showSlashHint = false;
		if (textareaEl) {
			textareaEl.style.height = 'auto';
		}
	}

	$effect(() => {
		if (inputValue !== undefined) {
			autoResize();
		}
	});
</script>

<div style="padding: 10px 12px; position: relative; background: linear-gradient(0deg, rgba(14, 18, 24, 0.8) 0%, transparent 100%);">
	<!-- @ mention hint -->
	{#if showAtHint}
		<div style="
			position: absolute; bottom: 100%; left: 16px; margin-bottom: 4px;
			padding: 6px 10px; background: var(--bg-elevated); border: 1px solid var(--border-default);
			border-radius: 6px; font-size: 11px; color: var(--text-muted); box-shadow: 0 4px 12px rgba(0,0,0,0.3);
		">
			File mentions coming soon
		</div>
	{/if}

	<!-- / command hint -->
	{#if showSlashHint}
		<div style="
			position: absolute; bottom: 100%; left: 16px; margin-bottom: 4px;
			padding: 6px 10px; background: var(--bg-elevated); border: 1px solid var(--border-default);
			border-radius: 6px; font-size: 11px; color: var(--text-muted); box-shadow: 0 4px 12px rgba(0,0,0,0.3);
		">
			Slash commands coming soon
		</div>
	{/if}

	<div
		style="
			display: flex; flex-direction: column; gap: 0;
			background: linear-gradient(180deg, rgba(26, 32, 40, 0.9) 0%, rgba(20, 25, 32, 0.95) 100%);
			border: 1px solid rgba(255, 255, 255, 0.07);
			border-radius: 14px; overflow: hidden;
			transition: border-color 0.2s ease, box-shadow 0.2s ease;
			box-shadow: 0 4px 16px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.04);
			{disabled ? 'opacity: 0.6;' : ''}
		"
	>
		<!-- Textarea -->
		<div style="padding: 8px 10px 4px;">
			<textarea
				bind:this={textareaEl}
				bind:value={inputValue}
				{placeholder}
				rows={2}
				{disabled}
				style="
					width: 100%; background: none; border: none; outline: none;
					color: var(--text-primary); font-family: var(--font-ui);
					font-size: 13px; resize: none; min-height: 36px;
					max-height: 200px; line-height: 1.5;
				"
				oninput={handleInput}
				onkeydown={handleKeydown}
				onfocus={(e) => { const p = (e.currentTarget as HTMLElement).closest('[style*="border-radius: 14px"]') as HTMLElement | null; if (p) { p.style.borderColor = 'color-mix(in srgb, var(--accent-primary) 30%, transparent)'; p.style.boxShadow = '0 4px 16px rgba(0, 0, 0, 0.3), 0 0 0 1px color-mix(in srgb, var(--accent-primary) 10%, transparent), inset 0 1px 0 rgba(255, 255, 255, 0.04)'; } }}
				onblur={(e) => { const p = (e.currentTarget as HTMLElement).closest('[style*="border-radius: 14px"]') as HTMLElement | null; if (p) { p.style.borderColor = 'rgba(255, 255, 255, 0.07)'; p.style.boxShadow = '0 4px 16px rgba(0, 0, 0, 0.3), inset 0 1px 0 rgba(255, 255, 255, 0.04)'; } showAtHint = false; showSlashHint = false; }}
			></textarea>
		</div>

		<!-- Toolbar row -->
		<div style="display: flex; align-items: center; gap: 4px; padding: 4px 8px 6px;">
			<!-- Image button -->
			<button
				style="
					width: 26px; height: 26px; display: flex; align-items: center; justify-content: center;
					border: none; border-radius: 5px; cursor: pointer; transition: all 0.12s ease;
					background: {imgHovered ? 'rgba(255,255,255,0.06)' : 'transparent'};
					color: {imgHovered ? 'var(--text-primary)' : 'var(--text-muted)'};
					transform: {imgHovered ? 'rotate(-5deg)' : 'rotate(0)'};
				"
				onmouseenter={() => imgHovered = true}
				onmouseleave={() => imgHovered = false}
				title="Attach image"
			>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><circle cx="8.5" cy="8.5" r="1.5"/><polyline points="21 15 16 10 5 21"/>
				</svg>
			</button>

			<!-- Mic button -->
			<button
				style="
					width: 26px; height: 26px; display: flex; align-items: center; justify-content: center;
					border: none; border-radius: 5px; cursor: pointer; transition: all 0.12s ease;
					background: {micHovered ? 'rgba(255,255,255,0.06)' : 'transparent'};
					color: {micHovered ? 'var(--text-primary)' : 'var(--text-muted)'};
				"
				onmouseenter={() => micHovered = true}
				onmouseleave={() => micHovered = false}
				title="Voice input (coming soon)"
			>
				<svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<path d="M12 1a3 3 0 0 0-3 3v8a3 3 0 0 0 6 0V4a3 3 0 0 0-3-3z"/><path d="M19 10v2a7 7 0 0 1-14 0v-2"/><line x1="12" y1="19" x2="12" y2="23"/><line x1="8" y1="23" x2="16" y2="23"/>
				</svg>
			</button>

			<div style="flex: 1;"></div>

			<!-- Model pill -->
			{#if modelName}
				<span style="
					padding: 2px 8px; border-radius: 10px; font-size: 10px;
					background: color-mix(in srgb, var(--accent-primary) 6%, transparent); color: var(--text-muted);
					font-family: var(--font-mono); white-space: nowrap;
				">
					{modelName}
				</span>
			{/if}

			<!-- Local indicator -->
			<span style="
				padding: 2px 6px; border-radius: 4px; font-size: 10px;
				color: var(--text-muted); font-family: var(--font-ui);
				display: flex; align-items: center; gap: 3px;
			">
				<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
					<rect x="2" y="3" width="20" height="14" rx="2" ry="2"/><line x1="8" y1="21" x2="16" y2="21"/><line x1="12" y1="17" x2="12" y2="21"/>
				</svg>
				Local
			</span>

			<!-- Send button -->
			<button
				style="
					width: 30px; height: 30px; display: flex; align-items: center;
					justify-content: center;
					background: {disabled ? 'var(--text-muted)' : sendHovered ? 'var(--accent-hover)' : 'var(--accent-primary)'};
					border: none; border-radius: 10px;
					cursor: {disabled ? 'not-allowed' : 'pointer'};
					flex-shrink: 0; transition: all 0.15s ease;
					transform: {sendHovered && !disabled ? 'scale(1.1) rotate(-5deg)' : 'scale(1)'};
					box-shadow: {disabled ? 'none' : sendHovered ? '0 4px 16px color-mix(in srgb, var(--accent-primary) 35%, transparent)' : '0 2px 8px color-mix(in srgb, var(--accent-primary) 20%, transparent)'};
				"
				onmouseenter={() => sendHovered = true}
				onmouseleave={() => sendHovered = false}
				onclick={send}
				{disabled}
				aria-label="Send message"
			>
				<svg width="14" height="14" viewBox="0 0 16 16" fill="#0D1117">
					<path d="M1.724 1.053a.5.5 0 0 1 .546-.065l13 6.5a.5.5 0 0 1 0 .894l-13 6.5a.5.5 0 0 1-.7-.58L3.39 8.5H8a.5.5 0 0 0 0-1H3.39L1.57 1.618a.5.5 0 0 1 .154-.565z"/>
				</svg>
			</button>
		</div>
	</div>

	<div style="display: flex; justify-content: space-between; padding: 4px 4px 0; font-size: 10px; color: var(--text-muted);">
		<span>Enter to send, Shift+Enter for new line</span>
	</div>
</div>
