import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { useState, useEffect, type FormEvent } from "react";
import { settingsQueries, queryKeys } from "@/lib/query";
import { updateAppSettings, type UpdateAppSettingsRequest } from "@/lib/tauri/commands";

export const Route = createFileRoute("/settings")({
  component: SettingsPage,
});

function SettingsPage() {
  const queryClient = useQueryClient();

  const settingsQuery = useQuery(settingsQueries.app());

  const [formData, setFormData] = useState<UpdateAppSettingsRequest>({});
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
    mutationFn: updateAppSettings,
    onSuccess: () => {
      setIsFormDirty(false);
      queryClient.invalidateQueries({ queryKey: queryKeys.settings.all });
    },
  });

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    updateMutation.mutate(formData);
  };

  // Helper to update form and mark as dirty
  const updateFormField = <K extends keyof UpdateAppSettingsRequest>(
    field: K,
    value: UpdateAppSettingsRequest[K]
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
        <p className="text-slate-600 dark:text-slate-400">Loading settings...</p>
      </div>
    );
  }

  if (settingsQuery.error) {
    return (
      <div className="container mx-auto p-8">
        <p className="text-red-600 dark:text-red-400">
          Error loading settings: {String(settingsQuery.error)}
        </p>
      </div>
    );
  }

  return (
    <div className="container mx-auto p-8">
      <div className="flex items-center gap-4 mb-6">
        <Link to="/" className="text-blue-600 dark:text-blue-400 hover:underline">
          &larr; Back
        </Link>
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
            className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100"
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
            className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100"
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
            className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100"
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
            className="w-full p-2 border border-slate-300 dark:border-slate-600 rounded bg-white dark:bg-slate-700 text-slate-900 dark:text-slate-100"
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
          <p className="text-green-600 dark:text-green-400">Settings saved successfully!</p>
        )}
        {updateMutation.isError && (
          <p className="text-red-600 dark:text-red-400">
            Error: {String(updateMutation.error)}
          </p>
        )}
      </form>
    </div>
  );
}
