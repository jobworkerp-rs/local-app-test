import { createFileRoute, Link } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { FolderGit2, Bot, ArrowRight } from "lucide-react";
import type { Repository, AgentJob } from "@/types/models";

export const Route = createFileRoute("/")({
  component: HomePage,
});

function HomePage() {
  const reposQuery = useQuery({
    queryKey: ["repositories"],
    queryFn: () => invoke<Repository[]>("list_repositories"),
  });

  const jobsQuery = useQuery({
    queryKey: ["jobs"],
    queryFn: () => invoke<AgentJob[]>("list_jobs"),
  });

  const recentJobs = jobsQuery.data?.slice(0, 5) ?? [];
  const repoCount = reposQuery.data?.length ?? 0;

  return (
    <div className="space-y-6">
      <h1 className="text-2xl font-bold">Dashboard</h1>

      {/* Stats */}
      <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
        <div className="p-4 bg-white dark:bg-slate-800 rounded-lg border border-slate-200 dark:border-slate-700 shadow-sm">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-blue-100 dark:bg-blue-900 rounded-lg">
              <FolderGit2 className="w-5 h-5 text-blue-600 dark:text-blue-400" />
            </div>
            <div>
              <p className="text-sm text-slate-500 dark:text-slate-400">Repositories</p>
              <p className="text-2xl font-bold">{repoCount}</p>
            </div>
          </div>
        </div>

        <div className="p-4 bg-white dark:bg-slate-800 rounded-lg border border-slate-200 dark:border-slate-700 shadow-sm">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-green-100 dark:bg-green-900 rounded-lg">
              <Bot className="w-5 h-5 text-green-600 dark:text-green-400" />
            </div>
            <div>
              <p className="text-sm text-slate-500 dark:text-slate-400">Total Jobs</p>
              <p className="text-2xl font-bold">{jobsQuery.data?.length ?? 0}</p>
            </div>
          </div>
        </div>

        <div className="p-4 bg-white dark:bg-slate-800 rounded-lg border border-slate-200 dark:border-slate-700 shadow-sm">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-purple-100 dark:bg-purple-900 rounded-lg">
              <Bot className="w-5 h-5 text-purple-600 dark:text-purple-400" />
            </div>
            <div>
              <p className="text-sm text-slate-500 dark:text-slate-400">Active Jobs</p>
              <p className="text-2xl font-bold">
                {jobsQuery.data?.filter(
                  (j) => !["Completed", "Failed", "Cancelled"].includes(j.status)
                ).length ?? 0}
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Recent Jobs */}
      <div className="bg-white dark:bg-slate-800 rounded-lg border border-slate-200 dark:border-slate-700 shadow-sm">
        <div className="p-4 border-b border-slate-200 dark:border-slate-700 flex justify-between items-center">
          <h2 className="font-semibold">Recent Agent Jobs</h2>
          <Link
            to="/jobs"
            className="text-sm text-blue-600 dark:text-blue-400 hover:underline flex items-center gap-1"
          >
            View all <ArrowRight className="w-4 h-4" />
          </Link>
        </div>
        <div className="divide-y divide-slate-200 dark:divide-slate-700">
          {recentJobs.length === 0 ? (
            <p className="p-4 text-slate-500 dark:text-slate-400 text-sm">No jobs yet</p>
          ) : (
            recentJobs.map((job) => (
              <Link
                key={job.id}
                to="/jobs/$jobId"
                params={{ jobId: String(job.id) }}
                className="p-4 flex justify-between items-center hover:bg-slate-50 dark:hover:bg-slate-700"
              >
                <div>
                  <p className="font-medium">Issue #{job.issue_number}</p>
                  <p className="text-sm text-slate-500 dark:text-slate-400">
                    Job ID: {job.jobworkerp_job_id.slice(0, 8)}...
                  </p>
                </div>
                <span
                  className={`px-2 py-1 text-xs rounded-full ${
                    job.status === "Completed"
                      ? "bg-green-100 dark:bg-green-900 text-green-700 dark:text-green-300"
                      : job.status === "Failed"
                        ? "bg-red-100 dark:bg-red-900 text-red-700 dark:text-red-300"
                        : job.status === "Cancelled"
                          ? "bg-gray-100 dark:bg-gray-800 text-gray-700 dark:text-gray-300"
                          : "bg-blue-100 dark:bg-blue-900 text-blue-700 dark:text-blue-300"
                  }`}
                >
                  {job.status}
                </span>
              </Link>
            ))
          )}
        </div>
      </div>

      {/* Quick Actions */}
      <div className="bg-white dark:bg-slate-800 rounded-lg border border-slate-200 dark:border-slate-700 shadow-sm p-4">
        <h2 className="font-semibold mb-3">Quick Actions</h2>
        <div className="flex gap-3">
          <Link
            to="/repositories"
            className="px-4 py-2 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors"
          >
            Add Repository
          </Link>
          <Link
            to="/settings"
            className="px-4 py-2 border border-slate-300 dark:border-slate-600 rounded-lg hover:bg-slate-50 dark:hover:bg-slate-700 transition-colors"
          >
            Configure Settings
          </Link>
        </div>
      </div>
    </div>
  );
}
