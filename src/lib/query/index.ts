/**
 * Query module exports
 *
 * This module provides centralized query key and query options factories
 * for TanStack Query, ensuring consistent cache key management and
 * type-safe query configurations across the application.
 *
 * Usage:
 *   import { queryKeys, repositoryQueries } from "@/lib/query";
 *
 *   // Using query options factory
 *   const query = useQuery(repositoryQueries.list());
 *
 *   // Using query keys for invalidation
 *   queryClient.invalidateQueries({ queryKey: queryKeys.repositories.all });
 */

export { queryKeys } from "./keys";
export {
  connectionQueries,
  settingsQueries,
  mcpServerQueries,
  repositoryQueries,
  issueQueries,
  pullQueries,
  jobQueries,
} from "./queries";
