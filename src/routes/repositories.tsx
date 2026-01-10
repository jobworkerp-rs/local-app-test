import { createFileRoute, Link, Outlet, useMatch } from "@tanstack/react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useState, useCallback, type FormEvent } from "react";
import {
  type Repository,
  type CreateRepositoryRequest,
  type McpServerInfo,
  type CreateMcpRunnerRequest,
  getGiteaWebBaseUrl,
} from "@/types/models";

export const Route = createFileRoute("/repositories")({
  component: RepositoriesLayout,
});

function RepositoriesLayout() {
  const repoIdMatch = useMatch({
    from: "/repositories/$repoId",
    shouldThrow: false,
  });

  // If we're on a child route (has repoId param), render Outlet for the child
  if (repoIdMatch) {
    return <Outlet />;
  }

  // Otherwise, render the repositories list page
  return <RepositoriesPage />;
}

interface DeleteConfirmState {
  isOpen: boolean;
  repositoryId: number | null;
  repositoryName: string;
}

function RepositoriesPage() {
  const queryClient = useQueryClient();
  const [showForm, setShowForm] = useState(false);
  const [deleteConfirm, setDeleteConfirm] = useState<DeleteConfirmState>({
    isOpen: false,
    repositoryId: null,
    repositoryName: "",
  });

  const repositoriesQuery = useQuery({
    queryKey: ["repositories"],
    queryFn: () => invoke<Repository[]>("list_repositories"),
  });

  const mcpServersQuery = useQuery({
    queryKey: ["mcp-servers"],
    queryFn: () => invoke<McpServerInfo[]>("mcp_list_servers"),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => invoke("delete_repository", { id }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["repositories"] });
      setDeleteConfirm({ isOpen: false, repositoryId: null, repositoryName: "" });
    },
    onError: (error) => {
      console.error("Delete repository error:", error);
    },
  });

  const openDeleteConfirm = useCallback((id: number, name: string) => {
    setDeleteConfirm({ isOpen: true, repositoryId: id, repositoryName: name });
  }, []);

  const closeDeleteConfirm = useCallback(() => {
    setDeleteConfirm({ isOpen: false, repositoryId: null, repositoryName: "" });
  }, []);

  const confirmDelete = useCallback(() => {
    if (deleteConfirm.repositoryId !== null) {
      deleteMutation.mutate(deleteConfirm.repositoryId);
    }
  }, [deleteConfirm.repositoryId, deleteMutation]);

  return (
    <div className="container mx-auto p-8">
      <div className="flex items-center gap-4 mb-6">
        <Link to="/" className="text-blue-600 dark:text-blue-400 hover:underline">
          &larr; Back
        </Link>
        <h1 className="text-3xl font-bold">Repositories</h1>
      </div>

      <div className="mb-6">
        <button
          type="button"
          onClick={() => setShowForm(!showForm)}
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700"
        >
          {showForm ? "Cancel" : "Add Repository"}
        </button>
      </div>

      {showForm && (
        <RepositoryForm
          mcpServers={mcpServersQuery.data ?? []}
          onSuccess={() => {
            setShowForm(false);
            queryClient.invalidateQueries({ queryKey: ["repositories"] });
          }}
        />
      )}

      {repositoriesQuery.isLoading ? (
        <p className="text-slate-600 dark:text-slate-400">Loading repositories...</p>
      ) : repositoriesQuery.error ? (
        <p className="text-red-600 dark:text-red-400">
          Error: {String(repositoriesQuery.error)}
        </p>
      ) : repositoriesQuery.data?.length === 0 ? (
        <p className="text-gray-500 dark:text-gray-400">
          No repositories registered. Click "Add Repository" to get started.
        </p>
      ) : (
        <div className="space-y-4">
          {repositoriesQuery.data?.map((repo) => (
            <RepositoryCard
              key={repo.id}
              repository={repo}
              onDelete={() => openDeleteConfirm(repo.id, repo.name)}
              isDeleting={deleteMutation.isPending && deleteConfirm.repositoryId === repo.id}
            />
          ))}
        </div>
      )}

      {/* Delete Confirmation Dialog */}
      {deleteConfirm.isOpen && (
        <ConfirmDialog
          title="Delete Repository"
          message={`Are you sure you want to delete "${deleteConfirm.repositoryName}"?`}
          confirmLabel="Delete"
          confirmVariant="danger"
          isLoading={deleteMutation.isPending}
          error={deleteMutation.error ? String(deleteMutation.error) : undefined}
          onConfirm={confirmDelete}
          onCancel={closeDeleteConfirm}
        />
      )}
    </div>
  );
}

