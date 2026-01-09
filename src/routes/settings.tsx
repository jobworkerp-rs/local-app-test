import { createFileRoute } from "@tanstack/react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { useState, useEffect, type FormEvent } from "react";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

interface AppSettings {
  id: number;
  worktree_base_path: string;
  default_base_branch: string;
  agent_timeout_minutes: number;
  sync_interval_minutes: number;
  created_at: string;
  updated_at: string;
}

interface UpdateSettingsRequest {
  worktree_base_path?: string;
  default_base_branch?: string;
  agent_timeout_minutes?: number;
  sync_interval_minutes?: number;
}

function SettingsPage() {
  const queryClient = useQueryClient();

  const settingsQuery = useQuery({
    queryKey: ["app-settings"],
    queryFn: () => invoke<AppSettings>("get_app_settings"),
  });

  const [formData, setFormData] = useState<UpdateSettingsRequest>({});
  const [isFormDirty, setIsFormDirty] = useState(false);

  useEffect(() => {
    // Only update form from server data if form is not dirty (user hasn't edited)
    if (settingsQuery.data && !isFormDirty) {
      setFormData({
        worktree_base_path: settingsQuery.data.worktree_base_path,
        default_base_branch: settingsQuery.data.default_base_branch,
        agent_timeout_minutes: settingsQuery.data.agent_timeout_minutes,
        sync_interval_minutes: settingsQuery.data.sync_interval_minutes,
      });
    }
  }, [settingsQuery.data, isFormDirty]);

  const updateMutation = useMutation({
    mutationFn: (request: UpdateSettingsRequest) =>
      invoke<AppSettings>("update_app_settings", { request }),
    onSuccess: () => {
      setIsFormDirty(false);
      queryClient.invalidateQueries({ queryKey: ["app-settings"] });
    },
  });

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    updateMutation.mutate(formData);
  };

  // Helper to update form and mark as dirty
  const updateFormField = <K extends keyof UpdateSettingsRequest>(
    field: K,
    value: UpdateSettingsRequest[K]
  ) => {
    setFormData({ ...formData, [field]: value });
    setIsFormDirty(true);
  };

  // Parse and validate numeric input
  const handleNumericChange = (
    field: "agent_timeout_minutes" | "sync_interval_minutes",
    value: string
  ) => {
    if (value === "") {
      updateFormField(field, undefined);
      return;
    }
    const parsed = Number(value);
    if (Number.isFinite(parsed) && parsed >= 0) {
      updateFormField(field, parsed);
    }
    // Invalid input is ignored (keeps previous value)
  };

  if (settingsQuery.isLoading) {
    return (
      <div className="container mx-auto p-8">
        <p>Loading settings...</p>
      </div>
    );
  }

  if (settingsQuery.error) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-red-600">
          Error loading settings: {String(settingsQuery.error)}
        </p>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-8">
      <div className="flex items-center gap-4 mb-6">
        <a href="/" className="text-blue-600 hover:underline">
          &larr; Back
        </a>
        <h1 className="text-3xl font-bold">Settings</h1>
      </div>

      <form onSubmit={handleSubmit} className="space-y-6 max-w-md">
        <div>
          <label
            htmlFor="worktree_base_path"
            className="block text-sm font-medium mb-1"
          >
            Worktree Base Path
          </label>
          <input
            id="worktree_base_path"
            type="text"
            value={formData.worktree_base_path ?? ""}
            onChange={(e) =>
              updateFormField("worktree_base_path", e.target.value)
            }
            className="w-full p-2 border rounded"
          />
        </div>

        <div>
          <label
            htmlFor="default_base_branch"
            className="block text-sm font-medium mb-1"
          >
            Default Base Branch
          </label>
          <input
            id="default_base_branch"
            type="text"
            value={formData.default_base_branch ?? ""}
            onChange={(e) =>
              updateFormField("default_base_branch", e.target.value)
            }
            className="w-full p-2 border rounded"
          />
        </div>

        <div>
          <label
            htmlFor="agent_timeout_minutes"
            className="block text-sm font-medium mb-1"
          >
            Agent Timeout (minutes)
          </label>
          <input
            id="agent_timeout_minutes"
            type="number"
            min="0"
            value={formData.agent_timeout_minutes ?? ""}
            onChange={(e) =>
              handleNumericChange("agent_timeout_minutes", e.target.value)
            }
            className="w-full p-2 border rounded"
          />
        </div>

        <div>
          <label
            htmlFor="sync_interval_minutes"
            className="block text-sm font-medium mb-1"
          >
            Sync Interval (minutes)
          </label>
          <input
            id="sync_interval_minutes"
            type="number"
            min="0"
            value={formData.sync_interval_minutes ?? ""}
            onChange={(e) =>
              handleNumericChange("sync_interval_minutes", e.target.value)
            }
            className="w-full p-2 border rounded"
          />
        </div>

        <button
          type="submit"
          disabled={updateMutation.isPending}
          className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
        >
          {updateMutation.isPending ? "Saving..." : "Save Settings"}
        </button>

        {updateMutation.isSuccess && (
          <p className="text-green-600">Settings saved successfully!</p>
        )}
        {updateMutation.isError && (
          <p className="text-red-600">
            Error: {String(updateMutation.error)}
          </p>
        )}
      </form>
    </div>
  );
}
