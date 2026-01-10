import { useState } from "react";
import { useNavigate } from "@tanstack/react-router";
import type { Issue, PullRequest } from "@/types/models";
import { useStartAgent } from "@/hooks";

interface RunAgentDialogProps {
  isOpen: boolean;
  onClose: () => void;
  repositoryId: number;
  issue: Issue;
  relatedPrs: PullRequest[];
}

/**
 * Dialog for confirming agent execution with optional custom prompt input
 */
export function RunAgentDialog({
  isOpen,
  onClose,
  repositoryId,
  issue,
  relatedPrs,
}: RunAgentDialogProps) {
  const navigate = useNavigate();
  const startAgentMutation = useStartAgent();
  const [customPrompt, setCustomPrompt] = useState("");

  const hasOpenPr = relatedPrs.some((pr) => pr.state === "open");

  const handleSubmit = async () => {
    try {
      const response = await startAgentMutation.mutateAsync({
        repository_id: repositoryId,
        issue_number: issue.number,
        issue_title: issue.title,
        custom_prompt: customPrompt.trim() || undefined,
      });
      onClose();
      navigate({ to: "/jobs/$jobId", params: { jobId: String(response.job_id) } });
    } catch (error) {
      console.error("Failed to start agent:", error);
    }
  };

  if (!isOpen) return null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center">
      {/* Backdrop */}
      <div
        className="absolute inset-0 bg-black/50"
        onClick={onClose}
        onKeyDown={(e) => e.key === "Escape" && onClose()}
        role="button"
        tabIndex={0}
        aria-label="Close dialog"
      />

      {/* Dialog */}
      <div className="relative bg-white dark:bg-slate-800 rounded-lg shadow-xl max-w-lg w-full mx-4 p-6">
        <h2 className="text-xl font-bold mb-4">Run Agent</h2>

        {/* Issue Summary */}
        <div className="mb-4 p-3 bg-slate-50 dark:bg-slate-900 rounded">
          <div className="flex items-center gap-2 mb-1">
            <span className="text-gray-500 dark:text-gray-400">#{issue.number}</span>
            <span
              className={`px-2 py-0.5 rounded text-xs font-medium ${
                issue.state === "open"
                  ? "text-green-700 dark:text-green-300 bg-green-100 dark:bg-green-900"
                  : "text-purple-700 dark:text-purple-300 bg-purple-100 dark:bg-purple-900"
              }`}
            >
              {issue.state}
            </span>
          </div>
          <p className="font-medium">{issue.title}</p>
        </div>

        {/* Warning for existing PRs */}
        {hasOpenPr && (
          <div className="mb-4 p-3 bg-yellow-50 dark:bg-yellow-900/30 border border-yellow-200 dark:border-yellow-800 rounded">
            <p className="text-yellow-800 dark:text-yellow-200 text-sm font-medium">
              Warning: This issue already has an open pull request.
            </p>
            <p className="text-yellow-700 dark:text-yellow-300 text-sm mt-1">
              Running the agent may create duplicate work. Consider reviewing the existing PR first.
            </p>
            <ul className="mt-2 text-sm text-yellow-700 dark:text-yellow-300">
              {relatedPrs
                .filter((pr) => pr.state === "open")
                .map((pr) => (
                  <li key={pr.number}>
                    PR #{pr.number}: {pr.title}
                  </li>
                ))}
            </ul>
          </div>
        )}

        {/* Custom Prompt Input */}
        <div className="mb-6">
          <label
            htmlFor="custom-prompt"
            className="block text-sm font-medium text-gray-700 dark:text-gray-300 mb-2"
          >
            Additional Instructions (Optional)
          </label>
          <textarea
            id="custom-prompt"
            value={customPrompt}
            onChange={(e) => setCustomPrompt(e.target.value)}
            placeholder="Enter any additional instructions or context for the agent..."
            rows={4}
            className="w-full px-3 py-2 border border-slate-300 dark:border-slate-600 rounded-md bg-white dark:bg-slate-900 text-gray-900 dark:text-gray-100 placeholder-gray-400 dark:placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent resize-none"
          />
          <p className="mt-1 text-xs text-gray-500 dark:text-gray-400">
            These instructions will be included in the agent&apos;s prompt alongside the issue details.
          </p>
        </div>

        {/* Error Display */}
        {startAgentMutation.error && (
          <div className="mb-4 p-3 bg-red-50 dark:bg-red-900/30 border border-red-200 dark:border-red-800 rounded">
            <p className="text-red-800 dark:text-red-200 text-sm">
              Error: {String(startAgentMutation.error)}
            </p>
          </div>
        )}

        {/* Actions */}
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={onClose}
            disabled={startAgentMutation.isPending}
            className="px-4 py-2 text-sm border border-slate-300 dark:border-slate-600 rounded hover:bg-gray-50 dark:hover:bg-slate-700 disabled:opacity-50"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSubmit}
            disabled={startAgentMutation.isPending}
            className="px-4 py-2 text-sm bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            {startAgentMutation.isPending ? "Starting..." : "Run Agent"}
          </button>
        </div>
      </div>
    </div>
  );
}