interface RepositoryCardProps {
  repository: Repository;
  onDelete: () => void;
  isDeleting: boolean;
}

function RepositoryCard({ repository, onDelete, isDeleting }: RepositoryCardProps) {
  return (
    <div className="border border-slate-200 dark:border-slate-700 bg-white dark:bg-slate-800 rounded-lg p-4 hover:shadow-md transition-shadow">
      <div className="flex justify-between items-start">
        <div className="flex-1 min-w-0">
          <Link
            to="/repositories/$repoId"
            params={{ repoId: String(repository.id) }}
          >
            <h3 className="text-lg font-semibold text-blue-600 dark:text-blue-400 hover:underline">
              {repository.owner}/{repository.repo_name}
            </h3>
          </Link>
          <p className="text-sm text-gray-500 dark:text-gray-400">
            {repository.platform} &middot; MCP: {repository.mcp_server_name}
          </p>
          {repository.local_path && (
            <p className="text-sm text-gray-400 dark:text-gray-500 mt-1">
              Local: {repository.local_path}
            </p>
          )}
          {repository.last_synced_at && (
            <p className="text-xs text-gray-400 dark:text-gray-500 mt-1">
              Last synced: {new Date(repository.last_synced_at).toLocaleString()}
            </p>
          )}
        </div>
        <div className="flex gap-2 ml-4 shrink-0">
          <a
            href={repository.url}
            target="_blank"
            rel="noopener noreferrer"
            className="px-3 py-1 text-sm border border-slate-300 dark:border-slate-600 rounded hover:bg-gray-50 dark:hover:bg-slate-700 cursor-pointer"
          >
            Open
          </a>
          <button
            type="button"
            onClick={onDelete}
            disabled={isDeleting}
            className="px-3 py-1 text-sm text-red-600 dark:text-red-400 border border-red-600 dark:border-red-500 rounded hover:bg-red-50 dark:hover:bg-red-900/30 disabled:opacity-50 cursor-pointer"
          >
            {isDeleting ? "Deleting..." : "Delete"}
          </button>
        </div>
      </div>
    </div>
  );
}

interface RepositoryFormProps {
  mcpServers: McpServerInfo[];
  onSuccess: () => void;
}

