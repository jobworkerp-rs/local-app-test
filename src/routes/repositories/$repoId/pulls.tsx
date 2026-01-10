import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { type PullRequest } from "@/types/models";
import { ExternalLink } from "@/components/ExternalLink";
import { repositoryQueries, pullQueries } from "@/lib/query";

/**
 * Format a date string safely, returning fallback for invalid dates
 */
function formatDate(dateStr: string | null | undefined, fallback = "-"): string {
  if (!dateStr) return fallback;
  const date = new Date(dateStr);
  if (Number.isNaN(date.getTime())) return fallback;
  return date.toLocaleDateString();
}

export const Route = createFileRoute("/repositories/$repoId/pulls")({
  component: RepositoryPullsPage,
});

type PullState = "open" | "closed" | "all";

function RepositoryPullsPage() {
  const { repoId } = Route.useParams();
  const numericRepoId = Number(repoId);
  const isValidRepoId = Number.isSafeInteger(numericRepoId) && numericRepoId > 0;
  const [stateFilter, setStateFilter] = useState<PullState>("open");

  const repositoryQuery = useQuery({
    ...repositoryQueries.detail(numericRepoId),
    enabled: isValidRepoId,
  });

  const pullsQuery = useQuery({
    ...pullQueries.list(numericRepoId, stateFilter),
    enabled: isValidRepoId && repositoryQuery.isSuccess,
  });

  if (!isValidRepoId) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-red-600 dark:text-red-400">Error: Invalid repository ID</p>
      </div>
    );
  }

  if (repositoryQuery.isLoading) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-slate-600 dark:text-slate-400">Loading repository...</p>
      </div>
    );
  }

  if (repositoryQuery.error) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-red-600 dark:text-red-400">Error: {String(repositoryQuery.error)}</p>
      </div>
    );
  }

  const repo = repositoryQuery.data;

  return (
    <div className="container mx-auto p-8">
      <div className="flex items-center gap-4 mb-6">
        <Link
          to="/repositories/$repoId"
          params={{ repoId }}
          className="text-blue-600 dark:text-blue-400 hover:underline"
        >
          &larr; Back to Repository
        </Link>
        <h1 className="text-3xl font-bold">
          {repo ? `${repo.owner}/${repo.repo_name}` : "..."} - Pull Requests
        </h1>
      </div>

      {/* Filter */}
      <div className="mb-6 flex gap-2">
        {(["open", "closed", "all"] as PullState[]).map((state) => (
          <button
            key={state}
            type="button"
            aria-pressed={stateFilter === state}
            onClick={() => setStateFilter(state)}
            className={`px-4 py-2 rounded ${
              stateFilter === state
                ? "bg-green-600 text-white"
                : "border border-slate-300 dark:border-slate-600 hover:bg-gray-50 dark:hover:bg-slate-700"
            }`}
          >
            {state.charAt(0).toUpperCase() + state.slice(1)}
          </button>
        ))}
      </div>

      {/* PRs List */}
      {pullsQuery.isLoading ? (
        <p className="text-slate-600 dark:text-slate-400">Loading pull requests...</p>
      ) : pullsQuery.error ? (
        <p className="text-red-600 dark:text-red-400">Error: {String(pullsQuery.error)}</p>
      ) : pullsQuery.data?.length === 0 ? (
        <div className="text-center py-12">
          <p className="text-gray-500 dark:text-gray-400">
            {stateFilter === "all"
              ? "No pull requests found."
              : `No ${stateFilter} pull requests found.`}
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {pullsQuery.data?.map((pr) => (
            <PullRequestCard key={pr.number} pr={pr} />
          ))}
        </div>
      )}
    </div>
  );
}

interface PullRequestCardProps {
  pr: PullRequest;
}

function PullRequestCard({ pr }: PullRequestCardProps) {
  const getStatusBadge = () => {
    if (pr.merged) {
      return { text: "Merged", color: "text-indigo-700 dark:text-indigo-300 bg-indigo-100 dark:bg-indigo-900" };
    }
    if (pr.state === "open") {
      return { text: "Open", color: "text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-900" };
    }
    return { text: "Closed", color: "text-red-700 dark:text-red-300 bg-red-100 dark:bg-red-900" };
  };

  const status = getStatusBadge();

  return (
    <div className="border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 rounded-lg p-4 hover:shadow-md transition-shadow">
      <div className="flex justify-between items-start">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <span
              className={`px-2 py-1 rounded text-sm font-medium ${status.color}`}
            >
              {status.text}
            </span>
            <span className="text-gray-500 dark:text-gray-400 text-sm">#{pr.number}</span>
          </div>

          <h3 className="text-lg font-semibold">
            <ExternalLink
              href={pr.html_url}
              className="hover:text-blue-600 dark:hover:text-blue-400"
            >
              {pr.title}
            </ExternalLink>
          </h3>

          {(pr.head_branch || pr.base_branch) && (
            <div className="flex gap-4 text-sm text-gray-500 dark:text-gray-400 mt-2">
              <span>
                <span className="font-medium">{pr.head_branch ?? "?"}</span>
                {" → "}
                <span className="font-medium">{pr.base_branch ?? "?"}</span>
              </span>
            </div>
          )}

          <p className="text-sm text-gray-500 dark:text-gray-400 mt-1">
            Created on {formatDate(pr.created_at)}
          </p>
        </div>

        <div className="flex flex-col gap-2 ml-4">
          <ExternalLink
            href={pr.html_url}
            className="px-3 py-1 text-sm border border-slate-300 dark:border-slate-600 rounded hover:bg-gray-50 dark:hover:bg-slate-700"
          >
            View
          </ExternalLink>
        </div>
      </div>
    </div>
  );
}
