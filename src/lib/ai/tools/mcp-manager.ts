import { getSettings, updateSetting, type MCPServerConfig } from '$lib/stores/settings';
import { stdioMcpStop } from '$lib/utils/tauri';
import { refreshMcpToolDefinitions } from './mcp-registry';

export async function manageMcpServers(args: Record<string, unknown>): Promise<string> {
  const action = args.action as string;
  const currentSettings = getSettings();
  const servers = currentSettings.mcpServers;

  switch (action) {
    case 'list': {
      if (servers.length === 0) return 'No MCP servers configured.';
      const list = servers.map((s) => {
        const transport = s.transport || 'http';
        const target = transport === 'stdio' ? `${s.command} ${(s.args || []).join(' ')}` : s.url;
        return `- ${s.name} [${transport}] (${s.enabled ? 'enabled' : 'disabled'}) id=${s.id}\n  ${target}`;
      });
      return list.join('\n');
    }

    case 'add': {
      const transport = (args.transport as string) || 'stdio';
      const name = (args.name as string) || 'Unnamed MCP Server';
      const id = `mcp-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;

      const newServer: MCPServerConfig = {
        id,
        name,
        enabled: true,
        timeoutMs: 30000,
        transport: transport as 'http' | 'stdio',
        url: (args.url as string) || '',
        authToken: '',
        headersJson: '{}',
        command: (args.command as string) || '',
        args: (args.args as string[]) || [],
        env: {},
      };

      updateSetting('mcpServers', [...servers, newServer]);

      try {
        const tools = await refreshMcpToolDefinitions();
        const serverTools = tools.filter((t) =>
          t.function.name.startsWith(`mcp__${id.replace(/[^a-zA-Z0-9_]/g, '_')}__`),
        );
        return `Added MCP server "${name}" (${transport}, id=${id}). ${serverTools.length} tools available.`;
      } catch {
        return `Added MCP server "${name}" (${transport}, id=${id}). Tools will load on next message.`;
      }
    }

    case 'remove': {
      const serverId = args.server_id as string;
      if (!serverId) return 'Error: server_id is required for remove action.';

      const server = servers.find((s) => s.id === serverId);
      if (!server) return `No MCP server found with id: ${serverId}`;

      if ((server.transport || 'http') === 'stdio') {
        try {
          await stdioMcpStop(serverId);
        } catch { /* ignore */ }
      }

      updateSetting('mcpServers', servers.filter((s) => s.id !== serverId));
      await refreshMcpToolDefinitions();
      return `Removed MCP server "${server.name}" (${serverId}).`;
    }

    case 'enable': {
      const serverId = args.server_id as string;
      if (!serverId) return 'Error: server_id is required for enable action.';

      const server = servers.find((s) => s.id === serverId);
      if (!server) return `No MCP server found with id: ${serverId}`;

      updateSetting(
        'mcpServers',
        servers.map((s) => (s.id === serverId ? { ...s, enabled: true } : s)),
      );
      await refreshMcpToolDefinitions();
      return `Enabled MCP server "${server.name}".`;
    }

    case 'disable': {
      const serverId = args.server_id as string;
      if (!serverId) return 'Error: server_id is required for disable action.';

      const server = servers.find((s) => s.id === serverId);
      if (!server) return `No MCP server found with id: ${serverId}`;

      if ((server.transport || 'http') === 'stdio') {
        try {
          await stdioMcpStop(serverId);
        } catch { /* ignore */ }
      }

      updateSetting(
        'mcpServers',
        servers.map((s) => (s.id === serverId ? { ...s, enabled: false } : s)),
      );
      await refreshMcpToolDefinitions();
      return `Disabled MCP server "${server.name}".`;
    }

    default:
      return `Unknown action: ${action}. Valid actions: list, add, remove, enable, disable.`;
  }
}
