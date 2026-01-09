import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { type Repository, type Issue, type PullRequest } from "@/types/models";

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
 * Format a datetime string safely, returning fallback for invalid dates
 */
function formatDateTime(dateStr: string | null | undefined, fallback = "-"): string {
  if (!dateStr) return fallback;
  const date = new Date(dateStr);
  if (Number.isNaN(date.getTime())) return fallback;
  return date.toLocaleString();
}

export const Route = createFileRoute("/repositories/$repoId")({
  component: RepositoryDetailPage,
});

function RepositoryDetailPage() {
  const { repoId } = Route.useParams();
  const numericRepoId = Number(repoId);
  const isValidRepoId = Number.isSafeInteger(numericRepoId) && numericRepoId > 0;

  const repositoryQuery = useQuery({
    queryKey: ["repository", numericRepoId],
    queryFn: () => invoke<Repository>("get_repository", { id: numericRepoId }),
    enabled: isValidRepoId,
  });

  const issuesQuery = useQuery({
    queryKey: ["issues", numericRepoId, "open"],
    queryFn: () =>
      invoke<Issue[]>("list_issues", {
        repository_id: numericRepoId,
        state: "open",
      }),
    enabled: isValidRepoId && !!repositoryQuery.data,
  });

  const pullsQuery = useQuery({
    queryKey: ["pulls", numericRepoId, "open"],
    queryFn: () =>
      invoke<PullRequest[]>("list_pulls", {
        repository_id: numericRepoId,
        state: "open",
      }),
    enabled: isValidRepoId && !!repositoryQuery.data,
  });

  if (!isValidRepoId) {
    return (
      <div className="container mx-auto p-8">
        <div className="flex items-center gap-4 mb-6">
          <Link to="/repositories" className="text-blue-600 hover:underline">
            &larr; Back to Repositories
          </Link>
        </div>
        <p className="text-red-600">Error: Invalid repository ID</p>
      </div>
    );
  }

  if (repositoryQuery.isLoading) {
    return (
      <div className="container mx-auto p-8">
        <p>Loading repository...</p>
      </div>
    );
  }

  if (repositoryQuery.error) {
    return (
      <div className="container mx-auto p-8">
        <div className="flex items-center gap-4 mb-6">
          <Link to="/repositories" className="text-blue-600 hover:underline">
            &larr; Back to Repositories
          </Link>
        </div>
        <p className="text-red-600">
          Error: {String(repositoryQuery.error)}
        </p>
      </div>
    );
  }

  const repo = repositoryQuery.data;
  if (!repo) {
    return (
      <div className="container mx-auto p-8">
        <div className="flex items-center gap-4 mb-6">
          <Link to="/repositories" className="text-blue-600 hover:underline">
            &larr; Back to Repositories
          </Link>
        </div>
        <p className="text-red-600">Repository not found</p>
      </div>
    );
  }

  const openIssueCount = issuesQuery.data?.length ?? 0;
  const openPullCount = pullsQuery.data?.length ?? 0;

  return (
    <div className="container mx-auto p-8">
      <div className="flex items-center gap-4 mb-6">
        <Link to="/repositories" className="text-blue-600 hover:underline">
          &larr; Back to Repositories
        </Link>
        <h1 className="text-3xl font-bold">
          {repo.owner}/{repo.repo_name}
        </h1>
      </div>

      <div className="grid grid-cols-1 lg:grid-cols-3 gap-6">
        <div className="lg:col-span-2 space-y-6">
          {/* Repository Info */}
          <div className="border rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Repository Details</h2>
            <dl className="grid grid-cols-2 gap-4">
              <div>
                <dt className="text-sm text-gray-500">Platform</dt>
                <dd className="font-medium">{repo.platform}</dd>
              </div>
              <div>
                <dt className="text-sm text-gray-500">MCP Server</dt>
                <dd className="font-medium">{repo.mcp_server_name}</dd>
              </div>
              <div>
                <dt className="text-sm text-gray-500">URL</dt>
                <dd className="font-medium">
                  <a
                    href={repo.url}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="text-blue-600 hover:underline"
                  >
                    {repo.url}
                  </a>
                </dd>
              </div>
              {repo.local_path && (
                <div>
                  <dt className="text-sm text-gray-500">Local Path</dt>
                  <dd className="font-medium font-mono text-sm">
                    {repo.local_path}
                  </dd>
                </div>
              )}
              {repo.last_synced_at && (
                <div>
                  <dt className="text-sm text-gray-500">Last Synced</dt>
                  <dd className="font-medium">
                    {formatDateTime(repo.last_synced_at)}
                  </dd>
                </div>
              )}
            </dl>
          </div>

          {/* Quick Stats */}
          <div className="grid grid-cols-2 gap-4">
            <Link
              to="/repositories/$repoId/issues"
              params={{ repoId: String(repo.id) }}
              className="border rounded-lg p-6 hover:shadow-md transition-shadow"
            >
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-lg font-semibold">Issues</h3>
                  <p className="text-sm text-gray-500">
                    {issuesQuery.isLoading
                      ? "Loading..."
                      : `${openIssueCount} open`}
                  </p>
                </div>
                <span className="text-3xl font-bold text-blue-600">
                  {issuesQuery.isLoading ? "-" : openIssueCount}
                </span>
              </div>
            </Link>

            <Link
              to="/repositories/$repoId/pulls"
              params={{ repoId: String(repo.id) }}
              className="border rounded-lg p-6 hover:shadow-md transition-shadow"
            >
              <div className="flex items-center justify-between">
                <div>
                  <h3 className="text-lg font-semibold">Pull Requests</h3>
                  <p className="text-sm text-gray-500">
                    {pullsQuery.isLoading
                      ? "Loading..."
                      : `${openPullCount} open`}
                  </p>
                </div>
                <span className="text-3xl font-bold text-green-600">
                  {pullsQuery.isLoading ? "-" : openPullCount}
                </span>
              </div>
            </Link>
          </div>
        </div>

        {/* Sidebar */}
        <div className="space-y-6">
          <div className="border rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Actions</h2>
            <div className="space-y-3">
              <a
                href={repo.url}
                target="_blank"
                rel="noopener noreferrer"
                className="block w-full px-4 py-2 text-center border rounded hover:bg-gray-50"
              >
                View on {repo.platform}
              </a>
              <Link
                to="/repositories/$repoId/issues"
                params={{ repoId: String(repo.id) }}
                className="block w-full px-4 py-2 text-center bg-blue-600 text-white rounded hover:bg-blue-700"
              >
                Browse Issues
              </Link>
              <Link
                to="/repositories/$repoId/pulls"
                params={{ repoId: String(repo.id) }}
                className="block w-full px-4 py-2 text-center border border-green-600 text-green-600 rounded hover:bg-green-50"
              >
                Browse Pull Requests
              </Link>
            </div>
          </div>

          <div className="border rounded-lg p-6">
            <h2 className="text-xl font-semibold mb-4">Info</h2>
            <dl className="space-y-2 text-sm">
              <div>
                <dt className="text-gray-500">Created</dt>
                <dd>{formatDate(repo.created_at)}</dd>
              </div>
              <div>
                <dt className="text-gray-500">Updated</dt>
                <dd>{formatDate(repo.updated_at)}</dd>
              </div>
            </dl>
          </div>
        </div>
      </div>
    </div>
  );
}
