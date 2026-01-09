export type AgentJobStatus =
  | "Pending"
  | "PreparingWorkspace"
  | "FetchingIssue"
  | "RunningAgent"
  | "CreatingPR"
  | "PrCreated"
  | "Merged"
  | "Completed"
  | "Failed"
  | "Cancelled";

/**
 * Statuses that indicate a job is actively running and should be polled.
 */
export const ACTIVE_JOB_STATUSES: AgentJobStatus[] = [
  "Pending",
  "PreparingWorkspace",
  "FetchingIssue",
  "RunningAgent",
  "CreatingPR",
];

export interface AgentJob {
  id: number;
  repository_id: number;
  issue_number: number;
  jobworkerp_job_id: string;
  status: AgentJobStatus;
  worktree_path: string | null;
  branch_name: string | null;
  pr_number: number | null;
  error_message: string | null;
  created_at: string;
  updated_at: string;
}

export interface Repository {
  id: number;
  mcp_server_name: string;
  platform: "GitHub" | "Gitea";
  base_url: string;
  name: string;
  url: string;
  owner: string;
  repo_name: string;
  local_path: string | null;
  last_synced_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateRepositoryRequest {
  mcp_server_name: string;
  platform: "GitHub" | "Gitea";
  base_url: string;
  name: string;
  url: string;
  owner: string;
  repo_name: string;
  local_path: string | null;
}

export interface McpServerInfo {
  name: string;
  description: string | null;
  runner_type: string;
}

/**
 * Build a repository URL from Gitea API base URL.
 * Handles various URL formats:
 * - https://gitea.example.com/api/v1
 * - https://gitea.example.com/api/v1/
 * - https://gitea.example.com/custom/path/api/v1
 * - https://gitea.example.com (no API path)
 */
export function buildGiteaRepoUrl(baseUrl: string, owner: string, repoName: string): string {
  try {
    const url = new URL(baseUrl);
    let pathname = url.pathname;

    // Remove /api/v1 suffix if present
    if (pathname.endsWith("/api/v1")) {
      pathname = pathname.slice(0, -7);
    } else if (pathname.endsWith("/api/v1/")) {
      pathname = pathname.slice(0, -8);
    }

    // Ensure pathname doesn't end with slash for clean concatenation
    if (pathname.endsWith("/")) {
      pathname = pathname.slice(0, -1);
    }

    return `${url.origin}${pathname}/${owner}/${repoName}`;
  } catch {
    // Fallback: naive replacement if URL parsing fails
    return `${baseUrl.replace(/\/api\/v1\/?$/, "")}/${owner}/${repoName}`;
  }
}

/**
 * Get the PR path segment for a given platform.
 * GitHub uses /pull/{number}, Gitea uses /pulls/{number}
 */
export function getPrPath(platform: "GitHub" | "Gitea"): string {
  return platform === "Gitea" ? "pulls" : "pull";
}

/**
 * Build a PR URL for the given repository and PR number.
 */
export function buildPrUrl(repository: Repository, prNumber: number): string {
  const prPath = getPrPath(repository.platform);
  return `${repository.url}/${prPath}/${prNumber}`;
}

/**
 * Issue from GitHub/Gitea
 */
export interface Issue {
  number: number;
  title: string;
  body: string | null;
  state: string;
  labels: string[];
  user: string;
  html_url: string;
  created_at: string;
  updated_at: string;
}

/**
 * Pull Request from GitHub/Gitea
 */
export interface PullRequest {
  number: number;
  title: string;
  body: string | null;
  state: string;
  head_branch: string;
  base_branch: string;
  html_url: string;
  merged: boolean;
  created_at: string;
  updated_at: string;
}

/**
 * Build the web-facing base URL from a Gitea API base URL.
 */
export function getGiteaWebBaseUrl(baseUrl: string): string {
  try {
    const url = new URL(baseUrl);
    let pathname = url.pathname;

    // Remove /api/v1 suffix if present
    if (pathname.endsWith("/api/v1")) {
      pathname = pathname.slice(0, -7);
    } else if (pathname.endsWith("/api/v1/")) {
      pathname = pathname.slice(0, -8);
    }

    // Remove trailing slash
    if (pathname.endsWith("/") && pathname.length > 1) {
      pathname = pathname.slice(0, -1);
    }

    return `${url.origin}${pathname}`;
  } catch {
    // Fallback
    return baseUrl.replace(/\/api\/v1\/?$/, "");
  }
}
