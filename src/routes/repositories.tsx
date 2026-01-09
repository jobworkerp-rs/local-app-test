import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useState, type FormEvent } from "react";

export const Route = createFileRoute("/repositories")({
  component: RepositoriesPage,
});

interface Repository {
  id: number;
  mcp_server_name: string;
  platform: "GitHub" | "Gitea";
  base_url: string;
  name: string;
  url: string;
  owner: string;
  repo_name: string;
  local_path: string | null;
  last_synced_at: string | null;
  created_at: string;
  updated_at: string;
}

interface CreateRepositoryRequest {
  mcp_server_name: string;
  platform: "GitHub" | "Gitea";
  base_url: string;
  name: string;
  url: string;
  owner: string;
  repo_name: string;
  local_path: string | null;
}

interface McpServerInfo {
  name: string;
  description: string | null;
  runner_type: string;
}

function RepositoriesPage() {
  const queryClient = useQueryClient();
  const [showForm, setShowForm] = useState(false);

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
    },
  });

  const handleDelete = (id: number, name: string) => {
    if (window.confirm(`Delete repository "${name}"?`)) {
      deleteMutation.mutate(id);
    }
  };

  return (
    <div className="container mx-auto p-8">
      <div className="flex items-center gap-4 mb-6">
        <Link to="/" className="text-blue-600 hover:underline">
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
        <p>Loading repositories...</p>
      ) : repositoriesQuery.error ? (
        <p className="text-red-600">
          Error: {String(repositoriesQuery.error)}
        </p>
      ) : repositoriesQuery.data?.length === 0 ? (
        <p className="text-gray-500">
          No repositories registered. Click "Add Repository" to get started.
        </p>
      ) : (
        <div className="space-y-4">
          {repositoriesQuery.data?.map((repo) => (
            <RepositoryCard
              key={repo.id}
              repository={repo}
              onDelete={() => handleDelete(repo.id, repo.name)}
              isDeleting={deleteMutation.isPending}
            />
          ))}
        </div>
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
    <div className="border rounded-lg p-4 hover:shadow-md transition-shadow">
      <div className="flex justify-between items-start">
        <div>
          <h3 className="text-lg font-semibold">
            <a
              href={repository.url}
              target="_blank"
              rel="noopener noreferrer"
              className="text-blue-600 hover:underline"
            >
              {repository.owner}/{repository.repo_name}
            </a>
          </h3>
          <p className="text-sm text-gray-500">
            {repository.platform} &middot; MCP: {repository.mcp_server_name}
          </p>
          {repository.local_path && (
            <p className="text-sm text-gray-400 mt-1">
              Local: {repository.local_path}
            </p>
          )}
          {repository.last_synced_at && (
            <p className="text-xs text-gray-400 mt-1">
              Last synced: {new Date(repository.last_synced_at).toLocaleString()}
            </p>
          )}
        </div>
        <div className="flex gap-2">
          <button
            type="button"
            onClick={onDelete}
            disabled={isDeleting}
            className="px-3 py-1 text-sm text-red-600 border border-red-600 rounded hover:bg-red-50 disabled:opacity-50"
          >
            Delete
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

  const createMutation = useMutation({
    mutationFn: (request: CreateRepositoryRequest) =>
      invoke<Repository>("create_repository", { request }),
    onSuccess: () => {
      onSuccess();
    },
  });

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
      : formData.base_url.replace("/api/v1", "");

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

  return (
    <form onSubmit={handleSubmit} className="border rounded-lg p-6 mb-6 bg-gray-50">
      <h2 className="text-xl font-semibold mb-4">Add Repository</h2>

      <div className="grid grid-cols-2 gap-4 mb-4">
        <div>
          <label htmlFor="mcp_server_name" className="block text-sm font-medium mb-1">
            MCP Server
          </label>
          <select
            id="mcp_server_name"
            value={formData.mcp_server_name}
            onChange={(e) => setFormData({ ...formData, mcp_server_name: e.target.value })}
            className="w-full p-2 border rounded"
            required
          >
            <option value="">Select MCP Server</option>
            {mcpServers.map((server) => (
              <option key={server.name} value={server.name}>
                {server.name}
                {server.description ? ` - ${server.description}` : ""}
              </option>
            ))}
          </select>
        </div>

        <div>
          <label htmlFor="platform" className="block text-sm font-medium mb-1">
            Platform
          </label>
          <select
            id="platform"
            value={formData.platform}
            onChange={(e) => handlePlatformChange(e.target.value as "GitHub" | "Gitea")}
            className="w-full p-2 border rounded"
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
            className="w-full p-2 border rounded"
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
            className="w-full p-2 border rounded"
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
            className="w-full p-2 border rounded"
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
          className="w-full p-2 border rounded"
        />
      </div>

      {formData.url && (
        <p className="text-sm text-gray-500 mb-4">
          Repository URL: {formData.url}
        </p>
      )}

      <div className="flex gap-2">
        <button
          type="submit"
          disabled={createMutation.isPending}
          className="px-4 py-2 bg-green-600 text-white rounded hover:bg-green-700 disabled:opacity-50"
        >
          {createMutation.isPending ? "Creating..." : "Create"}
        </button>
      </div>

      {createMutation.isError && (
        <p className="text-red-600 mt-2">
          Error: {String(createMutation.error)}
        </p>
      )}
    </form>
  );
}
