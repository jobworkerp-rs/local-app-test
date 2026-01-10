/**
 * Query key factory for TanStack Query
 *
 * Provides consistent query keys across the application.
 * Using factory pattern allows for better cache management and type safety.
 */

export const queryKeys = {
  // Connection
  connection: {
    all: ["connection"] as const,
    jobworkerp: () => [...queryKeys.connection.all, "jobworkerp"] as const,
  },

  // App Settings
  settings: {
    all: ["settings"] as const,
    app: () => [...queryKeys.settings.all, "app"] as const,
  },

  // MCP Servers
  mcpServers: {
    all: ["mcp-servers"] as const,
    list: () => [...queryKeys.mcpServers.all, "list"] as const,
    connection: (serverName: string) =>
      [...queryKeys.mcpServers.all, "connection", serverName] as const,
  },

  // Repositories
  repositories: {
    all: ["repositories"] as const,
    list: () => [...queryKeys.repositories.all, "list"] as const,
    detail: (id: number) =>
      [...queryKeys.repositories.all, "detail", id] as const,
  },

  // Issues
  issues: {
    all: ["issues"] as const,
    list: (repositoryId: number, state?: "open" | "closed" | "all") =>
      [...queryKeys.issues.all, "list", repositoryId, state ?? "open"] as const,
    detail: (repositoryId: number, issueNumber: number) =>
      [...queryKeys.issues.all, "detail", repositoryId, issueNumber] as const,
    comments: (repositoryId: number, issueNumber: number) =>
      [...queryKeys.issues.all, "comments", repositoryId, issueNumber] as const,
  },

  // Pull Requests
  pulls: {
    all: ["pulls"] as const,
    list: (repositoryId: number, state?: "open" | "closed" | "all") =>
      [...queryKeys.pulls.all, "list", repositoryId, state ?? "open"] as const,
    related: (repositoryId: number, issueNumber: number) =>
      [...queryKeys.pulls.all, "related", repositoryId, issueNumber] as const,
  },

  // Agent Jobs
  jobs: {
    all: ["agent-jobs"] as const,
    list: (repositoryId?: number | null, status?: string | null) =>
      [...queryKeys.jobs.all, "list", repositoryId ?? null, status ?? null] as const,
    detail: (id: number) => [...queryKeys.jobs.all, "detail", id] as const,
  },
} as const;
