<script lang="ts">
	import { onMount } from 'svelte';
	import { getCommentStore } from '$lib/stores/comments.svelte';
	import type { AeComment } from '$lib/types';

	interface Props {
		taskId: string;
	}

	let { taskId }: Props = $props();
	const commentStore = getCommentStore();

	let commentInput = $state('');
	let posting = $state(false);
	let inputEl = $state<HTMLTextAreaElement | null>(null);
	let commentsEl = $state<HTMLDivElement | null>(null);
	let sendHovered = $state(false);

	let comments = $derived(commentStore.getComments(taskId));

	onMount(() => {
		commentStore.fetchComments(taskId);
	});

	async function handlePost() {
		const content = commentInput.trim();
		if (!content || posting) return;
		posting = true;
		try {
			await commentStore.postComment(taskId, 'matt', content);
			commentInput = '';
			// Scroll to bottom
			requestAnimationFrame(() => {
				if (commentsEl) {
					commentsEl.scrollTop = commentsEl.scrollHeight;
				}
			});
		} finally {
			posting = false;
		}
	}

	function handleKeyDown(e: KeyboardEvent) {
		if (e.key === 'Enter' && !e.shiftKey) {
			e.preventDefault();
			handlePost();
		}
	}

	function formatTime(iso: string): string {
		const d = new Date(iso);
		const now = new Date();
		const diff = now.getTime() - d.getTime();
		if (diff < 60000) return 'just now';
		if (diff < 3600000) return `${Math.floor(diff / 60000)}m ago`;
		if (diff < 86400000) return `${Math.floor(diff / 3600000)}h ago`;
		return d.toLocaleDateString();
	}

	/** Render content with @mentions highlighted, URLs clickable, and basic markdown */
	function renderContent(content: string): string {
		// First escape HTML to prevent XSS
		let safe = content
			.replace(/&/g, '&amp;')
			.replace(/</g, '&lt;')
			.replace(/>/g, '&gt;');

		// Extract code blocks and inline code to placeholders so they aren't
		// processed by bold/URL/mention/newline regexes.
		const codeSlots: string[] = [];
		function stash(html: string): string {
			codeSlots.push(html);
			return `\x00CODE${codeSlots.length - 1}\x00`;
		}

		// Code blocks (triple backticks) - use [\s\S] to allow backticks inside
		safe = safe.replace(/```([\s\S]*?)```/g, (_m, inner: string) =>
			stash(`<code style="display: block; background: rgba(0,0,0,0.3); padding: 8px 10px; border-radius: 6px; font-family: var(--font-mono); font-size: 11px; margin: 4px 0; white-space: pre-wrap; overflow-x: auto;">${inner}</code>`)
		);
		// Inline code (single backticks)
		safe = safe.replace(/`([^`]+)`/g, (_m, inner: string) =>
			stash(`<code style="background: rgba(0,0,0,0.25); padding: 1px 5px; border-radius: 3px; font-family: var(--font-mono); font-size: 11px;">${inner}</code>`)
		);

		// Bold (**text**)
		safe = safe.replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>');
		// Make URLs clickable
		safe = safe.replace(
			/(https?:\/\/[^\s<]+)/g,
			'<a href="$1" target="_blank" rel="noopener" style="color: var(--accent-indigo); text-decoration: underline; cursor: pointer;" onclick="event.stopPropagation()">$1</a>'
		);
		// Highlight @mentions
		safe = safe.replace(/@(\w+)/g, '<span style="color: var(--accent-indigo); font-weight: 600;">@$1</span>');
		// Newlines to <br> (only outside code blocks, which are stashed)
		safe = safe.replace(/\n/g, '<br>');

		// Restore code blocks/inline code
		safe = safe.replace(/\x00CODE(\d+)\x00/g, (_m, idx: string) => codeSlots[parseInt(idx)]);
		return safe;
	}

	function getAuthorInfo(author: string): { name: string; color: string; bg: string; isSystem: boolean } {
		if (author === 'agent') return { name: 'Agent', color: 'var(--accent-indigo)', bg: 'rgba(99, 102, 241, 0.1)', isSystem: false };
		if (author === 'system') return { name: 'System', color: 'var(--text-muted)', bg: 'rgba(110, 118, 129, 0.08)', isSystem: true };
		return { name: 'Matt', color: 'var(--accent-green)', bg: 'rgba(63, 185, 80, 0.1)', isSystem: false };
	}
</script>

<div style="display: flex; flex-direction: column; gap: 12px;">
	<div style="font-size: 11px; font-weight: 600; color: var(--text-muted); text-transform: uppercase; letter-spacing: 0.5px;">
		Comments
		{#if comments.length > 0}
			<span style="font-family: var(--font-mono); font-weight: 700; color: var(--text-secondary);">({comments.length})</span>
		{/if}
	</div>

	<!-- Comment list (scrollable) -->
	{#if comments.length > 0}
		<div
			bind:this={commentsEl}
			style="
				max-height: 50vh; overflow-y: auto;
				display: flex; flex-direction: column; gap: 8px;
				padding-right: 4px;
			"
		>
			{#each comments as comment (comment.id)}
				{@const info = getAuthorInfo(comment.author)}

				{#if info.isSystem}
					<!-- System message: compact, no avatar -->
					<div style="
						padding: 4px 10px; border-radius: 6px;
						background: rgba(110, 118, 129, 0.04);
						font-size: 11px; color: var(--text-muted);
						line-height: 1.5; animation: fade-in 0.2s ease;
					">
						<span style="font-weight: 600; margin-right: 6px; font-size: 10px; text-transform: uppercase; letter-spacing: 0.3px;">system</span>
						{@html renderContent(comment.content)}
						<span style="margin-left: 8px; font-size: 9px; opacity: 0.6;">{formatTime(comment.created_at)}</span>
					</div>
				{:else}
					<!-- User/Agent comment -->
					<div style="
						display: flex; gap: 10px; align-items: flex-start;
						animation: slide-in-left 0.2s ease;
					">
						<!-- Avatar -->
						<div style="
							width: 28px; height: 28px; border-radius: 50%; flex-shrink: 0;
							display: flex; align-items: center; justify-content: center;
							background: {info.bg}; color: {info.color};
							font-size: 12px; font-weight: 700;
						">
							{#if comment.author === 'agent'}
								<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
									<path d="M8 0a1 1 0 011 1v1.07A6.002 6.002 0 0114 8v3a2 2 0 01-2 2H4a2 2 0 01-2-2V8a6.002 6.002 0 015-5.93V1a1 1 0 011-1zM6 9a1 1 0 100 2 1 1 0 000-2zm4 0a1 1 0 100 2 1 1 0 000-2z"/>
								</svg>
							{:else}
								<svg width="14" height="14" viewBox="0 0 16 16" fill="currentColor">
									<path d="M8 8a3 3 0 100-6 3 3 0 000 6zm-5 6s-1 0-1-1 1-4 6-4 6 3 6 4-1 1-1 1H3z"/>
								</svg>
							{/if}
						</div>

						<!-- Content -->
						<div style="flex: 1; min-width: 0;">
							<div style="display: flex; align-items: center; gap: 6px; margin-bottom: 3px;">
								<span style="font-size: 12px; font-weight: 700; color: {info.color};">{info.name}</span>
								<span style="font-size: 9px; color: var(--text-muted);">{formatTime(comment.created_at)}</span>
							</div>
							<div style="
								font-size: 12px; color: var(--text-secondary); line-height: 1.5;
								word-break: break-word;
							">
								{@html renderContent(comment.content)}
							</div>
						</div>
					</div>
				{/if}
			{/each}
		</div>
	{:else}
		<div style="
			padding: 16px; text-align: center;
			color: var(--text-muted); font-size: 11px;
			border: 1px dashed var(--border-default);
			border-radius: 8px; opacity: 0.5;
		">
			No comments yet. Be the first to comment.
		</div>
	{/if}

	<!-- Input -->
	<div style="
		display: flex; gap: 8px; align-items: flex-end;
	">
		<div style="flex: 1; position: relative;">
			<textarea
				bind:this={inputEl}
				bind:value={commentInput}
				placeholder="Add a comment... (use @agent or @matt for mentions)"
				rows={2}
				style="
					width: 100%; padding: 8px 12px;
					background: var(--bg-primary); border: 1px solid var(--border-default);
					border-radius: 10px; color: var(--text-primary);
					font-family: var(--font-ui); font-size: 12px;
					outline: none; resize: none; line-height: 1.5;
					transition: border-color 0.15s;
				"
				onfocus={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'rgba(99, 102, 241, 0.3)'; }}
				onblur={(e) => { (e.currentTarget as HTMLElement).style.borderColor = 'var(--border-default)'; }}
				onkeydown={handleKeyDown}
				disabled={posting}
			></textarea>
		</div>
		<button
			style="
				height: 38px; padding: 0 14px; border-radius: 10px;
				background: {!commentInput.trim() || posting ? 'var(--text-muted)' : sendHovered ? 'var(--accent-hover)' : 'var(--accent-primary)'};
				border: none; color: white; font-size: 11px; font-weight: 700;
				font-family: var(--font-ui); cursor: {!commentInput.trim() || posting ? 'not-allowed' : 'pointer'};
				transition: all 0.15s ease; flex-shrink: 0;
				transform: {sendHovered && commentInput.trim() && !posting ? 'scale(1.05)' : 'scale(1)'};
				box-shadow: {commentInput.trim() && !posting ? '0 2px 8px rgba(99, 102, 241, 0.25)' : 'none'};
			"
			onmouseenter={() => sendHovered = true}
			onmouseleave={() => sendHovered = false}
			onclick={handlePost}
			disabled={!commentInput.trim() || posting}
		>
			{posting ? '...' : 'Post'}
		</button>
	</div>
	<div style="font-size: 9px; color: var(--text-muted); margin-top: -4px;">
		Enter to post, Shift+Enter for new line
	</div>
</div>
