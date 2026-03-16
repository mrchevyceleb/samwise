<script lang="ts">
	import type { ClaudeCodeMessage } from '$lib/stores/claude-code.svelte';
	import { renderMarkdown } from '$lib/utils/markdown';

	interface Props {
		message: ClaudeCodeMessage;
		fontSize?: number;
		fontFamily?: string;
	}

	let { message, fontSize = 14, fontFamily = 'inherit' }: Props = $props();

	// Extract user-attached images
	let userImages = $derived((() => {
		if (message.type !== 'user') return [];
		if (typeof message.raw === 'object' && message.raw !== null && Array.isArray(message.raw.images)) {
			return message.raw.images as Array<{ base64: string; mediaType: string }>;
		}
		return [];
	})());

	// Extract text content
	let textContent = $derived((() => {
		if (message.type === 'user') {
			if (typeof message.raw === 'string') return message.raw;
			if (typeof message.raw === 'object' && message.raw !== null && typeof message.raw.text === 'string') return message.raw.text;
			return '';
		}
		if (message.type === 'assistant') {
			const content = message.raw?.message?.content;
			if (Array.isArray(content)) {
				return content
					.filter((c: any) => c.type === 'text' && c.text)
					.map((c: any) => c.text.trim())
					.filter(Boolean)
					.join('\n')
					.trim();
			}
			return '';
		}
		if (message.type === 'result') {
			return message.raw?.result || '';
		}
		if (message.type === 'system') {
			if (message.raw?.subtype === 'init') {
				const model = (message.raw.model || 'unknown').replace(/\[.*\]/, '');
				return `Session started. Model: ${model}`;
			}
			return '';
		}
		if (message.type === 'error') {
			return message.raw?.error || JSON.stringify(message.raw);
		}
		if (message.type === 'stderr') {
			return typeof message.raw === 'string' ? message.raw : JSON.stringify(message.raw);
		}
		return '';
	})());

	// Extract tool use blocks
	let toolUses = $derived((() => {
		if (message.type !== 'assistant') return [];
		const content = message.raw?.message?.content;
		if (!Array.isArray(content)) return [];
		return content.filter((c: any) => c.type === 'tool_use');
	})());

	// Cost info from result
	let costInfo = $derived((() => {
		if (message.type !== 'result') return null;
		return {
			cost: message.raw.total_cost_usd,
			duration: message.raw.duration_ms,
			turns: message.raw.num_turns,
		};
	})());

	let isUser = $derived(message.type === 'user');
	let isSystem = $derived(message.type === 'system');
	let isResult = $derived(message.type === 'result');
	let isError = $derived(message.type === 'error');
	let isAssistant = $derived(message.type === 'assistant');

	// Streaming detection: require message.raw.message to exist before considering streaming
	let isStreaming = $derived(
		isAssistant && !!message.raw?.message && !message.raw.message.usage && !message.raw.message.stop_reason
	);

	// Debounced markdown rendering
	let renderedHtml = $state('');
	let renderVersion = 0;
	let debounceTimer: ReturnType<typeof setTimeout> | null = null;

	function sanitizeHtml(html: string): string {
		return html
			.replace(/\s+on\w+\s*=\s*("[^"]*"|'[^']*'|[^\s>]*)/gi, '')
			.replace(/href\s*=\s*"javascript:[^"]*"/gi, 'href="#"')
			.replace(/href\s*=\s*'javascript:[^']*'/gi, "href='#'")
			.replace(/src\s*=\s*"javascript:[^"]*"/gi, 'src=""')
			.replace(/src\s*=\s*'javascript:[^']*'/gi, "src=''");
	}

	function doRender(text: string) {
		const thisVersion = ++renderVersion;
		renderMarkdown(text).then((html) => {
			if (thisVersion === renderVersion) {
				renderedHtml = sanitizeHtml(html);
			}
		});
	}

	$effect(() => {
		if (!isAssistant || !textContent) {
			renderedHtml = '';
			if (debounceTimer) { clearTimeout(debounceTimer); debounceTimer = null; }
			return () => {};
		}
		if (isStreaming) {
			if (debounceTimer) clearTimeout(debounceTimer);
			debounceTimer = setTimeout(() => { debounceTimer = null; doRender(textContent); }, 300);
		} else {
			if (debounceTimer) { clearTimeout(debounceTimer); debounceTimer = null; }
			doRender(textContent);
		}
		return () => {
			if (debounceTimer) { clearTimeout(debounceTimer); debounceTimer = null; }
		};
	});

	// Tool expand state
	let expandedTools = $state<Set<number>>(new Set());
	function toggleTool(idx: number) {
		const next = new Set(expandedTools);
		if (next.has(idx)) next.delete(idx);
		else next.add(idx);
		expandedTools = next;
	}
</script>

{#if isUser && (textContent || userImages.length > 0)}
	<!-- User message -->
	<div style="display: flex; justify-content: flex-end;">
		<div
			style="
				max-width: 85%; padding: 10px 14px; border-radius: 12px;
				background: linear-gradient(135deg, color-mix(in srgb, var(--accent-primary) 12%, transparent), color-mix(in srgb, var(--accent-primary) 6%, transparent));
				border: 1px solid color-mix(in srgb, var(--accent-primary) 20%, transparent);
				color: var(--text-primary); font-size: {fontSize}px; font-family: {fontFamily};
				user-select: text;
			"
		>
			{#if userImages.length > 0}
				<div style="display: flex; flex-wrap: wrap; gap: 8px; {textContent ? 'margin-bottom: 8px;' : ''}">
					{#each userImages as img}
						<img
							src="data:{img.mediaType};base64,{img.base64}"
							alt="Attached"
							style="max-width: 240px; max-height: 180px; object-fit: contain; border-radius: 8px; border: 1px solid var(--border-default);"
						/>
					{/each}
				</div>
			{/if}
			{#if textContent}
				<pre style="margin: 0; white-space: pre-wrap; word-break: break-word; font-family: inherit;">{textContent}</pre>
			{/if}
		</div>
	</div>
{:else if isSystem}
	<!-- System message -->
	<div style="display: flex; justify-content: center;">
		<span style="font-size: {fontSize - 3}px; color: var(--text-muted); font-family: var(--font-mono);">
			{textContent}
		</span>
	</div>
{:else if isResult}
	<!-- Result summary -->
	<div style="display: flex; justify-content: center;">
		<div
			style="
				padding: 6px 14px; border-radius: 8px; font-size: {fontSize - 3}px;
				border: 1px solid rgba(34, 197, 94, 0.15); background: rgba(34, 197, 94, 0.05);
				font-family: var(--font-mono); display: flex; align-items: center; gap: 8px;
			"
		>
			<svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="rgb(34, 197, 94)" stroke-width="2.5">
				<polyline points="20 6 9 17 4 12"/>
			</svg>
			<span style="color: var(--text-muted);">
				Done
				{#if costInfo}
					<span style="color: var(--accent-dim);">
						{costInfo.duration != null ? `${(costInfo.duration / 1000).toFixed(1)}s` : ''}
						{costInfo.cost != null ? ` $${costInfo.cost.toFixed(4)}` : ''}
						{costInfo.turns ? ` ${costInfo.turns} turn${costInfo.turns > 1 ? 's' : ''}` : ''}
					</span>
				{/if}
			</span>
		</div>
	</div>
{:else if message.type === 'stderr'}
	<!-- Stderr output -->
	<div
		style="
			padding: 8px 12px; border-radius: 8px; font-size: {fontSize - 2}px;
			border: 1px solid rgba(234, 179, 8, 0.15); background: rgba(234, 179, 8, 0.04);
			color: rgba(234, 179, 8, 0.8); user-select: text;
		"
	>
		<pre style="margin: 0; white-space: pre-wrap; word-break: break-word; font-family: var(--font-mono);">{textContent}</pre>
	</div>
{:else if isError}
	<!-- Error -->
	<div
		style="
			padding: 10px 14px; border-radius: 8px; font-size: {fontSize}px;
			border: 1px solid rgba(239, 68, 68, 0.2); background: rgba(239, 68, 68, 0.06);
			color: rgb(239, 68, 68); user-select: text;
		"
	>
		<pre style="margin: 0; white-space: pre-wrap; word-break: break-word; font-family: var(--font-mono);">{textContent}</pre>
	</div>
{:else if isAssistant}
	<!-- Assistant message -->
	<div style="display: flex; flex-direction: column; gap: 8px;">
		{#if renderedHtml}
			<div
				class="cc-prose"
				style="
					padding: 10px 14px; border-radius: 12px; max-width: 95%;
					background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.06);
					font-size: {fontSize}px; font-family: {fontFamily};
					color: var(--text-primary); user-select: text;
				"
			>
				{@html renderedHtml}
			</div>
		{:else if textContent}
			<div
				style="
					padding: 10px 14px; border-radius: 12px; max-width: 95%;
					background: rgba(255, 255, 255, 0.03); border: 1px solid rgba(255, 255, 255, 0.06);
					font-size: {fontSize}px; font-family: {fontFamily};
					color: var(--text-primary); user-select: text;
				"
			>
				<pre style="margin: 0; white-space: pre-wrap; word-break: break-word; font-family: inherit;">{textContent}</pre>
			</div>
		{/if}

		<!-- Tool use blocks -->
		{#each toolUses as tool, idx}
			<div
				style="
					border-radius: 8px; overflow: hidden;
					border: 1px solid color-mix(in srgb, var(--accent-primary) 15%, transparent);
					border-left: 3px solid var(--accent-primary);
					background: color-mix(in srgb, var(--accent-primary) 3%, transparent);
					font-size: {fontSize - 2}px;
				"
			>
				<button
					style="
						width: 100%; display: flex; align-items: center; gap: 8px;
						padding: 6px 10px; border: none; background: none; cursor: pointer;
						text-align: left; color: var(--text-secondary); font-family: var(--font-mono);
						transition: background 0.1s ease;
					"
					onmouseenter={(e) => { (e.currentTarget as HTMLElement).style.background = 'rgba(255,255,255,0.03)'; }}
					onmouseleave={(e) => { (e.currentTarget as HTMLElement).style.background = 'none'; }}
					onclick={() => toggleTool(idx)}
				>
					<div style="width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; background: {isStreaming ? 'var(--accent-primary)' : 'color-mix(in srgb, var(--accent-primary) 40%, transparent)'}; {isStreaming ? 'animation: cc-pulse 1.5s ease-in-out infinite;' : ''}"></div>
					<span style="flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;">{tool.name}</span>
					<svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" style="flex-shrink: 0; opacity: 0.4; transition: transform 0.15s; transform: {expandedTools.has(idx) ? 'rotate(90deg)' : 'rotate(0)'};">
						<polyline points="9 18 15 12 9 6"/>
					</svg>
				</button>

				{#if expandedTools.has(idx)}
					<div style="border-top: 1px solid color-mix(in srgb, var(--accent-primary) 10%, transparent); padding: 8px 10px;">
						{#if tool.input}
							<div>
								<div style="font-size: {fontSize - 4}px; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.04em; font-weight: 500; margin-bottom: 4px;">Input</div>
								<pre style="margin: 0; padding: 8px; background: var(--bg-primary); border: 1px solid var(--border-default); border-radius: 6px; font-family: var(--font-mono); font-size: {fontSize - 3}px; color: var(--text-secondary); white-space: pre-wrap; word-break: break-word; max-height: 192px; overflow: auto; user-select: text;">{JSON.stringify(tool.input, null, 2)}</pre>
							</div>
						{/if}
					</div>
				{/if}
			</div>
		{/each}
	</div>
{/if}

<style>
	@keyframes cc-pulse {
		0%, 100% { opacity: 0.4; }
		50% { opacity: 0.8; }
	}

	/* Chat prose styles */
	.cc-prose :global(p) { margin: 0.35rem 0; }
	.cc-prose :global(p:first-child) { margin-top: 0; }
	.cc-prose :global(p:last-child) { margin-bottom: 0; }

	.cc-prose :global(.code-block-wrapper) {
		border: 1px solid var(--border-default);
		border-radius: 8px;
		overflow: hidden;
		margin: 0.5rem 0;
	}
	.cc-prose :global(.code-block-header) {
		background: var(--bg-elevated);
		border-bottom: 1px solid var(--border-default);
		font-size: 0.75em;
	}
	.cc-prose :global(.code-block-lang) {
		color: var(--text-muted);
		text-transform: uppercase;
		letter-spacing: 0.04em;
		font-weight: 500;
	}
	.cc-prose :global(.code-block-copy) {
		background: transparent;
		border: 1px solid var(--border-default);
		border-radius: 4px;
		color: var(--text-secondary);
		cursor: pointer;
		font-size: 0.9em;
		transition: all 0.15s ease;
	}
	.cc-prose :global(.code-block-copy:hover) {
		background: var(--accent-primary);
		color: var(--bg-primary);
		border-color: var(--accent-primary);
	}
	.cc-prose :global(pre.shiki) {
		background: var(--bg-primary) !important;
		border: none;
		border-radius: 0;
		padding: 0.75rem 1rem;
		margin: 0;
		overflow-x: auto;
		font-size: 0.85em;
	}
	.cc-prose :global(pre:not(.shiki)) {
		background: var(--bg-primary);
		border: 1px solid var(--border-default);
		border-radius: 8px;
		padding: 0.75rem 1rem;
		margin: 0.5rem 0;
		overflow-x: auto;
		font-size: 0.85em;
	}
	.cc-prose :global(code) {
		font-family: var(--font-mono);
		background: var(--bg-elevated);
		padding: 0.1em 0.35em;
		border-radius: 4px;
		font-size: 0.9em;
		color: var(--accent-primary);
	}
	.cc-prose :global(pre code) {
		background: none;
		padding: 0;
		color: inherit;
	}
	.cc-prose :global(a) {
		color: var(--accent-primary);
		text-decoration: none;
		transition: color 0.15s ease;
	}
	.cc-prose :global(a:hover) {
		text-decoration: underline;
		filter: brightness(1.15);
	}
	.cc-prose :global(ul) { list-style-type: disc; padding-left: 1.5rem; margin: 0.4rem 0; }
	.cc-prose :global(ol) { list-style-type: decimal; padding-left: 1.5rem; margin: 0.4rem 0; }
	.cc-prose :global(li) { margin: 0.3rem 0; color: var(--text-secondary); }
	.cc-prose :global(li::marker) { color: var(--accent-primary); font-weight: 700; }
	.cc-prose :global(h1) { font-size: 1.35em; font-weight: 800; margin: 1rem 0 0.4rem; color: #ffffff; border-bottom: 1px solid var(--border-default); padding-bottom: 0.3rem; }
	.cc-prose :global(h2) { font-size: 1.2em; font-weight: 700; margin: 0.85rem 0 0.35rem; color: var(--accent-primary); }
	.cc-prose :global(h3) { font-size: 1.08em; font-weight: 700; margin: 0.75rem 0 0.3rem; color: rgb(34, 197, 94); }
	.cc-prose :global(blockquote) {
		border-left: 3px solid var(--accent-primary);
		background: color-mix(in srgb, var(--accent-primary) 4%, transparent);
		padding: 0.35rem 0.85rem;
		margin: 0.5rem 0;
		border-radius: 0 6px 6px 0;
		color: var(--text-secondary);
	}
	.cc-prose :global(table) { width: 100%; border-collapse: collapse; margin: 0.5rem 0; font-size: 0.9em; }
	.cc-prose :global(th), .cc-prose :global(td) { border: 1px solid var(--border-default); padding: 0.4rem 0.6rem; }
	.cc-prose :global(th) { background: var(--bg-elevated); font-weight: 700; color: var(--accent-primary); }
	.cc-prose :global(strong) { font-weight: 700; color: var(--accent-primary); }
	.cc-prose :global(em) { color: color-mix(in srgb, var(--accent-primary) 85%, transparent); font-style: italic; }
	.cc-prose :global(hr) { border: none; border-top: 1px solid var(--border-default); margin: 0.85rem 0; opacity: 0.6; }
</style>
