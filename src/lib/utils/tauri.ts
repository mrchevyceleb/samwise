import { invoke } from '@tauri-apps/api/core';

/** Typed wrapper for Tauri invoke calls */

export interface FileEntry {
	name: string;
	path: string;
	is_dir: boolean;
	children?: FileEntry[];
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
	modified: string;
}

export interface FlatFileEntry {
	relative_path: string;
	is_dir: boolean;
}

export interface CommandResult {
	stdout: string;
	stderr: string;
	exit_code: number;
}

// ── File Operations ────────────────────────────────────────────────

export async function readDirectoryTree(path: string): Promise<FileEntry[]> {
	return invoke<FileEntry[]>('read_directory_tree', { path });
}

export async function readDirectoryChildren(path: string): Promise<FileEntry[]> {
	return invoke<FileEntry[]>('read_directory_children', { path });
}

export async function readFileText(path: string): Promise<string> {
	return invoke<string>('read_file_text', { path });
}

export async function writeFileText(path: string, content: string): Promise<void> {
	return invoke<void>('write_file_text', { path, content });
}

export async function createFile(path: string, isDirectory: boolean): Promise<void> {
	return invoke<void>('create_file', { path, isDirectory });
}

export async function deletePath(path: string): Promise<void> {
	return invoke<void>('delete_path', { path });
}

export async function renamePath(oldPath: string, newPath: string): Promise<void> {
	return invoke<void>('rename_path', { oldPath, newPath });
}

export async function searchFiles(path: string, query: string, caseSensitive: boolean): Promise<SearchResult[]> {
	return invoke<SearchResult[]>('search_files', { path, query, caseSensitive });
}

export async function getFileInfo(path: string): Promise<FileInfo> {
	return invoke<FileInfo>('get_file_info', { path });
}

export async function listAllFiles(path: string): Promise<FlatFileEntry[]> {
	return invoke<FlatFileEntry[]>('list_all_files', { path });
}

// ── Command Execution ──────────────────────────────────────────────

export async function runCommandSync(
	command: string,
	cwd: string,
	timeoutMs?: number,
): Promise<CommandResult> {
	return invoke<CommandResult>('run_command_sync', { command, cwd, timeoutMs });
}

// ── AI Streaming ───────────────────────────────────────────────────

export async function aiChatStream(
	requestId: string,
	baseUrl: string,
	apiKey: string,
	bodyJson: string,
): Promise<void> {
	return invoke<void>('ai_chat_stream', { requestId, baseUrl, apiKey, bodyJson });
}

export async function aiChatStreamAnthropic(
	requestId: string,
	baseUrl: string,
	apiKey: string,
	bodyJson: string,
): Promise<void> {
	return invoke<void>('ai_chat_stream_anthropic', { requestId, baseUrl, apiKey, bodyJson });
}

export async function aiChatStreamOpenAICodex(
	requestId: string,
	baseUrl: string,
	accessToken: string,
	bodyJson: string,
	clientVersion: string,
): Promise<void> {
	return invoke<void>('ai_chat_stream_openai_codex', {
		requestId, baseUrl, accessToken, bodyJson, clientVersion,
	});
}

export async function aiFetchModels(baseUrl: string, apiKey: string): Promise<string> {
	return invoke<string>('ai_fetch_models', { baseUrl, apiKey });
}

// ── AI OAuth ───────────────────────────────────────────────────────

export async function aiExchangeOpenRouterOAuthCode(code: string, codeVerifier: string): Promise<string> {
	return invoke<string>('ai_exchange_openrouter_oauth_code', { code, codeVerifier });
}

export async function aiOpenAIDeviceStart(issuer: string, clientId: string): Promise<string> {
	return invoke<string>('ai_openai_device_start', { issuer, clientId });
}

export async function aiOpenAIDevicePoll(issuer: string, deviceAuthId: string, userCode: string): Promise<string> {
	return invoke<string>('ai_openai_device_poll', { issuer, deviceAuthId, userCode });
}

export async function aiOpenAIExchangeAuthorizationCode(
	issuer: string,
	clientId: string,
	authorizationCode: string,
	codeVerifier: string,
	redirectUri: string,
): Promise<string> {
	return invoke<string>('ai_openai_exchange_authorization_code', {
		issuer, clientId, authorizationCode, codeVerifier, redirectUri,
	});
}

export async function aiOpenAIRefreshOAuthToken(
	issuer: string,
	clientId: string,
	refreshToken: string,
): Promise<string> {
	return invoke<string>('ai_openai_refresh_oauth_token', { issuer, clientId, refreshToken });
}

// ── Chat Session Persistence ───────────────────────────────────────

export async function saveChatSession(sessionId: string, data: string): Promise<void> {
	return invoke<void>('save_chat_session', { sessionId, data });
}

export async function loadChatSession(sessionId: string): Promise<string> {
	return invoke<string>('load_chat_session', { sessionId });
}

export async function listChatSessions(): Promise<string[]> {
	return invoke<string[]>('list_chat_sessions');
}

export async function deleteChatSession(sessionId: string): Promise<void> {
	return invoke<void>('delete_chat_session', { sessionId });
}

// ── MCP (HTTP Transport) ──────────────────────────────────────────

export async function mcpListTools(
	serverUrl: string,
	authToken?: string,
	headersJson?: string,
	timeoutMs?: number,
): Promise<string> {
	return invoke<string>('mcp_list_tools', { serverUrl, authToken, headersJson, timeoutMs });
}

export async function mcpCallTool(
	serverUrl: string,
	toolName: string,
	argumentsJson: string,
	authToken?: string,
	headersJson?: string,
	timeoutMs?: number,
): Promise<string> {
	return invoke<string>('mcp_call_tool', {
		serverUrl, toolName, argumentsJson, authToken, headersJson, timeoutMs,
	});
}

// ── MCP (Stdio Transport) ─────────────────────────────────────────

export async function stdioMcpSpawn(
	serverId: string,
	command: string,
	args: string[],
	env: Record<string, string>,
): Promise<void> {
	return invoke<void>('stdio_mcp_spawn', { serverId, command, args, env });
}

export async function stdioMcpStop(serverId: string): Promise<void> {
	return invoke<void>('stdio_mcp_stop', { serverId });
}

export async function stdioMcpListTools(serverId: string): Promise<string> {
	return invoke<string>('stdio_mcp_list_tools', { serverId });
}

export async function stdioMcpCallTool(
	serverId: string,
	toolName: string,
	argumentsJson: string,
): Promise<string> {
	return invoke<string>('stdio_mcp_call_tool', { serverId, toolName, argumentsJson });
}

export async function stdioMcpStatus(serverId: string): Promise<string> {
	return invoke<string>('stdio_mcp_status', { serverId });
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
