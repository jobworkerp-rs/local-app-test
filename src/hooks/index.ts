/**
 * React hooks for Local Code Agent
 *
 * Re-exports all hooks for convenient imports.
 */

// Job streaming hooks
export {
  useJobStream,
  useJobStreamText,
  type StreamStatus,
  type WorkflowResult,
} from "./use-job-stream";

// Repository and issue hooks
export {
  useRepositories,
  useRepository,
  useCreateRepository,
  useDeleteRepository,
  useIssues,
  useIssue,
  usePullRequests,
  useRelatedPullRequests,
  repositoryKeys,
} from "./use-repository";

// Job status and management hooks
export {
  useJobs,
  useJobsWithPolling,
  useJob,
  useJobWithPolling,
  useJobStatusSubscription,
  useStartAgent,
  useCancelAgent,
  isActiveJob,
  getJobStatusLabel,
  getJobStatusColor,
  jobKeys,
} from "./use-job-status";

// Settings hooks
export {
  useAppSettings,
  useUpdateAppSettings,
  useJobworkerpConnection,
  useJobworkerpConnectionWithPolling,
  settingsKeys,
} from "./use-settings";

// MCP server hooks
export {
  useMcpServers,
  useMcpConnection,
  useCreateMcpRunner,
  mcpKeys,
} from "./use-mcp";
