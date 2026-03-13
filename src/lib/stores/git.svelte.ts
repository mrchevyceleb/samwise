/** Git store using Svelte 5 runes */

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
  };
}
