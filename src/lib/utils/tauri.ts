/** Typed wrapper for Tauri invoke calls */

// Lazy-load invoke to avoid SSR crashes (SvelteKit processes module graph server-side)
let _invoke: typeof import('@tauri-apps/api/core').invoke | null = null;

async function getInvoke() {
	if (!_invoke) {
		const mod = await import('@tauri-apps/api/core');
		_invoke = mod.invoke;
	}
	return _invoke;
}

async function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
	const fn = await getInvoke();
	return fn<T>(cmd, args);
}

/** Open a URL in the system browser. Tauri's webview traps `<a target="_blank">`
 *  and does nothing with it; this goes through the shell plugin so PR/GitHub
 *  links actually open. */
export async function openExternal(url: string): Promise<void> {
	if (!url) return;
	try {
		const { open } = await import('@tauri-apps/plugin-shell');
		await open(url);
	} catch (e) {
		console.warn('[openExternal] shell.open failed, falling back to window.open', e);
		try { window.open(url, '_blank'); } catch {}
	}
}

export interface FileNode {
	name: string;
	path: string;
	is_dir: boolean;
	size?: number;
	ext?: string;
	children?: FileNode[];
}

export interface FileEntry {
	name: string;
	path: string;
	relative_path: string;
	ext?: string;
}

export interface WorkspaceInfo {
	path: string;
	name: string;
}

export interface SearchResult {
	path: string;
	line_number: number;
	line_content: string;
}

export interface FileInfo {
	name: string;
	path: string;
	is_dir: boolean;
	size: number;
	ext?: string;
	modified?: number;
}

// ── File Operations ────────────────────────────────────────────────

export async function readDirectoryTree(path: string, showHidden: boolean = false): Promise<FileNode> {
	return invoke<FileNode>('read_directory_tree', { path, showHidden });
}

export async function readDirectoryChildren(path: string, showHidden: boolean = false): Promise<FileNode[]> {
	return invoke<FileNode[]>('read_directory_children', { path, showHidden });
}

export async function readFileText(path: string): Promise<string> {
	return invoke<string>('read_file_text', { path });
}

export async function writeFileText(path: string, content: string): Promise<void> {
	return invoke<void>('write_file_text', { path, content });
}

export async function createFile(path: string, isDir: boolean): Promise<void> {
	return invoke<void>('create_file', { path, isDir });
}

export async function deletePath(path: string): Promise<void> {
	return invoke<void>('delete_path', { path });
}

export async function renamePath(oldPath: string, newPath: string): Promise<void> {
	return invoke<void>('rename_path', { oldPath, newPath });
}

export async function searchFiles(root: string, query: string, caseSensitive: boolean, showHidden: boolean = false): Promise<SearchResult[]> {
	return invoke<SearchResult[]>('search_files', { root, query, caseSensitive, showHidden });
}

export async function getFileInfo(path: string): Promise<FileInfo> {
	return invoke<FileInfo>('get_file_info', { path });
}

export async function listAllFiles(root: string, showHidden: boolean = false): Promise<FileEntry[]> {
	return invoke<FileEntry[]>('list_all_files', { root, showHidden });
}

// ── Claude Code ───────────────────────────────────────────────────

export async function spawnClaudeCode(
	id: string,
	cwd: string,
	args: string[] = [],
): Promise<void> {
	return invoke<void>('spawn_claude_code', { id, cwd, args });
}

export async function closeClaudeCode(id: string): Promise<void> {
	return invoke<void>('close_claude_code', { id });
}

export async function writeClaudeCode(
	id: string,
	message: string,
): Promise<void> {
	return invoke<void>('write_claude_code', { id, message });
}

// ── Settings Persistence ──────────────────────────────────────────

export async function saveSettings(data: string): Promise<void> {
	return invoke<void>('save_settings', { data });
}

export async function loadSettings(): Promise<string> {
	return invoke<string>('load_settings');
}

/** Safe invoke that catches errors when Tauri is not available (e.g. browser dev) */
export async function safeInvoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T | null> {
	try {
		return await invoke<T>(cmd, args);
	} catch {
		console.warn(`Tauri command "${cmd}" not available`);
		return null;
	}
}
