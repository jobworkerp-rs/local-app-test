/**
 * Query options factory for TanStack Query
 *
 * Provides reusable query options for all API calls.
 * Uses the typed command wrappers from lib/tauri/commands.ts.
 */
import { queryOptions } from "@tanstack/react-query";
import {
  checkJobworkerpConnection,
  getAppSettings,
  listMcpServers,
  checkMcpConnection,
  listRepositories,
  getRepository,
  listIssues,
  getIssue,
  listPulls,
  findRelatedPrs,
  listJobs,
  getJob,
} from "@/lib/tauri/commands";
import { queryKeys } from "./keys";

// ============================================================================
// Connection Queries
// ============================================================================

export const connectionQueries = {
  jobworkerp: () =>
    queryOptions({
      queryKey: queryKeys.connection.jobworkerp(),
      queryFn: checkJobworkerpConnection,
      staleTime: 30_000,
    }),
};

// ============================================================================
// Settings Queries
// ============================================================================

export const settingsQueries = {
  app: () =>
    queryOptions({
      queryKey: queryKeys.settings.app(),
      queryFn: getAppSettings,
      staleTime: 60_000,
    }),
};

// ============================================================================
// MCP Server Queries
// ============================================================================

export const mcpServerQueries = {
  list: () =>
    queryOptions({
      queryKey: queryKeys.mcpServers.list(),
      queryFn: listMcpServers,
      staleTime: 60_000,
    }),

  connection: (serverName: string) =>
    queryOptions({
      queryKey: queryKeys.mcpServers.connection(serverName),
      queryFn: () => checkMcpConnection(serverName),
      staleTime: 30_000,
      enabled: !!serverName,
    }),
};

// ============================================================================
// Repository Queries
// ============================================================================

export const repositoryQueries = {
  list: () =>
    queryOptions({
      queryKey: queryKeys.repositories.list(),
      queryFn: listRepositories,
    }),

  detail: (id: number) =>
    queryOptions({
      queryKey: queryKeys.repositories.detail(id),
      queryFn: () => getRepository(id),
      enabled: Number.isSafeInteger(id) && id > 0,
    }),
};

// ============================================================================
// Issue Queries
// ============================================================================

export const issueQueries = {
  list: (repositoryId: number, state?: "open" | "closed" | "all") =>
    queryOptions({
      queryKey: queryKeys.issues.list(repositoryId, state),
      queryFn: () => listIssues(repositoryId, state),
      enabled: Number.isSafeInteger(repositoryId) && repositoryId > 0,
    }),

  detail: (repositoryId: number, issueNumber: number) =>
    queryOptions({
      queryKey: queryKeys.issues.detail(repositoryId, issueNumber),
      queryFn: () => getIssue(repositoryId, issueNumber),
      enabled:
        Number.isSafeInteger(repositoryId) &&
        repositoryId > 0 &&
        Number.isSafeInteger(issueNumber) &&
        issueNumber > 0,
    }),
};

// ============================================================================
// Pull Request Queries
// ============================================================================

export const pullQueries = {
  list: (repositoryId: number, state?: "open" | "closed" | "all") =>
    queryOptions({
      queryKey: queryKeys.pulls.list(repositoryId, state),
      queryFn: () => listPulls(repositoryId, state),
      enabled: Number.isSafeInteger(repositoryId) && repositoryId > 0,
    }),

  related: (repositoryId: number, issueNumber: number) =>
    queryOptions({
      queryKey: queryKeys.pulls.related(repositoryId, issueNumber),
      queryFn: () => findRelatedPrs(repositoryId, issueNumber),
      staleTime: 60_000,
      enabled:
        Number.isSafeInteger(repositoryId) &&
        repositoryId > 0 &&
        Number.isSafeInteger(issueNumber) &&
        issueNumber > 0,
    }),
};

// ============================================================================
// Agent Job Queries
// ============================================================================

export const jobQueries = {
  list: (repositoryId?: number | null, status?: string | null) =>
    queryOptions({
      queryKey: queryKeys.jobs.list(repositoryId, status),
      queryFn: () => listJobs(status ?? undefined),
      refetchInterval: 5_000,
    }),

  detail: (id: number) =>
    queryOptions({
      queryKey: queryKeys.jobs.detail(id),
      queryFn: () => getJob(id),
      enabled: Number.isSafeInteger(id) && id > 0,
      refetchInterval: 5_000,
    }),
};
