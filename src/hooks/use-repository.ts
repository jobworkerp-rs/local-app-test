/**
 * Repository-related React Query hooks
 *
 * These hooks provide cached, reactive access to repository data
 * with automatic refetching and optimistic updates.
 */
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  listRepositories,
  getRepository,
  createRepository,
  deleteRepository,
  listIssues,
  getIssue,
  listPulls,
  findRelatedPrs,
} from "@/lib/tauri/commands";
import type { CreateRepositoryRequest } from "@/types/models";

// ============================================================================
// Query Keys
// ============================================================================

export const repositoryKeys = {
  all: ["repositories"] as const,
  lists: () => [...repositoryKeys.all, "list"] as const,
  list: () => [...repositoryKeys.lists()] as const,
  details: () => [...repositoryKeys.all, "detail"] as const,
  detail: (id: number) => [...repositoryKeys.details(), id] as const,
  issues: (repoId: number) => [...repositoryKeys.detail(repoId), "issues"] as const,
  issue: (repoId: number, issueNumber: number) =>
    [...repositoryKeys.issues(repoId), issueNumber] as const,
  pulls: (repoId: number) => [...repositoryKeys.detail(repoId), "pulls"] as const,
  relatedPrs: (repoId: number, issueNumber: number) =>
    [...repositoryKeys.issues(repoId), issueNumber, "related-prs"] as const,
};

// ============================================================================
// Repository Hooks
// ============================================================================

/**
 * Fetch all repositories
 */
export function useRepositories() {
  return useQuery({
    queryKey: repositoryKeys.list(),
    queryFn: listRepositories,
    staleTime: 30_000,
  });
}

/**
 * Fetch a single repository by ID
 */
export function useRepository(repositoryId: number | undefined) {
  return useQuery({
    queryKey: repositoryKeys.detail(repositoryId ?? 0),
    queryFn: () => getRepository(repositoryId!),
    enabled: repositoryId !== undefined && repositoryId > 0,
    staleTime: 30_000,
  });
}

/**
 * Create a new repository
 */
export function useCreateRepository() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: CreateRepositoryRequest) => createRepository(request),
    onSuccess: (newRepo) => {
      queryClient.invalidateQueries({ queryKey: repositoryKeys.lists() });
      queryClient.setQueryData(repositoryKeys.detail(newRepo.id), newRepo);
    },
  });
}

/**
 * Delete a repository
 */
export function useDeleteRepository() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: number) => deleteRepository(id),
    onSuccess: (_data, id) => {
      queryClient.invalidateQueries({ queryKey: repositoryKeys.lists() });
      queryClient.removeQueries({ queryKey: repositoryKeys.detail(id) });
    },
  });
}

// ============================================================================
// Issue Hooks
// ============================================================================

/**
 * Fetch issues for a repository
 */
export function useIssues(
  repositoryId: number | undefined,
  state: "open" | "closed" | "all" = "open"
) {
  return useQuery({
    queryKey: [...repositoryKeys.issues(repositoryId ?? 0), state],
    queryFn: () => listIssues(repositoryId!, state),
    enabled: repositoryId !== undefined && repositoryId > 0,
    staleTime: 60_000,
  });
}

/**
 * Fetch a single issue
 */
export function useIssue(
  repositoryId: number | undefined,
  issueNumber: number | undefined
) {
  return useQuery({
    queryKey: repositoryKeys.issue(repositoryId ?? 0, issueNumber ?? 0),
    queryFn: () => getIssue(repositoryId!, issueNumber!),
    enabled:
      repositoryId !== undefined &&
      repositoryId > 0 &&
      issueNumber !== undefined &&
      issueNumber > 0,
    staleTime: 60_000,
  });
}

// ============================================================================
// Pull Request Hooks
// ============================================================================

/**
 * Fetch pull requests for a repository
 */
export function usePullRequests(
  repositoryId: number | undefined,
  state: "open" | "closed" | "all" = "open"
) {
  return useQuery({
    queryKey: [...repositoryKeys.pulls(repositoryId ?? 0), state],
    queryFn: () => listPulls(repositoryId!, state),
    enabled: repositoryId !== undefined && repositoryId > 0,
    staleTime: 60_000,
  });
}

/**
 * Fetch pull requests related to a specific issue
 */
export function useRelatedPullRequests(
  repositoryId: number | undefined,
  issueNumber: number | undefined
) {
  return useQuery({
    queryKey: repositoryKeys.relatedPrs(repositoryId ?? 0, issueNumber ?? 0),
    queryFn: () => findRelatedPrs(repositoryId!, issueNumber!),
    enabled:
      repositoryId !== undefined &&
      repositoryId > 0 &&
      issueNumber !== undefined &&
      issueNumber > 0,
    staleTime: 60_000,
  });
}
