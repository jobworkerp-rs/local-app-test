/**
 * Settings-related React Query hooks
 *
 * These hooks provide access to application settings
 * with automatic caching and updates.
 */
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import {
  getAppSettings,
  updateAppSettings,
  checkJobworkerpConnection,
  type UpdateAppSettingsRequest,
} from "@/lib/tauri/commands";

// ============================================================================
// Query Keys
// ============================================================================

export const settingsKeys = {
  all: ["settings"] as const,
  app: () => [...settingsKeys.all, "app"] as const,
  connection: () => [...settingsKeys.all, "connection"] as const,
};

// ============================================================================
// Settings Hooks
// ============================================================================

/**
 * Fetch application settings
 */
export function useAppSettings() {
  return useQuery({
    queryKey: settingsKeys.app(),
    queryFn: getAppSettings,
    staleTime: 60_000,
  });
}

/**
 * Update application settings
 */
export function useUpdateAppSettings() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (settings: UpdateAppSettingsRequest) =>
      updateAppSettings(settings),
    onSuccess: (updatedSettings) => {
      queryClient.setQueryData(settingsKeys.app(), updatedSettings);
    },
  });
}

// ============================================================================
// Connection Hooks
// ============================================================================

/**
 * Check jobworkerp-rs backend connection status
 */
export function useJobworkerpConnection() {
  return useQuery({
    queryKey: settingsKeys.connection(),
    queryFn: checkJobworkerpConnection,
    staleTime: 30_000,
    retry: 1,
  });
}

/**
 * Check connection with periodic refresh
 */
export function useJobworkerpConnectionWithPolling(pollInterval = 30_000) {
  return useQuery({
    queryKey: settingsKeys.connection(),
    queryFn: checkJobworkerpConnection,
    refetchInterval: pollInterval,
    staleTime: 10_000,
    retry: 1,
  });
}
