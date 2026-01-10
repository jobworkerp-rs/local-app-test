import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import {
  type AgentJobStatus,
  type AgentJob,
  type Repository,
  ACTIVE_JOB_STATUSES,
} from "@/types/models";
import { jobQueries, repositoryQueries } from "@/lib/query";

export const Route = createFileRoute("/jobs/")({
  component: JobsPage,
});

const statusConfig: Record<AgentJobStatus, { label: string; color: string; darkColor: string; bgColor: string; darkBgColor: string }> = {
  Pending: { label: "Pending", color: "text-gray-700", darkColor: "dark:text-gray-300", bgColor: "bg-gray-100", darkBgColor: "dark:bg-gray-800" },
  PreparingWorkspace: { label: "Preparing", color: "text-blue-700", darkColor: "dark:text-blue-300", bgColor: "bg-blue-100", darkBgColor: "dark:bg-blue-900" },
  FetchingIssue: { label: "Fetching", color: "text-blue-700", darkColor: "dark:text-blue-300", bgColor: "bg-blue-100", darkBgColor: "dark:bg-blue-900" },
  RunningAgent: { label: "Running", color: "text-yellow-700", darkColor: "dark:text-yellow-300", bgColor: "bg-yellow-100", darkBgColor: "dark:bg-yellow-900" },
  CreatingPR: { label: "Creating PR", color: "text-purple-700", darkColor: "dark:text-purple-300", bgColor: "bg-purple-100", darkBgColor: "dark:bg-purple-900" },
  PrCreated: { label: "PR Created", color: "text-green-700", darkColor: "dark:text-green-300", bgColor: "bg-green-100", darkBgColor: "dark:bg-green-900" },
  Merged: { label: "Merged", color: "text-indigo-700", darkColor: "dark:text-indigo-300", bgColor: "bg-indigo-100", darkBgColor: "dark:bg-indigo-900" },
  Completed: { label: "Completed", color: "text-green-700", darkColor: "dark:text-green-300", bgColor: "bg-green-100", darkBgColor: "dark:bg-green-900" },
  Failed: { label: "Failed", color: "text-red-700", darkColor: "dark:text-red-300", bgColor: "bg-red-100", darkBgColor: "dark:bg-red-900" },
  Cancelled: { label: "Cancelled", color: "text-gray-700", darkColor: "dark:text-gray-300", bgColor: "bg-gray-100", darkBgColor: "dark:bg-gray-800" },
};

function JobsPage() {
  const jobsQuery = useQuery(jobQueries.list());
  const repositoriesQuery = useQuery(repositoryQueries.list());

  const repoMap = new Map<number, Repository>();
  repositoriesQuery.data?.forEach((repo) => {
    repoMap.set(repo.id, repo);
  });

  return (
    <div className="container mx-auto p-8">
      <div className="flex items-center gap-4 mb-6">
        <Link to="/" className="text-blue-600 dark:text-blue-400 hover:underline">
          &larr; Back
        </Link>
        <h1 className="text-3xl font-bold">Agent Jobs</h1>
      </div>

      {jobsQuery.isLoading ? (
        <p className="text-slate-600 dark:text-slate-400">Loading jobs...</p>
      ) : jobsQuery.error ? (
        <p className="text-red-600 dark:text-red-400">Error: {String(jobsQuery.error)}</p>
      ) : jobsQuery.data?.length === 0 ? (
        <div className="text-center py-12">
          <p className="text-gray-500 dark:text-gray-400 mb-4">
            No agent jobs yet. Start by selecting an issue from a repository.
          </p>
          <Link
            to="/repositories"
            className="text-blue-600 dark:text-blue-400 hover:underline"
          >
            Go to Repositories
          </Link>
        </div>
      ) : (
        <div className="space-y-4">
          {jobsQuery.data?.map((job) => (
            <JobCard
              key={job.id}
              job={job}
              repository={repoMap.get(job.repository_id)}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface JobCardProps {
  job: AgentJob;
  repository?: Repository;
}

function JobCard({ job, repository }: JobCardProps) {
  const status = statusConfig[job.status];
  const isActive = ACTIVE_JOB_STATUSES.includes(job.status);

  return (
    <Link
      to="/jobs/$jobId"
      params={{ jobId: String(job.id) }}
      className="block border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 rounded-lg p-4 hover:shadow-md transition-shadow"
    >
      <div className="flex justify-between items-start">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <span className={`px-2 py-1 rounded text-sm font-medium ${status.color} ${status.darkColor} ${status.bgColor} ${status.darkBgColor}`}>
              {status.label}
              {isActive && (
                <span className="ml-1 inline-block w-2 h-2 bg-current rounded-full animate-pulse" />
              )}
            </span>
            <span className="text-gray-500 dark:text-gray-400 text-sm">
              #{job.issue_number}
            </span>
          </div>

          <h3 className="text-lg font-semibold">
            {repository ? (
              <>
                {repository.owner}/{repository.repo_name}
              </>
            ) : (
              <span className="text-gray-400 dark:text-gray-500">Unknown Repository</span>
            )}
          </h3>

          <div className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            {job.branch_name && (
              <span className="mr-4">Branch: {job.branch_name}</span>
            )}
            {job.pr_number && (
              <span className="mr-4">PR #{job.pr_number}</span>
            )}
          </div>

          {job.error_message && (
            <p className="text-sm text-red-600 dark:text-red-400 mt-2 line-clamp-2">
              {job.error_message}
            </p>
          )}
        </div>

        <div className="text-right text-sm text-gray-400 dark:text-gray-500">
          <p>{new Date(job.created_at).toLocaleDateString()}</p>
          <p>{new Date(job.created_at).toLocaleTimeString()}</p>
        </div>
      </div>
    </Link>
  );
}
