import { createFileRoute } from "@tanstack/react-router";
import { useQuery } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";

export const Route = createFileRoute("/")({
  component: HomePage,
});

interface ConnectionStatus {
  connected: boolean;
  error?: string;
}

function HomePage() {
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
    <div className="container mx-auto p-8">
      <h1 className="text-3xl font-bold mb-6">Local Code Agent</h1>

      <div className="space-y-4">
        <section className="p-4 border rounded-lg">
          <h2 className="text-xl font-semibold mb-2">Connection Status</h2>
          {connectionQuery.isLoading ? (
            <p className="text-muted-foreground">Checking connection...</p>
          ) : connectionQuery.data?.connected ? (
            <p className="text-green-600">Connected to jobworkerp-rs</p>
          ) : (
            <p className="text-red-600">
              Not connected
              {connectionQuery.data?.error && `: ${connectionQuery.data.error}`}
            </p>
          )}
        </section>

        <nav className="space-y-2">
          <a
            href="/settings"
            className="block p-3 border rounded hover:bg-accent transition-colors"
          >
            Settings
          </a>
          <a
            href="/repositories"
            className="block p-3 border rounded hover:bg-accent transition-colors"
          >
            Repositories
          </a>
          <a
            href="/jobs"
            className="block p-3 border rounded hover:bg-accent transition-colors"
          >
            Agent Jobs
          </a>
        </nav>
      </div>
    </div>
  );
}
