/** Git store using Svelte 5 runes */

import { getSettings } from './settings.svelte';

async function getInvoke() {
	const { invoke } = await import('@tauri-apps/api/core');
	return invoke;
}

export interface GitFileStatus {
  path: string;
  status: string;
  staged: boolean;
  conflicted: boolean;
}

export interface GitStatus {
  branch: string;
  files: GitFileStatus[];
  ahead: number;
  behind: number;
}

export interface GitCommitInfo {
  hash: string;
  short_hash: string;
  author: string;
  author_email: string;
  timestamp: number;
  message: string;
}

export interface GitBranchInfo {
  name: string;
  is_current: boolean;
  is_remote: boolean;
}

let status = $state<GitStatus | null>(null);
let branches = $state<GitBranchInfo[]>([]);
let logEntries = $state<GitCommitInfo[]>([]);
let loading = $state(false);
let error = $state<string | null>(null);
let selectedFile = $state<string | null>(null);
let diffContent = $state<string>('');

export function getGitStore() {
  return {
    get status() { return status; },
    get branches() { return branches; },
    get log() { return logEntries; },
    get loading() { return loading; },
    get error() { return error; },
    get selectedFile() { return selectedFile; },
    set selectedFile(v: string | null) { selectedFile = v; },
    get diffContent() { return diffContent; },

    async refresh(projectDir: string) {
      if (!projectDir) return;
      const invoke = await getInvoke();
      loading = true;
      error = null;
      try {
        const [s, b, l] = await Promise.all([
          invoke<GitStatus>('git_status', { projectDir }),
          invoke<GitBranchInfo[]>('git_branch_list', { projectDir }),
          invoke<GitCommitInfo[]>('git_log', { projectDir, count: 50 }),
        ]);
        status = s;
        branches = b;
        logEntries = l;
      } catch (e) {
        error = String(e);
      } finally {
        loading = false;
      }
    },

    async stageFile(projectDir: string, filePath: string) {
      const invoke = await getInvoke();
      await invoke('git_stage_file', { projectDir, filePath });
      await this.refresh(projectDir);
    },

    async unstageFile(projectDir: string, filePath: string) {
      const invoke = await getInvoke();
      await invoke('git_unstage_file', { projectDir, filePath });
      await this.refresh(projectDir);
    },

    async stageAll(projectDir: string) {
      const invoke = await getInvoke();
      await invoke('git_stage_all', { projectDir });
      await this.refresh(projectDir);
    },

    async unstageAll(projectDir: string) {
      const invoke = await getInvoke();
      await invoke('git_unstage_all', { projectDir });
      await this.refresh(projectDir);
    },

    async discardFile(projectDir: string, filePath: string) {
      const invoke = await getInvoke();
      await invoke('git_discard_file', { projectDir, filePath });
      await this.refresh(projectDir);
    },

    async commit(projectDir: string, message: string, files: string[] = []) {
      const invoke = await getInvoke();
      const hash = await invoke<string>('git_commit', { projectDir, message, files });
      await this.refresh(projectDir);
      return hash;
    },

    async checkout(projectDir: string, branch: string) {
      const invoke = await getInvoke();
      await invoke('git_checkout', { projectDir, branch });
      await this.refresh(projectDir);
    },

    async createBranch(projectDir: string, branchName: string) {
      const invoke = await getInvoke();
      await invoke('git_create_branch', { projectDir, branchName });
      await this.refresh(projectDir);
    },

    async getDiff(projectDir: string, filePath: string, staged: boolean = false) {
      const invoke = await getInvoke();
      diffContent = await invoke<string>('git_diff', { projectDir, filePath, staged });
      return diffContent;
    },

    async stash(projectDir: string) {
      const invoke = await getInvoke();
      await invoke('git_stash', { projectDir });
      await this.refresh(projectDir);
    },

    async stashPop(projectDir: string) {
      const invoke = await getInvoke();
      await invoke('git_stash_pop', { projectDir });
      await this.refresh(projectDir);
    },

    async push(projectDir: string) {
      const invoke = await getInvoke();
      await invoke('git_push', { projectDir });
      await this.refresh(projectDir);
    },

    async pull(projectDir: string) {
      const invoke = await getInvoke();
      await invoke('git_pull', { projectDir });
      await this.refresh(projectDir);
    },

    async getStagedDiff(projectDir: string): Promise<string> {
      const invoke = await getInvoke();
      return invoke<string>('git_diff_staged', { projectDir });
    },

    async generateCommitMessage(projectDir: string): Promise<string> {
      const diff = await this.getStagedDiff(projectDir);
      if (!diff.trim()) {
        throw new Error('No staged changes to summarize.');
      }

      const truncated = diff.length > 8000 ? diff.slice(0, 8000) + '\n... (truncated)' : diff;
      const systemPrompt = 'You write concise git commit messages. Output ONLY the commit message, nothing else. Use conventional commit format (feat:, fix:, refactor:, docs:, chore:, style:, test:). Keep the first line under 72 chars. Add a blank line and brief body only if the change is complex.';
      const userPrompt = `Write a commit message for this diff:\n\n${truncated}`;

      const settings = getSettings();

      // Fallback 1: OpenAI (OAuth token or API key)
      const openaiKey = (settings.aiOpenAIOAuthAccessToken || '').trim() || (settings.aiOpenAIApiKey || '').trim();
      if (openaiKey) {
        return this._callOpenAI(openaiKey, systemPrompt, userPrompt);
      }

      // Fallback 2: Anthropic API key
      const anthropicKey = (settings.aiAnthropicApiKey || '').trim();
      if (anthropicKey) {
        return this._callAnthropic(anthropicKey, systemPrompt, userPrompt);
      }

      // Fallback 3: OpenRouter API key
      const openrouterKey = (settings.aiOpenRouterApiKey || '').trim();
      if (openrouterKey) {
        return this._callOpenRouter(openrouterKey, systemPrompt, userPrompt);
      }

      // Fallback 4: Claude Code CLI
      try {
        const invoke = await getInvoke();
        const result = await invoke<string>('claude_code_prompt', {
          prompt: `${systemPrompt}\n\n${userPrompt}`,
          cwd: projectDir,
        });
        return result.trim();
      } catch (ccErr: any) {
        throw new Error('No API key found and Claude Code CLI unavailable. Add an OpenAI, Anthropic, or OpenRouter key in Settings > AI & Tools.');
      }
    },

    async _callOpenAI(apiKey: string, system: string, user: string): Promise<string> {
      const resp = await fetch('https://api.openai.com/v1/chat/completions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${apiKey}` },
        body: JSON.stringify({
          model: 'gpt-4o-mini',
          max_tokens: 120,
          temperature: 0.3,
          messages: [{ role: 'system', content: system }, { role: 'user', content: user }],
        }),
      });
      if (!resp.ok) throw new Error(`OpenAI error: ${resp.status} ${await resp.text()}`);
      const data = await resp.json();
      return (data.choices?.[0]?.message?.content || '').trim();
    },

    async _callAnthropic(apiKey: string, system: string, user: string): Promise<string> {
      const resp = await fetch('https://api.anthropic.com/v1/messages', {
        method: 'POST',
        headers: {
          'Content-Type': 'application/json',
          'x-api-key': apiKey,
          'anthropic-version': '2023-06-01',
          'anthropic-dangerous-direct-browser-access': 'true',
        },
        body: JSON.stringify({
          model: 'claude-haiku-4-5-20251001',
          max_tokens: 120,
          system,
          messages: [{ role: 'user', content: user }],
        }),
      });
      if (!resp.ok) throw new Error(`Anthropic error: ${resp.status} ${await resp.text()}`);
      const data = await resp.json();
      return (data.content?.[0]?.text || '').trim();
    },

    async _callOpenRouter(apiKey: string, system: string, user: string): Promise<string> {
      const resp = await fetch('https://openrouter.ai/api/v1/chat/completions', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json', 'Authorization': `Bearer ${apiKey}` },
        body: JSON.stringify({
          model: 'openai/gpt-4o-mini',
          max_tokens: 120,
          temperature: 0.3,
          messages: [{ role: 'system', content: system }, { role: 'user', content: user }],
        }),
      });
      if (!resp.ok) throw new Error(`OpenRouter error: ${resp.status} ${await resp.text()}`);
      const data = await resp.json();
      return (data.choices?.[0]?.message?.content || '').trim();
    },
  };
}
