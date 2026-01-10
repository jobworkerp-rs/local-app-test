/**
 * MCP Server related React Query hooks
 *
 * These hooks provide access to MCP server management
 * including listing, creating, and checking connections.
 */
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  listMcpServers,
  checkMcpConnection,
  createMcpRunner,
} from "@/lib/tauri/commands";

// ============================================================================
// Query Keys
// ============================================================================

export const mcpKeys = {
  all: ["mcp"] as const,
  servers: () => [...mcpKeys.all, "servers"] as const,
  connection: (serverName: string) =>
    [...mcpKeys.all, "connection", serverName] as const,
};

// ============================================================================
// MCP Server Hooks
// ============================================================================

/**
 * Fetch all configured MCP servers
 */
export function useMcpServers() {
  return useQuery({
    queryKey: mcpKeys.servers(),
    queryFn: listMcpServers,
    staleTime: 60_000,
  });
}

/**
 * Check if a specific MCP server is connected
 */
export function useMcpConnection(serverName: string | undefined) {
  return useQuery({
    queryKey: mcpKeys.connection(serverName ?? ""),
    queryFn: () => checkMcpConnection(serverName!),
    enabled: !!serverName,
    staleTime: 30_000,
    retry: 1,
  });
}

/**
 * Create a new MCP server (Runner) dynamically
 */
export function useCreateMcpRunner() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: ({
      platform,
      name,
      url,
      token,
    }: {
      platform: "GitHub" | "Gitea";
      name: string;
      url: string;
      token: string;
    }) => createMcpRunner(platform, name, url, token),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: mcpKeys.servers() });
    },
  });
}
