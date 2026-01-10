/**
 * Type-safe Tauri command wrappers
 *
 * This module provides typed wrapper functions for all Tauri commands,
 * ensuring type safety between the frontend and Rust backend.
 */
import { invoke } from "@tauri-apps/api/core";
import type {
  Repository,
  CreateRepositoryRequest,
  McpServerInfo,
  Issue,
  PullRequest,
  AgentJob,
} from "@/types/models";

// ============================================================================
// App Settings Types
// ============================================================================

export interface AppSettings {
  id: number;
  worktree_base_path: string;
  default_base_branch: string;
  agent_timeout_minutes: number;
  sync_interval_minutes: number;
  grpc_server_url: string;
  locale: string;
  created_at: string;
  updated_at: string;
}

export interface UpdateAppSettingsRequest {
  worktree_base_path?: string;
  default_base_branch?: string;
  agent_timeout_minutes?: number;
  sync_interval_minutes?: number;
  grpc_server_url?: string;
  locale?: string;
}

// ============================================================================
// Connection Commands
// ============================================================================

/**
 * Check connection to jobworkerp-rs backend
 */
export function checkJobworkerpConnection(): Promise<boolean> {
  return invoke<boolean>("check_jobworkerp_connection");
}

// ============================================================================
// Settings Commands
// ============================================================================

/**
 * Get application settings
 */
export function getAppSettings(): Promise<AppSettings> {
  return invoke<AppSettings>("get_app_settings");
}

/**
 * Update application settings
 */
export function updateAppSettings(
  settings: UpdateAppSettingsRequest
): Promise<AppSettings> {
  return invoke<AppSettings>("update_app_settings", { settings });
}

// ============================================================================
// MCP Server Commands
// ============================================================================

/**
 * List all configured MCP servers
 */
export function listMcpServers(): Promise<McpServerInfo[]> {
  return invoke<McpServerInfo[]>("mcp_list_servers");
}

/**
 * Check if an MCP server is connected
 */
export function checkMcpConnection(serverName: string): Promise<boolean> {
  return invoke<boolean>("mcp_check_connection", { serverName });
}

/**
 * Create a new MCP server (Runner) dynamically
 */
export function createMcpRunner(
  platform: "GitHub" | "Gitea",
  name: string,
  url: string,
  token: string
): Promise<McpServerInfo> {
  return invoke<McpServerInfo>("mcp_create_runner", {
    platform,
    name,
    url,
    token,
  });
}

// ============================================================================
// Repository Commands
// ============================================================================

/**
 * List all registered repositories
 */
export function listRepositories(): Promise<Repository[]> {
  return invoke<Repository[]>("list_repositories");
}

/**
 * Get a single repository by ID
 */
export function getRepository(repositoryId: number): Promise<Repository> {
  return invoke<Repository>("get_repository", { repositoryId });
}

/**
 * Create a new repository
 */
export function createRepository(
  request: CreateRepositoryRequest
): Promise<Repository> {
  return invoke<Repository>("create_repository", { request });
}

/**
 * Delete a repository by ID
 */
export function deleteRepository(id: number): Promise<void> {
  return invoke<void>("delete_repository", { id });
}

// ============================================================================
// Issue Commands
// ============================================================================

/**
 * List issues for a repository
 */
export function listIssues(
  repositoryId: number,
  state?: "open" | "closed" | "all"
): Promise<Issue[]> {
  return invoke<Issue[]>("list_issues", {
    repositoryId,
    state: state ?? "open",
  });
}

/**
 * Get a single issue by number
 */
export function getIssue(
  repositoryId: number,
  issueNumber: number
): Promise<Issue> {
  return invoke<Issue>("get_issue", {
    repositoryId,
    issueNumber,
  });
}

/**
 * Issue comment from GitHub/Gitea
 */
export interface IssueComment {
  id: number;
  user: string;
  body: string;
  created_at: string;
  updated_at: string;
}

/**
 * Get comments for a specific issue
 */
export function getIssueComments(
  repositoryId: number,
  issueNumber: number
): Promise<IssueComment[]> {
  return invoke<IssueComment[]>("get_issue_comments", {
    repositoryId,
    issueNumber,
  });
}

// ============================================================================
// Pull Request Commands
// ============================================================================

/**
 * List pull requests for a repository
 */
export function listPulls(
  repositoryId: number,
  state?: "open" | "closed" | "all"
): Promise<PullRequest[]> {
  return invoke<PullRequest[]>("list_pulls", {
    repositoryId,
    state: state ?? "open",
  });
}

/**
 * Find pull requests related to a specific issue
 */
export function findRelatedPrs(
  repositoryId: number,
  issueNumber: number
): Promise<PullRequest[]> {
  return invoke<PullRequest[]>("find_related_prs", {
    repositoryId,
    issueNumber,
  });
}

// ============================================================================
// Job Commands
// ============================================================================

/**
 * List all agent jobs, optionally filtered by status
 */
export function listJobs(status?: string): Promise<AgentJob[]> {
  return invoke<AgentJob[]>("list_jobs", { status });
}

/**
 * Get a single job by ID
 */
export function getJob(id: number): Promise<AgentJob> {
  return invoke<AgentJob>("get_job", { id });
}

// ============================================================================
// Agent Commands
// ============================================================================

export interface StartAgentRequest {
  repository_id: number;
  issue_number: number;
  issue_title: string;
  custom_prompt?: string;
}

export interface StartAgentResponse {
  job_id: number;
  jobworkerp_job_id: string;
}

/**
 * Start an agent to process an issue
 */
export function startAgent(
  request: StartAgentRequest
): Promise<StartAgentResponse> {
  return invoke<StartAgentResponse>("agent_start", { request });
}

/**
 * Cancel a running agent job
 */
export function cancelAgent(jobworkerpJobId: string): Promise<void> {
  return invoke<void>("agent_cancel", { jobworkerpJobId });
}
