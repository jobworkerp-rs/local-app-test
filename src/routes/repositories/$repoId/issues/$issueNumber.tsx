import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { useState } from "react";
import { ExternalLink } from "@/components/ExternalLink";
import { repositoryQueries, issueQueries, pullQueries } from "@/lib/query";
import { RunAgentDialog } from "@/components/agent/RunAgentDialog";

/**
 * Format a date string safely, returning fallback for invalid dates
 */
function formatDate(dateStr: string | null | undefined, fallback = "-"): string {
  if (!dateStr) return fallback;
  const date = new Date(dateStr);
  if (Number.isNaN(date.getTime())) return fallback;
  return date.toLocaleDateString();
}

/**
 * Format a date string with time
 */
function formatDateTime(dateStr: string | null | undefined, fallback = "-"): string {
  if (!dateStr) return fallback;
  const date = new Date(dateStr);
  if (Number.isNaN(date.getTime())) return fallback;
  return date.toLocaleString();
}

export const Route = createFileRoute("/repositories/$repoId/issues/$issueNumber")({
  component: IssueDetailPage,
});

function IssueDetailPage() {
  const { repoId, issueNumber } = Route.useParams();
  const numericRepoId = Number(repoId);
  const numericIssueNumber = Number(issueNumber);
  const isValidRepoId = Number.isSafeInteger(numericRepoId) && numericRepoId > 0;
  const isValidIssueNumber = Number.isSafeInteger(numericIssueNumber) && numericIssueNumber > 0;

  const [isDialogOpen, setIsDialogOpen] = useState(false);

  const repositoryQuery = useQuery({
    ...repositoryQueries.detail(numericRepoId),
    enabled: isValidRepoId,
  });

  const issueQuery = useQuery({
    ...issueQueries.detail(numericRepoId, numericIssueNumber),
    enabled: isValidRepoId && isValidIssueNumber,
  });

  const commentsQuery = useQuery({
    ...issueQueries.comments(numericRepoId, numericIssueNumber),
    enabled: isValidRepoId && isValidIssueNumber,
  });

  const relatedPrsQuery = useQuery({
    ...pullQueries.related(numericRepoId, numericIssueNumber),
    enabled: isValidRepoId && isValidIssueNumber,
  });

  if (!isValidRepoId || !isValidIssueNumber) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-red-600 dark:text-red-400">
          Error: Invalid repository ID or issue number
        </p>
      </div>
    );
  }

  if (repositoryQuery.isLoading || issueQuery.isLoading) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-slate-600 dark:text-slate-400">Loading...</p>
      </div>
    );
  }

  if (repositoryQuery.error) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-red-600 dark:text-red-400">
          Error: {String(repositoryQuery.error)}
        </p>
      </div>
    );
  }

  if (issueQuery.error) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-red-600 dark:text-red-400">
          Error: {String(issueQuery.error)}
        </p>
      </div>
    );
  }

  const repo = repositoryQuery.data;
  const issue = issueQuery.data;
  const comments = commentsQuery.data ?? [];
  const relatedPrs = relatedPrsQuery.isSuccess ? relatedPrsQuery.data : [];
  const hasOpenPr = relatedPrs.some((pr) => pr.state === "open");
  const hasMergedPr = relatedPrs.some((pr) => pr.merged);

  if (!issue) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-red-600 dark:text-red-400">Issue not found</p>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-8">
      {/* Header Navigation */}
      <div className="flex items-center gap-4 mb-6">
        <Link
          to="/repositories/$repoId/issues"
          params={{ repoId }}
          className="text-blue-600 dark:text-blue-400 hover:underline"
        >
          &larr; Back to Issues
        </Link>
        <span className="text-gray-400">/</span>
        <span className="text-gray-600 dark:text-gray-400">
          {repo ? `${repo.owner}/${repo.repo_name}` : "..."}
        </span>
      </div>

      {/* Issue Header */}
      <div className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg p-6 mb-6">
        <div className="flex items-start justify-between">
          <div className="flex-1">
            <div className="flex items-center gap-3 mb-3">
              <span
                className={`px-2 py-1 rounded text-sm font-medium ${
                  issue.state === "open"
                    ? "text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-900"
                    : "text-purple-700 dark:text-purple-300 bg-purple-100 dark:bg-purple-900"
                }`}
              >
                {issue.state}
              </span>
              <span className="text-gray-500 dark:text-gray-400">#{issue.number}</span>
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

            <h1 className="text-2xl font-bold mb-3">{issue.title}</h1>

            {issue.labels.length > 0 && (
              <div className="flex flex-wrap gap-1 mb-3">
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

            <p className="text-sm text-gray-500 dark:text-gray-400">
              Opened by <strong>{issue.user}</strong> on {formatDate(issue.created_at)}
              {issue.updated_at && issue.updated_at !== issue.created_at && (
                <> &middot; Updated {formatDate(issue.updated_at)}</>
              )}
            </p>
          </div>

          <div className="flex flex-col gap-2 ml-6">
            <ExternalLink
              href={issue.html_url}
              className="px-4 py-2 text-sm border border-slate-300 dark:border-slate-600 rounded hover:bg-gray-50 dark:hover:bg-slate-700 text-center"
            >
              Open in Browser
            </ExternalLink>
            {issue.state === "open" && (
              <button
                type="button"
                onClick={() => setIsDialogOpen(true)}
                className="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700"
              >
                Run Agent
              </button>
            )}
          </div>
        </div>
      </div>

      {/* Issue Body */}
      <div className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg p-6 mb-6">
        <h2 className="text-lg font-semibold mb-4">Description</h2>
        {issue.body ? (
          <div className="prose dark:prose-invert max-w-none whitespace-pre-wrap">
            {issue.body}
          </div>
        ) : (
          <p className="text-gray-500 dark:text-gray-400 italic">No description provided.</p>
        )}
      </div>

      {/* Related PRs */}
      {relatedPrs.length > 0 && (
        <div className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg p-6 mb-6">
          <h2 className="text-lg font-semibold mb-4">Related Pull Requests</h2>
          <div className="space-y-3">
            {relatedPrs.map((pr) => (
              <div
                key={pr.number}
                className="flex items-center justify-between p-3 bg-slate-50 dark:bg-slate-900 rounded"
              >
                <div className="flex items-center gap-3">
                  <span
                    className={`px-2 py-1 rounded text-xs font-medium ${
                      pr.merged
                        ? "bg-indigo-100 dark:bg-indigo-900 text-indigo-700 dark:text-indigo-300"
                        : pr.state === "open"
                          ? "bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300"
                          : "bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300"
                    }`}
                  >
                    {pr.merged ? "Merged" : pr.state}
                  </span>
                  <span className="text-gray-500 dark:text-gray-400">#{pr.number}</span>
                  <span>{pr.title}</span>
                </div>
                <ExternalLink
                  href={pr.html_url}
                  className="text-blue-600 dark:text-blue-400 hover:underline text-sm"
                >
                  View PR
                </ExternalLink>
              </div>
            ))}
          </div>
        </div>
      )}

      {/* Comments */}
      <div className="bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 rounded-lg p-6">
        <h2 className="text-lg font-semibold mb-4">
          Comments {comments.length > 0 && `(${comments.length})`}
        </h2>
        {commentsQuery.isLoading ? (
          <p className="text-slate-600 dark:text-slate-400">Loading comments...</p>
        ) : commentsQuery.error ? (
          <p className="text-red-600 dark:text-red-400">
            Error loading comments: {String(commentsQuery.error)}
          </p>
        ) : comments.length === 0 ? (
          <p className="text-gray-500 dark:text-gray-400 italic">No comments yet.</p>
        ) : (
          <div className="space-y-4">
            {comments.map((comment) => (
              <div
                key={comment.id}
                className="border-l-2 border-slate-300 dark:border-slate-600 pl-4"
              >
                <div className="flex items-center gap-2 mb-2">
                  <span className="font-medium">{comment.user}</span>
                  <span className="text-gray-500 dark:text-gray-400 text-sm">
                    {formatDateTime(comment.created_at)}
                  </span>
                </div>
                <div className="prose dark:prose-invert max-w-none text-sm whitespace-pre-wrap">
                  {comment.body}
                </div>
              </div>
            ))}
          </div>
        )}
      </div>

      {/* Run Agent Dialog */}
      <RunAgentDialog
        isOpen={isDialogOpen}
        onClose={() => setIsDialogOpen(false)}
        repositoryId={numericRepoId}
        issue={issue}
        relatedPrs={relatedPrs}
      />
    </div>
  );
}
