import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import { Circle } from "lucide-react";

interface ConnectionStatus {
  connected: boolean;
  error?: string;
}

export function Header() {
  const connectionQuery = useQuery({
    queryKey: ["jobworkerp-connection"],
    queryFn: async (): Promise<ConnectionStatus> => {
      try {
        const connected = await invoke<boolean>("check_jobworkerp_connection");
        return { connected };
      } catch (error) {
        return { connected: false, error: String(error) };
      }
    },
    refetchInterval: 30000,
  });

  return (
    <header className="h-14 border-b bg-white flex items-center justify-between px-6">
      <div>{/* Breadcrumbs or page title can be added here */}</div>

      <div className="flex items-center gap-4">
        {/* Connection Status */}
        <div className="flex items-center gap-2 text-sm">
          <Circle
            className={`w-3 h-3 ${
              connectionQuery.isLoading
                ? "fill-yellow-400 text-yellow-400"
                : connectionQuery.data?.connected
                  ? "fill-green-500 text-green-500"
                  : "fill-red-500 text-red-500"
            }`}
          />
          <span className="text-slate-600">
            {connectionQuery.isLoading
              ? "Connecting..."
              : connectionQuery.data?.connected
                ? "Connected"
                : "Disconnected"}
          </span>
        </div>
      </div>
    </header>
  );
}
