import { getSettings, type MCPServerConfig } from '$lib/stores/settings';
import { mcpListTools, stdioMcpSpawn, stdioMcpListTools, stdioMcpStop, stdioMcpStatus } from '$lib/utils/tauri';
import type { ToolDefinition } from '../types';
import { setDynamicToolDefinitions } from './tool-definitions';

interface McpToolMetadata {
  server: MCPServerConfig;
  originalName: string;
  transport: 'http' | 'stdio';
}

const mcpToolMap = new Map<string, McpToolMetadata>();

function normalizeToolName(serverId: string, toolName: string): string {
  const safeServer = serverId.replace(/[^a-zA-Z0-9_]/g, '_');
  const safeTool = toolName.replace(/[^a-zA-Z0-9_]/g, '_');
  return `mcp__${safeServer}__${safeTool}`;
}

function parseTools(payload: string): Array<{ name: string; description?: string; inputSchema?: unknown; input_schema?: unknown }> {
  try {
    const parsed = JSON.parse(payload);
    if (Array.isArray(parsed)) return parsed;
    if (Array.isArray(parsed.tools)) return parsed.tools;
  } catch {
    return [];
  }
  return [];
}

export function getMcpToolMetadata(normalizedName: string): McpToolMetadata | undefined {
  return mcpToolMap.get(normalizedName);
}

function registerToolsFromPayload(
  raw: string,
  server: MCPServerConfig,
  transport: 'http' | 'stdio',
  dynamicDefinitions: ToolDefinition[],
) {
  const tools = parseTools(raw);
  for (const tool of tools) {
    if (!tool?.name) continue;
    const normalized = normalizeToolName(server.id, tool.name);
    const schema = (tool.inputSchema || tool.input_schema || {
      type: 'object',
      properties: {},
    }) as { type: 'object'; properties: Record<string, unknown>; required?: string[] };

    dynamicDefinitions.push({
      type: 'function',
      function: {
        name: normalized,
        description: `[MCP:${server.name}] ${tool.description || tool.name}`,
        parameters: schema,
      },
    });

    mcpToolMap.set(normalized, {
      server,
      originalName: tool.name,
      transport,
    });
  }
}

async function ensureStdioServerRunning(server: MCPServerConfig): Promise<void> {
  try {
    const status = await stdioMcpStatus(server.id);
    if (status === 'running') return;
    await stdioMcpStop(server.id);
  } catch {
    try { await stdioMcpStop(server.id); } catch { /* ignore */ }
  }

  await stdioMcpSpawn(
    server.id,
    server.command || '',
    server.args || [],
    server.env || {},
  );
}

async function shutdownDisabledStdioServers(enabledIds: Set<string>): Promise<void> {
  const conf = getSettings();
  for (const server of conf.mcpServers) {
    if (server.transport === 'stdio' && !enabledIds.has(server.id)) {
      try {
        await stdioMcpStop(server.id);
      } catch {
        // Ignore errors on cleanup
      }
    }
  }
}

export async function refreshMcpToolDefinitions(): Promise<ToolDefinition[]> {
  const conf = getSettings();
  const enabledServers = conf.mcpServers.filter((s) => s.enabled);

  const dynamicDefinitions: ToolDefinition[] = [];
  mcpToolMap.clear();

  const enabledStdioIds = new Set<string>();

  for (const server of enabledServers) {
    const transport = server.transport || 'http';

    try {
      if (transport === 'stdio') {
        if (!server.command?.trim()) continue;
        enabledStdioIds.add(server.id);
        await ensureStdioServerRunning(server);
        const raw = await stdioMcpListTools(server.id);
        registerToolsFromPayload(raw, server, 'stdio', dynamicDefinitions);
      } else {
        if (!server.url?.trim()) continue;
        const raw = await mcpListTools(
          server.url.trim(),
          server.authToken || undefined,
          server.headersJson || undefined,
          server.timeoutMs,
        );
        registerToolsFromPayload(raw, server, 'http', dynamicDefinitions);
      }
    } catch {
      // Keep going so one broken server does not remove all tools.
    }
  }

  await shutdownDisabledStdioServers(enabledStdioIds);

  setDynamicToolDefinitions(dynamicDefinitions);
  return dynamicDefinitions;
}