function RepositoryForm({ mcpServers, onSuccess }: RepositoryFormProps) {
  const queryClient = useQueryClient();

  // MCP server selection: existing server name, "new" for creating new, or empty
  const [mcpSelection, setMcpSelection] = useState<string>("");

  // New MCP server creation form data
  const [newMcpData, setNewMcpData] = useState<CreateMcpRunnerRequest>({
    platform: "GitHub",
    name: "",
    url: "https://github.com",
    token: "",
  });

  const [formData, setFormData] = useState<CreateRepositoryRequest>({
    mcp_server_name: "",
    platform: "GitHub",
    base_url: "https://api.github.com",
    name: "",
    url: "",
    owner: "",
    repo_name: "",
    local_path: null,
  });

  // MCP server creation mutation
  const createMcpMutation = useMutation({
    mutationFn: (request: CreateMcpRunnerRequest) =>
      invoke<McpServerInfo>("mcp_create_runner", {
        platform: request.platform,
        name: request.name,
        url: request.url,
        token: request.token,
      }),
    onSuccess: (newServer) => {
      // Refresh MCP server list
      queryClient.invalidateQueries({ queryKey: ["mcp-servers"] });
      // Select the newly created server
      setMcpSelection(newServer.name);
      setFormData((prev) => ({ ...prev, mcp_server_name: newServer.name }));
      // Reset new MCP form
      setNewMcpData({
        platform: "GitHub",
        name: "",
        url: "https://github.com",
        token: "",
      });
    },
  });

  const createMutation = useMutation({
    mutationFn: (request: CreateRepositoryRequest) =>
      invoke<Repository>("create_repository", { request }),
    onSuccess: () => {
      onSuccess();
    },
  });

  // Handle MCP server selection change
  const handleMcpSelectionChange = (value: string) => {
    setMcpSelection(value);
    if (value !== "new") {
      setFormData((prev) => ({ ...prev, mcp_server_name: value }));
    }
  };

  // Handle new MCP platform change
  const handleNewMcpPlatformChange = (platform: "GitHub" | "Gitea") => {
    setNewMcpData({
      ...newMcpData,
      platform,
      url: platform === "GitHub" ? "https://github.com" : "",
    });
  };

  const handlePlatformChange = (platform: "GitHub" | "Gitea") => {
    setFormData({
      ...formData,
      platform,
      base_url: platform === "GitHub" ? "https://api.github.com" : "",
    });
  };

  const handleOwnerRepoChange = (owner: string, repoName: string) => {
    const baseUrl = formData.platform === "GitHub"
      ? "https://github.com"
      : getGiteaWebBaseUrl(formData.base_url);

    setFormData({
      ...formData,
      owner,
      repo_name: repoName,
      name: `${owner}/${repoName}`,
      url: owner && repoName ? `${baseUrl}/${owner}/${repoName}` : "",
    });
  };

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    createMutation.mutate(formData);
  };

  // Check if the form can be submitted
  const canSubmit =
    mcpSelection !== "" &&
    mcpSelection !== "new" &&
    formData.mcp_server_name !== "" &&
    formData.owner.trim() !== "" &&
    formData.repo_name.trim() !== "";

  return (
    <form onSubmit={handleSubmit} className="border border-slate-200 dark:border-slate-700 rounded-lg p-6 mb-6 bg-gray-50 dark:bg-slate-800">
      <h2 className="text-xl font-semibold mb-4">Add Repository</h2>

      {/* MCP Server Selection */}
      <div className="mb-4">
        <label htmlFor="mcp_selection" className="block text-sm font-medium mb-1">
          MCP Server
        </label>
        <select
          id="mcp_selection"
          value={mcpSelection}
          onChange={(e) => handleMcpSelectionChange(e.target.value)}
          className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100"
          required
        >
          <option value="">Select MCP Server</option>
          {mcpServers.map((server) => (
            <option key={server.name} value={server.name}>
              {server.name}
              {server.description ? ` - ${server.description}` : ""}
            </option>
          ))}
          <option value="new">+ Create New MCP Server</option>
        </select>
      </div>

      {/* New MCP Server Creation Form */}
      {mcpSelection === "new" && (
        <div className="border border-blue-200 dark:border-blue-800 rounded-lg p-4 mb-4 bg-blue-50 dark:bg-blue-900/30">
          <h3 className="text-lg font-semibold mb-3">Create New MCP Server</h3>

          {/* Platform Selection */}
          <div className="mb-3">
            <label htmlFor="new_mcp_platform" className="block text-sm font-medium mb-1">
              Platform
            </label>
            <select
              id="new_mcp_platform"
              value={newMcpData.platform}
              onChange={(e) => handleNewMcpPlatformChange(e.target.value as "GitHub" | "Gitea")}
              className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100"
            >
              <option value="GitHub">GitHub</option>
              <option value="Gitea">Gitea</option>
            </select>
          </div>

          {/* Server Name */}
          <div className="mb-3">
            <label htmlFor="new_mcp_name" className="block text-sm font-medium mb-1">
              Server Name
            </label>
            <input
              id="new_mcp_name"
              type="text"
              value={newMcpData.name}
              onChange={(e) => setNewMcpData({ ...newMcpData, name: e.target.value })}
              placeholder="my-github-server"
              className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-500"
              required
            />
          </div>

          {/* URL */}
          <div className="mb-3">
            <label htmlFor="new_mcp_url" className="block text-sm font-medium mb-1">
              URL
              {newMcpData.platform === "GitHub" && (
                <span className="text-gray-500 dark:text-gray-400 ml-2 font-normal">
                  (Change for GitHub Enterprise)
                </span>
              )}
            </label>
            <input
              id="new_mcp_url"
              type="url"
              value={newMcpData.url}
              onChange={(e) => setNewMcpData({ ...newMcpData, url: e.target.value })}
              placeholder={newMcpData.platform === "GitHub" ? "https://github.com" : "https://gitea.example.com"}
              className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-500"
              required
            />
          </div>

          {/* Personal Access Token */}
          <div className="mb-3">
            <label htmlFor="new_mcp_token" className="block text-sm font-medium mb-1">
              Personal Access Token
            </label>
            <input
              id="new_mcp_token"
              type="password"
              value={newMcpData.token}
              onChange={(e) => setNewMcpData({ ...newMcpData, token: e.target.value })}
              placeholder="ghp_xxxx... / gitea_token_xxxx..."
              className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-500"
              required
            />
          </div>

          <button
            type="button"
            onClick={() => createMcpMutation.mutate(newMcpData)}
            disabled={createMcpMutation.isPending || !newMcpData.name || !newMcpData.token || !newMcpData.url}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
          >
            {createMcpMutation.isPending ? "Creating..." : "Create MCP Server"}
          </button>

          {createMcpMutation.isError && (
            <p className="text-red-600 dark:text-red-400 mt-2">
              Error: {String(createMcpMutation.error)}
            </p>
          )}
        </div>
      )}

      {/* Repository Details (only show when MCP server is selected) */}
      {mcpSelection !== "" && mcpSelection !== "new" && (
        <>
          <div className="grid grid-cols-2 gap-4 mb-4">
            <div>
              <label htmlFor="platform" className="block text-sm font-medium mb-1">
                Platform
              </label>
              <select
                id="platform"
                value={formData.platform}
                onChange={(e) => handlePlatformChange(e.target.value as "GitHub" | "Gitea")}
                className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100"
              >
                <option value="GitHub">GitHub</option>
                <option value="Gitea">Gitea</option>
              </select>
            </div>
          </div>

          {formData.platform === "Gitea" && (
            <div className="mb-4">
              <label htmlFor="base_url" className="block text-sm font-medium mb-1">
                Gitea API URL
              </label>
              <input
                id="base_url"
                type="url"
                value={formData.base_url}
                onChange={(e) => setFormData({ ...formData, base_url: e.target.value })}
                placeholder="https://gitea.example.com/api/v1"
                className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-500"
                required
              />
            </div>
          )}

          <div className="grid grid-cols-2 gap-4 mb-4">
            <div>
              <label htmlFor="owner" className="block text-sm font-medium mb-1">
                Owner
              </label>
              <input
                id="owner"
                type="text"
                value={formData.owner}
                onChange={(e) => handleOwnerRepoChange(e.target.value, formData.repo_name)}
                placeholder="owner"
                className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-500"
                required
              />
            </div>

            <div>
              <label htmlFor="repo_name" className="block text-sm font-medium mb-1">
                Repository Name
              </label>
              <input
                id="repo_name"
                type="text"
                value={formData.repo_name}
                onChange={(e) => handleOwnerRepoChange(formData.owner, e.target.value)}
                placeholder="repo-name"
                className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-500"
                required
              />
            </div>
          </div>

          <div className="mb-4">
            <label htmlFor="local_path" className="block text-sm font-medium mb-1">
              Local Clone Path (optional)
            </label>
            <input
              id="local_path"
              type="text"
              value={formData.local_path ?? ""}
              onChange={(e) =>
                setFormData({ ...formData, local_path: e.target.value || null })
              }
              placeholder="/path/to/local/clone"
              className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100 placeholder:text-slate-400 dark:placeholder:text-slate-500"
            />
          </div>

          {formData.url && (
            <p className="text-sm text-gray-500 dark:text-gray-400 mb-4">
              Repository URL: {formData.url}
            </p>
          )}

          <div className="flex gap-2">
            <button
              type="submit"
              disabled={createMutation.isPending || !canSubmit}
              className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
            >
              {createMutation.isPending ? "Creating..." : "Create"}
            </button>
          </div>

          {createMutation.isError && (
            <p className="text-red-600 dark:text-red-400 mt-2">
              Error: {String(createMutation.error)}
            </p>
          )}
        </>
      )}
    </form>
  );
}

