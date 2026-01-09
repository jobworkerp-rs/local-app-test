import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { type AgentJobStatus, type AgentJob, type Repository } from "@/types/models";

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

function JobDetailPage() {
  const { jobId } = Route.useParams();

  const jobQuery = useQuery({
    queryKey: ["agent-job", jobId],
    queryFn: () => invoke<AgentJob>("get_job", { id: Number(jobId) }),
    refetchInterval: (query) => {
      const job = query.state.data;
      if (!job) return 5000;
      const isActive = ["Pending", "PreparingWorkspace", "FetchingIssue", "RunningAgent", "CreatingPR"].includes(job.status);
      return isActive ? 2000 : false;
    },
  });

  const repositoriesQuery = useQuery({
    queryKey: ["repositories"],
    queryFn: () => invoke<Repository[]>("list_repositories"),
  });

  const job = jobQuery.data;
  const repository = repositoriesQuery.data?.find((r) => r.id === job?.repository_id);

  if (jobQuery.isLoading) {
    return (
      <div className="container mx-auto p-8">
        <p>Loading job details...</p>
      </div>
    );
  }

  if (jobQuery.error || !job) {
    return (
      <div className="container mx-auto p-8">
        <div className="flex items-center gap-4 mb-6">
          <Link to="/jobs" className="text-blue-600 hover:underline">
            &larr; Back to Jobs
          </Link>
        </div>
        <p className="text-red-600">
          Error: {jobQuery.error ? String(jobQuery.error) : "Job not found"}
        </p>
      </div>
    );
  }

  const issueUrl = repository
    ? `${repository.url}/issues/${job.issue_number}`
    : null;

  const prUrl = repository && job.pr_number
    ? `${repository.url}/pull/${job.pr_number}`
    : null;

  return (
    <div className="container mx-auto p-8">
      <div className="flex items-center gap-4 mb-6">
        <Link to="/jobs" className="text-blue-600 hover:underline">
          &larr; Back to Jobs
        </Link>
        <h1 className="text-3xl font-bold">Job #{job.id}</h1>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          <StatusProgress status={job.status} />

          <div className="border rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Job Details</h2>

            <dl className="grid grid-cols-2 gap-4">
              <div>
                <dt className="text-sm text-gray-500">Repository</dt>
                <dd className="font-medium">
                  {repository ? (
                    <a
                      href={repository.url}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:underline"
                    >
                      {repository.owner}/{repository.repo_name}
                    </a>
                  ) : (
                    <span className="text-gray-400">Unknown</span>
                  )}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500">Issue</dt>
                <dd className="font-medium">
                  {issueUrl ? (
                    <a
                      href={issueUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:underline"
                    >
                      #{job.issue_number}
                    </a>
                  ) : (
                    `#${job.issue_number}`
                  )}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500">Branch</dt>
                <dd className="font-medium">
                  {job.branch_name ?? <span className="text-gray-400">-</span>}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500">Pull Request</dt>
                <dd className="font-medium">
                  {prUrl ? (
                    <a
                      href={prUrl}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="text-blue-600 hover:underline"
                    >
                      #{job.pr_number}
                    </a>
                  ) : (
                    <span className="text-gray-400">-</span>
                  )}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500">Worktree Path</dt>
                <dd className="font-medium font-mono text-sm">
                  {job.worktree_path ?? <span className="text-gray-400">-</span>}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500">Jobworkerp Job ID</dt>
                <dd className="font-medium font-mono text-sm">
                  {job.jobworkerp_job_id}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500">Created</dt>
                <dd className="font-medium">
                  {new Date(job.created_at).toLocaleString()}
                </dd>
              </div>

              <div>
                <dt className="text-sm text-gray-500">Updated</dt>
                <dd className="font-medium">
                  {new Date(job.updated_at).toLocaleString()}
                </dd>
              </div>
            </dl>
          </div>

          {job.error_message && (
            <div className="border border-red-200 rounded-lg p-6 bg-red-50">
              <h2 className="text-xl font-semibold mb-2 text-red-700">Error</h2>
              <pre className="text-sm text-red-600 whitespace-pre-wrap font-mono">
                {job.error_message}
              </pre>
            </div>
          )}
        </div>

        <div className="space-y-6">
          <div className="border rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Actions</h2>

            <div className="space-y-3">
              {issueUrl && (
                <a
                  href={issueUrl}
                  target="_blank"
                  rel="noopener noreferrer"
                  className="block w-full px-4 py-2 text-center border rounded hover:bg-gray-50"
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

              {["Pending", "PreparingWorkspace", "FetchingIssue", "RunningAgent", "CreatingPR"].includes(job.status) && (
                <button
                  type="button"
                  className="block w-full px-4 py-2 text-center border border-red-600 text-red-600 rounded hover:bg-red-50"
                  disabled
                  title="Cancel functionality coming soon"
                >
                  Cancel Job
                </button>
              )}
            </div>
          </div>

          <div className="border rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Status Legend</h2>
            <ul className="text-sm space-y-2">
              <li className="flex items-center gap-2">
                <span className="w-3 h-3 rounded-full bg-gray-300" />
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
      <div className={`border rounded-lg p-6 ${status === "Failed" ? "border-red-200 bg-red-50" : "border-gray-200 bg-gray-50"}`}>
        <div className="flex items-center gap-3">
          <span className={`w-4 h-4 rounded-full ${status === "Failed" ? "bg-red-500" : "bg-gray-400"}`} />
          <span className={`text-lg font-semibold ${status === "Failed" ? "text-red-700" : "text-gray-700"}`}>
            {statusLabels[status]}
          </span>
        </div>
      </div>
    );
  }

  const currentIndex = statusSteps.indexOf(status);

  return (
    <div className="border rounded-lg p-6">
      <h2 className="text-lg font-semibold mb-4">Progress</h2>
      <div className="relative">
        <div className="absolute left-2 top-0 bottom-0 w-0.5 bg-gray-200" />

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
                  ${state === "pending" ? "bg-gray-300" : ""}
                `}
              />
              <span
                className={`
                  ${state === "completed" ? "text-green-700" : ""}
                  ${state === "current" ? "text-blue-700 font-semibold" : ""}
                  ${state === "pending" ? "text-gray-400" : ""}
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
