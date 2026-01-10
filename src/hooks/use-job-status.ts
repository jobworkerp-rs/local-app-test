/**
 * Job status and management hooks
 *
 * These hooks provide reactive access to agent job data
 * with automatic polling for active jobs.
 */
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useEffect } from "react";
import {
  listJobs,
  getJob,
  startAgent,
  cancelAgent,
  type StartAgentRequest,
} from "@/lib/tauri/commands";
import { listenJobStatus } from "@/lib/tauri/events";
import type { AgentJob } from "@/types/models";

// ============================================================================
// Query Keys
// ============================================================================

export const jobKeys = {
  all: ["jobs"] as const,
  lists: () => [...jobKeys.all, "list"] as const,
  list: (status?: string) => [...jobKeys.lists(), { status }] as const,
  details: () => [...jobKeys.all, "detail"] as const,
  detail: (id: number) => [...jobKeys.details(), id] as const,
};

// ============================================================================
// Job List Hooks
// ============================================================================

/**
 * Fetch all jobs, optionally filtered by status
 */
export function useJobs(status?: string) {
  return useQuery({
    queryKey: jobKeys.list(status),
    queryFn: () => listJobs(status),
    staleTime: 10_000,
  });
}

/**
 * Fetch jobs with automatic polling for active jobs
 *
 * @param status - Optional status filter
 * @param pollInterval - Polling interval in ms when active jobs exist (default: 3000)
 */
export function useJobsWithPolling(status?: string, pollInterval = 3000) {
  const query = useQuery({
    queryKey: jobKeys.list(status),
    queryFn: () => listJobs(status),
    staleTime: 5_000,
  });

  const hasActiveJobs =
    query.data?.some((job) => isActiveJob(job.status)) ?? false;

  return useQuery({
    queryKey: jobKeys.list(status),
    queryFn: () => listJobs(status),
    refetchInterval: hasActiveJobs ? pollInterval : false,
    staleTime: 5_000,
  });
}

// ============================================================================
// Single Job Hooks
// ============================================================================

/**
 * Fetch a single job by ID
 */
export function useJob(jobId: number | undefined) {
  return useQuery({
    queryKey: jobKeys.detail(jobId ?? 0),
    queryFn: () => getJob(jobId!),
    enabled: jobId !== undefined && jobId > 0,
    staleTime: 5_000,
  });
}

/**
 * Fetch a single job with automatic polling while active
 */
export function useJobWithPolling(
  jobId: number | undefined,
  pollInterval = 2000
) {
  const query = useQuery({
    queryKey: jobKeys.detail(jobId ?? 0),
    queryFn: () => getJob(jobId!),
    enabled: jobId !== undefined && jobId > 0,
    staleTime: 2_000,
  });

  const isActive = query.data ? isActiveJob(query.data.status) : false;

  return useQuery({
    queryKey: jobKeys.detail(jobId ?? 0),
    queryFn: () => getJob(jobId!),
    enabled: jobId !== undefined && jobId > 0,
    refetchInterval: isActive ? pollInterval : false,
    staleTime: 2_000,
  });
}

/**
 * Subscribe to real-time job status updates via Tauri events
 *
 * This hook listens to job status change events and automatically
 * invalidates the query cache when status changes.
 */
export function useJobStatusSubscription(jobId: number | undefined) {
  const queryClient = useQueryClient();

  useEffect(() => {
    if (!jobId || jobId <= 0) return;

    let unlisten: (() => void) | undefined;

    listenJobStatus(jobId, (newStatus) => {
      queryClient.setQueryData<AgentJob>(jobKeys.detail(jobId), (old) => {
        if (!old) return old;
        return { ...old, status: newStatus as AgentJob["status"] };
      });
      queryClient.invalidateQueries({ queryKey: jobKeys.lists() });
    })
      .then((fn) => {
        unlisten = fn;
      })
      .catch((err) => {
        console.error("Failed to subscribe to job status:", err);
      });

    return () => {
      unlisten?.();
    };
  }, [jobId, queryClient]);
}

// ============================================================================
// Job Mutation Hooks
// ============================================================================

/**
 * Start an agent job for an issue
 */
export function useStartAgent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: StartAgentRequest) => startAgent(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: jobKeys.lists() });
    },
  });
}

/**
 * Cancel a running agent job
 */
export function useCancelAgent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (jobworkerpJobId: string) => cancelAgent(jobworkerpJobId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: jobKeys.lists() });
    },
  });
}

// ============================================================================
// Utility Functions
// ============================================================================

const ACTIVE_STATUSES = [
  "Pending",
  "PreparingWorkspace",
  "FetchingIssue",
  "RunningAgent",
  "CreatingPR",
];

/**
 * Check if a job status indicates an active/running job
 */
export function isActiveJob(status: AgentJob["status"]): boolean {
  return ACTIVE_STATUSES.includes(status);
}

/**
 * Get a human-readable label for job status
 */
export function getJobStatusLabel(status: AgentJob["status"]): string {
  const labels: Record<AgentJob["status"], string> = {
    Pending: "Pending",
    PreparingWorkspace: "Preparing Workspace",
    FetchingIssue: "Fetching Issue",
    RunningAgent: "Running Agent",
    CreatingPR: "Creating PR",
    PrCreated: "PR Created",
    Merged: "Merged",
    Completed: "Completed",
    Failed: "Failed",
    Cancelled: "Cancelled",
  };
  return labels[status] ?? status;
}

/**
 * Get status color for UI display
 */
export function getJobStatusColor(
  status: AgentJob["status"]
): "default" | "primary" | "success" | "warning" | "danger" {
  switch (status) {
    case "Pending":
    case "PreparingWorkspace":
    case "FetchingIssue":
      return "default";
    case "RunningAgent":
    case "CreatingPR":
      return "primary";
    case "PrCreated":
    case "Completed":
    case "Merged":
      return "success";
    case "Failed":
      return "danger";
    case "Cancelled":
      return "warning";
    default:
      return "default";
  }
}