interface ConfirmDialogProps {
  title: string;
  message: string;
  confirmLabel?: string;
  cancelLabel?: string;
  confirmVariant?: "danger" | "primary";
  isLoading?: boolean;
  error?: string;
  onConfirm: () => void;
  onCancel: () => void;
}

function ConfirmDialog({
  title,
  message,
  confirmLabel = "Confirm",
  cancelLabel = "Cancel",
  confirmVariant = "primary",
  isLoading = false,
  error,
  onConfirm,
  onCancel,
}: ConfirmDialogProps) {
  const confirmButtonClass =
    confirmVariant === "danger"
      ? "bg-red-600 hover:bg-red-700 text-white"
      : "bg-blue-600 hover:bg-blue-700 text-white";

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={onCancel}
        onKeyDown={(e) => e.key === "Escape" && onCancel()}
      />

      {/* Dialog */}
      <div className="relative bg-white dark:bg-slate-800 rounded-lg shadow-xl max-w-md w-full mx-4 p-6">
        <h2 className="text-xl font-semibold mb-4">{title}</h2>
        <p className="text-slate-600 dark:text-slate-300 mb-6">{message}</p>

        {error && (
          <p className="text-red-600 dark:text-red-400 text-sm mb-4">{error}</p>
        )}

        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={onCancel}
            disabled={isLoading}
            className="px-4 py-2 border border-slate-300 dark:border-slate-600 rounded hover:bg-slate-100 dark:hover:bg-slate-700 disabled:opacity-50"
          >
            {cancelLabel}
          </button>
          <button
            type="button"
            onClick={onConfirm}
            disabled={isLoading}
            className={`px-4 py-2 rounded disabled:opacity-50 ${confirmButtonClass}`}
          >
            {isLoading ? "Processing..." : confirmLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
