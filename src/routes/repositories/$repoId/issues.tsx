import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { type Issue } from "@/types/models";
import { ExternalLink } from "@/components/ExternalLink";
import { repositoryQueries, issueQueries, pullQueries } from "@/lib/query";

/**
 * Format a date string safely, returning fallback for invalid dates
 */
function formatDate(dateStr: string | null | undefined, fallback = "-"): string {
  if (!dateStr) return fallback;
  const date = new Date(dateStr);
  if (Number.isNaN(date.getTime())) return fallback;
  return date.toLocaleDateString();
}

export const Route = createFileRoute("/repositories/$repoId/issues")({
  component: RepositoryIssuesPage,
});

type IssueState = "open" | "closed" | "all";

function RepositoryIssuesPage() {
  const { repoId } = Route.useParams();
  const numericRepoId = Number(repoId);
  const isValidRepoId = Number.isSafeInteger(numericRepoId) && numericRepoId > 0;
  const [stateFilter, setStateFilter] = useState<IssueState>("open");

  const repositoryQuery = useQuery({
    ...repositoryQueries.detail(numericRepoId),
    enabled: isValidRepoId,
  });

  const issuesQuery = useQuery({
    ...issueQueries.list(numericRepoId, stateFilter),
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
          {repo ? `${repo.owner}/${repo.repo_name}` : "..."} - Issues
        </h1>
      </div>

      {/* Filter */}
      <div className="mb-6 flex gap-2">
        {(["open", "closed", "all"] as IssueState[]).map((state) => (
          <button
            key={state}
            type="button"
            aria-pressed={stateFilter === state}
            onClick={() => setStateFilter(state)}
            className={`px-4 py-2 rounded ${
              stateFilter === state
                ? "bg-blue-600 text-white"
                : "border border-slate-300 dark:border-slate-600 hover:bg-gray-50 dark:hover:bg-slate-700"
            }`}
          >
            {state.charAt(0).toUpperCase() + state.slice(1)}
          </button>
        ))}
      </div>

      {/* Issues List */}
      {issuesQuery.isLoading ? (
        <p className="text-slate-600 dark:text-slate-400">Loading issues...</p>
      ) : issuesQuery.error ? (
        <p className="text-red-600 dark:text-red-400">Error: {String(issuesQuery.error)}</p>
      ) : issuesQuery.data?.length === 0 ? (
        <div className="text-center py-12">
          <p className="text-gray-500 dark:text-gray-400">
            {stateFilter === "all"
              ? "No issues found."
              : `No ${stateFilter} issues found.`}
          </p>
        </div>
      ) : (
        <div className="space-y-4">
          {issuesQuery.data?.map((issue) => (
            <IssueCard
              key={issue.number}
              issue={issue}
              repositoryId={numericRepoId}
              repoId={repoId}
            />
          ))}
        </div>
      )}
    </div>
  );
}

interface IssueCardProps {
  issue: Issue;
  repositoryId: number;
  repoId: string;
}

function IssueCard({ issue, repositoryId, repoId }: IssueCardProps) {
  const relatedPrsQuery = useQuery(pullQueries.related(repositoryId, issue.number));

  const relatedPrs = relatedPrsQuery.isSuccess ? relatedPrsQuery.data : [];
  const hasOpenPr = relatedPrsQuery.isSuccess && relatedPrs.some((pr) => pr.state === "open");
  const hasMergedPr = relatedPrsQuery.isSuccess && relatedPrs.some((pr) => pr.merged);
  const hasRelatedPrs = relatedPrsQuery.isSuccess && relatedPrs.length > 0;

  return (
    <div className="border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 rounded-lg p-4 hover:shadow-md transition-shadow">
      <div className="flex justify-between items-start">
        <div className="flex-1">
          <div className="flex items-center gap-3 mb-2">
            <span
              className={`px-2 py-1 rounded text-sm font-medium ${
                issue.state === "open"
                  ? "text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-900"
                  : "text-purple-700 dark:text-purple-300 bg-purple-100 dark:bg-purple-900"
              }`}
            >
              {issue.state}
            </span>
            <span className="text-gray-500 dark:text-gray-400 text-sm">#{issue.number}</span>
            {hasOpenPr && (
              <span className="px-2 py-1 rounded text-xs font-medium text-yellow-700 dark:text-yellow-300 bg-yellow-100 dark:bg-yellow-900">
                PR Open
              </span>
            )}
            {hasMergedPr && (
              <span className="px-2 py-1 rounded text-xs font-medium text-indigo-700 dark:text-indigo-300 bg-indigo-100 dark:bg-indigo-900">
                PR Merged
              </span>
            )}
          </div>

          <h3 className="text-lg font-semibold">
            <Link
              to="/repositories/$repoId/issues/$issueNumber"
              params={{ repoId, issueNumber: String(issue.number) }}
              className="hover:text-blue-600 dark:hover:text-blue-400"
            >
              {issue.title}
            </Link>
          </h3>

          {issue.labels.length > 0 && (
            <div className="flex flex-wrap gap-1 mt-2">
              {issue.labels.map((label) => (
                <span
                  key={label}
                  className="px-2 py-0.5 text-xs rounded bg-gray-200 dark:bg-gray-700"
                >
                  {label}
                </span>
              ))}
            </div>
          )}

          <p className="text-sm text-gray-500 dark:text-gray-400 mt-2">
            Opened by {issue.user} on {formatDate(issue.created_at)}
          </p>

          {/* Related PRs */}
          {hasRelatedPrs && (
            <div className="mt-3 pt-3 border-t border-slate-200 dark:border-slate-700">
              <p className="text-sm text-gray-600 dark:text-gray-400 mb-1">Related PRs:</p>
              <div className="flex flex-wrap gap-2">
                {relatedPrs.map((pr) => (
                  <ExternalLink
                    key={pr.number}
                    href={pr.html_url}
                    className={`text-sm px-2 py-1 rounded ${
                      pr.merged
                        ? "bg-indigo-100 dark:bg-indigo-900 text-indigo-700 dark:text-indigo-300"
                        : pr.state === "open"
                          ? "bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300"
                          : "bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300"
                    }`}
                  >
                    #{pr.number} {pr.merged ? "(merged)" : `(${pr.state})`}
                  </ExternalLink>
                ))}
              </div>
            </div>
          )}
        </div>

        <div className="flex flex-col gap-2 ml-4">
          <ExternalLink
            href={issue.html_url}
            className="px-3 py-1 text-sm border border-slate-300 dark:border-slate-600 rounded hover:bg-gray-50 dark:hover:bg-slate-700"
          >
            Open in Browser
          </ExternalLink>
          <Link
            to="/repositories/$repoId/issues/$issueNumber"
            params={{ repoId, issueNumber: String(issue.number) }}
            className="px-3 py-1 text-sm text-center border border-blue-500 text-blue-600 dark:text-blue-400 rounded hover:bg-blue-50 dark:hover:bg-blue-900/30"
          >
            View Details
          </Link>
        </div>
      </div>
    </div>
  );
}
