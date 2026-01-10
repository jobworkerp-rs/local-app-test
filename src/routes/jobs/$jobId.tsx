import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery, useQueryClient } from "@tanstack/react-query";
import {
  type AgentJobStatus,
  ACTIVE_JOB_STATUSES,
  buildPrUrl,
} from "@/types/models";
import { jobQueries, repositoryQueries } from "@/lib/query";
import { useJobStreamText, useCancelAgent } from "@/hooks";
import { ExternalLink } from "@/components/ExternalLink";

export const Route = createFileRoute("/jobs/$jobId")({
  component: JobDetailPage,
});

const statusSteps: AgentJobStatus[] = [
  "Pending",
  "PreparingWorkspace",
  "FetchingIssue",
  "RunningAgent",
  "CreatingPR",
  "PrCreated",
  "Merged",
  "Completed",
];

const statusLabels: Record<AgentJobStatus, string> = {
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

/**
 * Extract a user-friendly error message from an unknown error value.
 */
function getErrorMessage(error: unknown): string {
  if (error instanceof Error) {
    return error.message;
  }
  if (typeof error === "object" && error !== null && "message" in error) {
    return String((error as { message: unknown }).message);
  }
  if (typeof error === "string") {
    return error;
  }
  try {
    return JSON.stringify(error);
  } catch {
    return String(error);
  }
}

function JobDetailPage() {
  const { jobId } = Route.useParams();
  const numericJobId = Number(jobId);
  const isValidJobId = Number.isSafeInteger(numericJobId) && numericJobId > 0;
  const queryClient = useQueryClient();

  const jobQuery = useQuery({
    ...jobQueries.detail(numericJobId),
    enabled: isValidJobId,
    refetchInterval: (query) => {
      if (query.state.status === "error") return false;
      const job = query.state.data;
      if (!job) return 5000;
      const isActive = ACTIVE_JOB_STATUSES.includes(job.status);
      return isActive ? 2000 : 5000;
    },
  });

  const repositoriesQuery = useQuery(repositoryQueries.list());
  const cancelMutation = useCancelAgent();

  const job = jobQuery.data;
  const repository = repositoriesQuery.data?.find((r) => r.id === job?.repository_id);
  const isActive = job ? ACTIVE_JOB_STATUSES.includes(job.status) : false;

  const { text: streamOutput, status: streamStatus, result: streamResult } = useJobStreamText(
    isActive ? numericJobId : null,
    {
      onComplete: () => {
        queryClient.invalidateQueries({ queryKey: jobQueries.detail(numericJobId).queryKey });
      },
    }
  );

  const handleCancel = async () => {
    if (!job) return;
    try {
      await cancelMutation.mutateAsync(job.jobworkerp_job_id);
      queryClient.invalidateQueries({ queryKey: jobQueries.detail(numericJobId).queryKey });
    } catch (error) {
      console.error("Failed to cancel job:", error);
    }
  };

  if (!isValidJobId) {
    return (
      <div className="container mx-auto p-8">
        <div className="flex items-center gap-4 mb-6">
          <Link to="/jobs" className="text-blue-600 dark:text-blue-400 hover:underline">
            &larr; Back to Jobs
          </Link>
        </div>
        <p className="text-red-600 dark:text-red-400">Error: Invalid job ID</p>
      </div>
    );
  }

  if (jobQuery.isLoading) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-slate-600 dark:text-slate-400">Loading job details...</p>
      </div>
    );
  }

  if (jobQuery.error || !job) {
    return (
      <div className="container mx-auto p-8">
        <div className="flex items-center gap-4 mb-6">
          <Link to="/jobs" className="text-blue-600 dark:text-blue-400 hover:underline">
            &larr; Back to Jobs
          </Link>
        </div>
        <p className="text-red-600 dark:text-red-400">
          Error: {jobQuery.error ? getErrorMessage(jobQuery.error) : "Job not found"}
        </p>
      </div>
    );
  }

  const issueUrl = repository
    ? `${repository.url}/issues/${job.issue_number}`
    : null;

  const prUrl = repository && job.pr_number
    ? buildPrUrl(repository, job.pr_number)
    : null;

  return (
    <div className="container mx-auto p-8">
      <div className="flex items-center gap-4 mb-6">
        <Link to="/jobs" className="text-blue-600 dark:text-blue-400 hover:underline">
          &larr; Back to Jobs
        </Link>
        <h1 className="text-3xl font-bold">Job #{job.id}</h1>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          <StatusProgress status={job.status} />

          <div className="border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Job Details</h2>

            <dl className="grid grid-cols-2 gap-4">
              <div>
                <dt className="text-sm text-gray-500 dark:text-gray-400">Repository</dt>
                <dd className="font-medium">
                  {repository ? (
                    <a
                      href={repository.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 dark:text-blue-400 hover:underline"
                    >
                      {repository.owner}/{repository.repo_name}
                    </a>
                  ) : (
                    <span className="text-gray-400 dark:text-gray-500">Unknown</span>
                  )}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500 dark:text-gray-400">Issue</dt>
                <dd className="font-medium">
                  {issueUrl ? (
                    <a
                      href={issueUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 dark:text-blue-400 hover:underline"
                    >
                      #{job.issue_number}
                    </a>
                  ) : (
                    `#${job.issue_number}`
                  )}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500 dark:text-gray-400">Branch</dt>
                <dd className="font-medium">
                  {job.branch_name ?? <span className="text-gray-400 dark:text-gray-500">-</span>}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500 dark:text-gray-400">Pull Request</dt>
                <dd className="font-medium">
                  {prUrl ? (
                    <a
                      href={prUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 dark:text-blue-400 hover:underline"
                    >
                      #{job.pr_number}
                    </a>
                  ) : (
                    <span className="text-gray-400 dark:text-gray-500">-</span>
                  )}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500 dark:text-gray-400">Worktree Path</dt>
                <dd className="font-medium font-mono text-sm">
                  {job.worktree_path ?? <span className="text-gray-400 dark:text-gray-500">-</span>}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500 dark:text-gray-400">Jobworkerp Job ID</dt>
                <dd className="font-medium font-mono text-sm">
                  {job.jobworkerp_job_id}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500 dark:text-gray-400">Created</dt>
                <dd className="font-medium">
                  {new Date(job.created_at).toLocaleString()}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500 dark:text-gray-400">Updated</dt>
                <dd className="font-medium">
                  {new Date(job.updated_at).toLocaleString()}
                </dd>
              </div>
            </dl>
          </div>

          {job.error_message && (
            <div className="border border-red-200 dark:border-red-800 rounded-lg p-6 bg-red-50 dark:bg-red-900/30">
              <h2 className="text-xl font-semibold mb-2 text-red-700 dark:text-red-400">Error</h2>
              <pre className="text-sm text-red-600 dark:text-red-400 whitespace-pre-wrap font-mono">
                {job.error_message}
              </pre>
            </div>
          )}

          {/* Streaming output */}
          {(isActive || streamOutput || streamResult) && (
            <div className="border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 rounded-lg p-6">
              <div className="flex items-center justify-between mb-4">
                <h2 className="text-xl font-semibold">Agent Output</h2>
                {streamStatus === "streaming" && (
                  <span className="flex items-center gap-2 text-sm text-blue-600 dark:text-blue-400">
                    <span className="w-2 h-2 bg-blue-500 rounded-full animate-pulse" />
                    Streaming...
                  </span>
                )}
              </div>

              {streamResult && (
                <div className={`mb-4 p-3 rounded ${
                  streamResult.status === "success"
                    ? "bg-green-100 dark:bg-green-900/30 border border-green-200 dark:border-green-800"
                    : streamResult.status === "no_changes"
                      ? "bg-yellow-100 dark:bg-yellow-900/30 border border-yellow-200 dark:border-yellow-800"
                      : "bg-red-100 dark:bg-red-900/30 border border-red-200 dark:border-red-800"
                }`}>
                  <p className="font-medium">
                    {streamResult.status === "success" && "Completed successfully!"}
                    {streamResult.status === "no_changes" && "No changes were made."}
                    {streamResult.status === "failed" && "Job failed."}
                  </p>
                  {streamResult.pr_url && (
                    <p className="mt-2">
                      <ExternalLink href={streamResult.pr_url} className="text-blue-600 dark:text-blue-400 hover:underline">
                        View Pull Request #{streamResult.pr_number}
                      </ExternalLink>
                    </p>
                  )}
                </div>
              )}

              <pre className="bg-slate-900 text-slate-100 rounded p-4 text-sm overflow-x-auto max-h-96 overflow-y-auto font-mono whitespace-pre-wrap">
                {streamOutput || (isActive ? "Waiting for output..." : "No output captured.")}
                {streamStatus === "streaming" && <span className="animate-pulse">▋</span>}
              </pre>
            </div>
          )}
        </div>

        <div className="space-y-6">
          <div className="border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Actions</h2>

            <div className="space-y-3">
              {issueUrl && (
                <a
                  href={issueUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="block w-full px-4 py-2 text-center border border-slate-300 dark:border-slate-600 rounded hover:bg-gray-50 dark:hover:bg-slate-700"
                >
                  View Issue
                </a>
              )}

              {prUrl && (
                <a
                  href={prUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="block w-full px-4 py-2 text-center bg-green-600 text-white rounded hover:bg-green-700"
                >
                  View Pull Request
                </a>
              )}

              {ACTIVE_JOB_STATUSES.includes(job.status) && (
                <button
                  type="button"
                  onClick={handleCancel}
                  disabled={cancelMutation.isPending}
                  className="block w-full px-4 py-2 text-center border border-red-600 dark:border-red-500 text-red-600 dark:text-red-400 rounded hover:bg-red-50 dark:hover:bg-red-900/30 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {cancelMutation.isPending ? "Cancelling..." : "Cancel Job"}
                </button>
              )}
            </div>
          </div>

          <div className="border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Status Legend</h2>
            <ul className="text-sm space-y-2">
              <li className="flex items-center gap-2">
                <span className="w-3 h-3 rounded-full bg-gray-300 dark:bg-gray-600" />
                Pending - Waiting in queue
              </li>
              <li className="flex items-center gap-2">
                <span className="w-3 h-3 rounded-full bg-blue-500" />
                In Progress - Running
              </li>
              <li className="flex items-center gap-2">
                <span className="w-3 h-3 rounded-full bg-green-500" />
                Completed - Finished
              </li>
              <li className="flex items-center gap-2">
                <span className="w-3 h-3 rounded-full bg-red-500" />
                Failed - Error occurred
              </li>
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}

interface StatusProgressProps {
  status: AgentJobStatus;
}

function StatusProgress({ status }: StatusProgressProps) {
  if (status === "Failed" || status === "Cancelled") {
    return (
      <div className={`border rounded-lg p-6 ${status === "Failed" ? "border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-900/30" : "border-gray-200 dark:border-gray-700 bg-gray-50 dark:bg-gray-800"}`}>
        <div className="flex items-center gap-3">
          <span className={`w-4 h-4 rounded-full ${status === "Failed" ? "bg-red-500" : "bg-gray-400 dark:bg-gray-500"}`} />
          <span className={`text-lg font-semibold ${status === "Failed" ? "text-red-700 dark:text-red-400" : "text-gray-700 dark:text-gray-300"}`}>
            {statusLabels[status]}
          </span>
        </div>
      </div>
    );
  }

  const currentIndex = statusSteps.indexOf(status);

  return (
    <div className="border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 rounded-lg p-6">
      <h2 className="text-lg font-semibold mb-4">Progress</h2>
      <div className="relative">
        <div className="absolute left-2 top-0 bottom-0 w-0.5 bg-gray-200 dark:bg-gray-700" />

        {statusSteps.map((step, index) => {
          let state: "completed" | "current" | "pending";
          if (index < currentIndex) {
            state = "completed";
          } else if (index === currentIndex) {
            state = "current";
          } else {
            state = "pending";
          }

          return (
            <div key={step} className="relative flex items-center gap-4 pb-4 last:pb-0">
              <div
                className={`
                  w-4 h-4 rounded-full z-10
                  ${state === "completed" ? "bg-green-500" : ""}
                  ${state === "current" ? "bg-blue-500 animate-pulse" : ""}
                  ${state === "pending" ? "bg-gray-300 dark:bg-gray-600" : ""}
                `}
              />
              <span
                className={`
                  ${state === "completed" ? "text-green-700 dark:text-green-400" : ""}
                  ${state === "current" ? "text-blue-700 dark:text-blue-400 font-semibold" : ""}
                  ${state === "pending" ? "text-gray-400 dark:text-gray-500" : ""}
                `}
              >
                {statusLabels[step]}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}
