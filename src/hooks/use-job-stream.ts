/**
 * Hook for streaming job output from Tauri backend
 *
 * This hook manages the connection to job stream events and accumulates
 * data chunks for display in the UI.
 */
import { useState, useEffect, useMemo, useCallback } from "react";
import {
  listenJobStream,
  toUint8Array,
  chunksToString,
  type StreamEvent,
} from "@/lib/tauri/events";

export type StreamStatus =
  | "idle"
  | "connecting"
  | "streaming"
  | "completed"
  | "error";

const DEFAULT_MAX_CHUNKS = 1000;

interface UseJobStreamOptions {
  /**
   * Maximum number of chunks to keep in memory.
   * Older chunks are discarded when this limit is exceeded.
   * Default: 1000
   */
  maxChunks?: number;
  /**
   * Whether to start listening immediately.
   * Default: true
   */
  enabled?: boolean;
}

interface UseJobStreamResult {
  /** Accumulated data chunks */
  chunks: Uint8Array[];
  /** Current stream status */
  status: StreamStatus;
  /** Error if status is 'error' */
  error: Error | null;
  /** Decoded text content from all chunks */
  text: string;
  /** Reset the stream state */
  reset: () => void;
}

/**
 * Hook to receive streaming job output via Tauri events
 *
 * @param jobId - The jobworkerp job ID to listen for (or null/undefined to disable)
 * @param options - Configuration options
 * @returns Stream state and utilities
 *
 * @example
 * ```tsx
 * function JobOutput({ jobId }: { jobId: string }) {
 *   const { text, status, error } = useJobStream(jobId);
 *
 *   if (status === 'error') {
 *     return <div>Error: {error?.message}</div>;
 *   }
 *
 *   return (
 *     <pre>
 *       {text}
 *       {status === 'streaming' && <span className="animate-pulse">▋</span>}
 *     </pre>
 *   );
 * }
 * ```
 */
export function useJobStream(
  jobId: string | null | undefined,
  options: UseJobStreamOptions = {}
): UseJobStreamResult {
  const { maxChunks = DEFAULT_MAX_CHUNKS, enabled = true } = options;

  const [chunks, setChunks] = useState<Uint8Array[]>([]);
  const [status, setStatus] = useState<StreamStatus>("idle");
  const [error, setError] = useState<Error | null>(null);

  const reset = useCallback(() => {
    setChunks([]);
    setStatus("idle");
    setError(null);
  }, []);

  useEffect(() => {
    if (!jobId || !enabled) {
      return;
    }

    setStatus("connecting");
    setChunks([]);
    setError(null);

    let unlisten: (() => void) | undefined;

    const handleEvent = (event: StreamEvent) => {
      switch (event.type) {
        case "Data":
          setStatus("streaming");
          setChunks((prev) => {
            const newChunks = [...prev, toUint8Array(event.data)];
            // Memory protection: keep only the most recent maxChunks entries
            return newChunks.slice(-maxChunks);
          });
          break;

        case "FinalCollected":
          // Replace all chunks with the final collected result
          setChunks([toUint8Array(event.data)]);
          setStatus("completed");
          break;

        case "End":
          setStatus("completed");
          break;
      }
    };

    listenJobStream(jobId, handleEvent)
      .then((fn) => {
        unlisten = fn;
      })
      .catch((err) => {
        setError(err instanceof Error ? err : new Error("Failed to listen"));
        setStatus("error");
      });

    return () => {
      unlisten?.();
    };
  }, [jobId, enabled, maxChunks]);

  // Memoize the decoded text
  const text = useMemo(() => chunksToString(chunks), [chunks]);

  return { chunks, status, error, text, reset };
}

/**
 * Simplified hook that only returns the text content
 *
 * This is more memory-efficient for long-running streams as it
 * appends text directly instead of accumulating chunks.
 */
export function useJobStreamText(
  jobId: string | null | undefined,
  options: { enabled?: boolean } = {}
): {
  text: string;
  status: StreamStatus;
  error: Error | null;
  reset: () => void;
} {
  const { enabled = true } = options;

  const [text, setText] = useState("");
  const [status, setStatus] = useState<StreamStatus>("idle");
  const [error, setError] = useState<Error | null>(null);
  const decoder = useMemo(() => new TextDecoder("utf-8"), []);

  const reset = useCallback(() => {
    setText("");
    setStatus("idle");
    setError(null);
  }, []);

  useEffect(() => {
    if (!jobId || !enabled) {
      return;
    }

    setStatus("connecting");
    setText("");
    setError(null);

    let unlisten: (() => void) | undefined;

    const handleEvent = (event: StreamEvent) => {
      switch (event.type) {
        case "Data":
          setStatus("streaming");
          setText((prev) => prev + decoder.decode(toUint8Array(event.data)));
          break;

        case "FinalCollected":
          setText(decoder.decode(toUint8Array(event.data)));
          setStatus("completed");
          break;

        case "End":
          setStatus("completed");
          break;
      }
    };

    listenJobStream(jobId, handleEvent)
      .then((fn) => {
        unlisten = fn;
      })
      .catch((err) => {
        setError(err instanceof Error ? err : new Error("Failed to listen"));
        setStatus("error");
      });

    return () => {
      unlisten?.();
    };
  }, [jobId, enabled, decoder]);

  return { text, status, error, reset };
}
