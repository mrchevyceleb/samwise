/** Comment store using Svelte 5 runes - manages @mention threads on task cards */

import type { AeComment, CommentAuthor } from '$lib/types';
import { safeInvoke } from '$lib/utils/tauri';
import { SvelteMap } from 'svelte/reactivity';

// SvelteMap gives per-KEY reactivity: setting comments for one task only
// invalidates readers of THAT task's comments. The previous plain-Map +
// `commentsByTask = new Map(...)` reassignment invalidated every card on
// every comment event, which (under the software-rendered WebView) pinned
// the CPU at 100% whenever the worker was posting progress comments.
const commentsByTask = new SvelteMap<string, AeComment[]>();
let loading = $state(false);
let error = $state<string | null>(null);

/** Parse @mentions from content string */
function parseMentions(content: string): string[] {
  const matches = content.match(/@(\w+)/g);
  if (!matches) return [];
  return [...new Set(matches.map((m) => m.slice(1)))];
}

export function getCommentStore() {
  return {
    get loading() { return loading; },
    get error() { return error; },

    getComments(taskId: string): AeComment[] {
      return commentsByTask.get(taskId) || [];
    },

    getCommentCount(taskId: string): number {
      return (commentsByTask.get(taskId) || []).length;
    },

    /** Get the latest agent comment content for a task (for card preview) */
    getLatestComment(taskId: string): string | null {
      const comments = commentsByTask.get(taskId) || [];
      // Find last agent comment
      for (let i = comments.length - 1; i >= 0; i--) {
        if (comments[i].author === 'agent') {
          return comments[i].content;
        }
      }
      return null;
    },

    async fetchComments(taskId: string) {
      loading = true;
      error = null;
      try {
        const result = await safeInvoke<AeComment[]>('supabase_fetch_comments', {
          taskId,
        });
        if (result) {
          commentsByTask.set(taskId, result);
        }
      } catch (e) {
        error = String(e);
      } finally {
        loading = false;
      }
    },

    async postComment(taskId: string, author: CommentAuthor, content: string): Promise<AeComment | null> {
      const mentions = parseMentions(content);
      try {
        const result = await safeInvoke<AeComment[]>('supabase_post_comment', {
          comment: {
            task_id: taskId,
            author,
            content,
            mentions,
          },
        });
        if (result && Array.isArray(result) && result.length > 0) {
          const newComment = result[0] as AeComment;
          // Add to local state
          const existing = commentsByTask.get(taskId) || [];
          commentsByTask.set(taskId, [...existing, newComment]);
          return newComment;
        }
      } catch (e) {
        error = String(e);
      }
      return null;
    },

    async deleteComment(commentId: string, taskId: string): Promise<boolean> {
      try {
        await safeInvoke('supabase_delete_comment', { commentId });
        const existing = commentsByTask.get(taskId) || [];
        commentsByTask.set(taskId, existing.filter((c) => c.id !== commentId));
        return true;
      } catch (e) {
        error = String(e);
        return false;
      }
    },

    /** Apply a realtime comment insert */
    applyRealtimeComment(comment: AeComment) {
      const taskId = comment.task_id;
      const existing = commentsByTask.get(taskId) || [];
      if (!existing.find((c) => c.id === comment.id)) {
        commentsByTask.set(taskId, [...existing, comment]);
      }
    },

    /** Check if any comments mention a specific user */
    hasUnreadMention(taskId: string, user: string): boolean {
      const comments = commentsByTask.get(taskId) || [];
      return comments.some((c) => c.mentions.includes(user));
    },
  };
}
