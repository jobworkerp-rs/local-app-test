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

/** Final result from completed workflow */
export interface WorkflowResult {
  status: string;
  pr_number?: number;
  pr_url?: string;
}

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
  /**
   * Callback when workflow completes with final result
   */
  onComplete?: (result: WorkflowResult) => void;
  /**
   * Callback when an error event is received
   */
  onError?: (message: string) => void;
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
  /** Final workflow result (if completed) */
  result: WorkflowResult | null;
  /** Reset the stream state */
  reset: () => void;
}

/**
 * Hook to receive streaming job output via Tauri events
 *
 * @param jobId - The local job ID to listen for (or null/undefined to disable)
 * @param options - Configuration options
 * @returns Stream state and utilities
 *
 * @example
 * ```tsx
 * function JobOutput({ jobId }: { jobId: number }) {
 *   const { text, status, error, result } = useJobStream(jobId, {
 *     onComplete: (result) => {
 *       if (result.pr_url) {
 *         console.log('PR created:', result.pr_url);
 *       }
 *     },
 *   });
 *
 *   if (status === 'error') {
 *     return <div>Error: {error?.message}</div>;
 *   }
 *
 *   return (
 *     <pre>
 *       {text}
 *       {status === 'streaming' && <span className="animate-pulse">...</span>}
 *     </pre>
 *   );
 * }
 * ```
 */
export function useJobStream(
  jobId: number | null | undefined,
  options: UseJobStreamOptions = {}
): UseJobStreamResult {
  const { maxChunks = DEFAULT_MAX_CHUNKS, enabled = true, onComplete, onError } = options;

  const [chunks, setChunks] = useState<Uint8Array[]>([]);
  const [status, setStatus] = useState<StreamStatus>("idle");
  const [error, setError] = useState<Error | null>(null);
  const [result, setResult] = useState<WorkflowResult | null>(null);

  const reset = useCallback(() => {
    setChunks([]);
    setStatus("idle");
    setError(null);
    setResult(null);
  }, []);

  useEffect(() => {
    if (jobId === null || jobId === undefined || !enabled) {
      return;
    }

    setStatus("connecting");
    setChunks([]);
    setError(null);
    setResult(null);

    let unlisten: (() => void) | undefined;
    let mounted = true;

    const handleEvent = (event: StreamEvent) => {
      switch (event.type) {
        case "Data":
          setStatus("streaming");
          setChunks((prev) => {
            const newChunks = [...prev, toUint8Array(event.data)];
            return newChunks.slice(-maxChunks);
          });
          break;

        case "FinalCollected": {
          const workflowResult: WorkflowResult = {
            status: event.status,
            pr_number: event.pr_number,
            pr_url: event.pr_url,
          };
          setResult(workflowResult);
          setStatus("completed");
          onComplete?.(workflowResult);
          break;
        }

        case "End":
          setStatus("completed");
          break;

        case "Error":
          setError(new Error(event.message));
          setStatus("error");
          onError?.(event.message);
          break;
      }
    };

    listenJobStream(jobId, handleEvent)
      .then((fn) => {
        if (!mounted) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch((err) => {
        setError(err instanceof Error ? err : new Error("Failed to listen"));
        setStatus("error");
      });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [jobId, enabled, maxChunks, onComplete, onError]);

  const text = useMemo(() => chunksToString(chunks), [chunks]);

  return { chunks, status, error, text, result, reset };
}

/**
 * Simplified hook that only returns the text content
 *
 * This is more memory-efficient for long-running streams as it
 * appends text directly instead of accumulating chunks.
 */
export function useJobStreamText(
  jobId: number | null | undefined,
  options: { enabled?: boolean; onComplete?: (result: WorkflowResult) => void; onError?: (message: string) => void } = {}
): {
  text: string;
  status: StreamStatus;
  error: Error | null;
  result: WorkflowResult | null;
  reset: () => void;
} {
  const { enabled = true, onComplete, onError } = options;

  const [text, setText] = useState("");
  const [status, setStatus] = useState<StreamStatus>("idle");
  const [error, setError] = useState<Error | null>(null);
  const [result, setResult] = useState<WorkflowResult | null>(null);

  const reset = useCallback(() => {
    setText("");
    setStatus("idle");
    setError(null);
    setResult(null);
  }, []);

  useEffect(() => {
    if (jobId === null || jobId === undefined || !enabled) {
      return;
    }

    setStatus("connecting");
    setText("");
    setError(null);
    setResult(null);

    let unlisten: (() => void) | undefined;
    let mounted = true;
    const streamDecoder = new TextDecoder("utf-8");

    const handleEvent = (event: StreamEvent) => {
      switch (event.type) {
        case "Data":
          setStatus("streaming");
          setText(
            (prev) =>
              prev +
              streamDecoder.decode(toUint8Array(event.data), { stream: true })
          );
          break;

        case "FinalCollected": {
          const workflowResult: WorkflowResult = {
            status: event.status,
            pr_number: event.pr_number,
            pr_url: event.pr_url,
          };
          setResult(workflowResult);
          setStatus("completed");
          onComplete?.(workflowResult);
          break;
        }

        case "End":
          setText((prev) => {
            const remaining = streamDecoder.decode(new Uint8Array(), {
              stream: false,
            });
            return remaining ? prev + remaining : prev;
          });
          setStatus("completed");
          break;

        case "Error":
          setError(new Error(event.message));
          setStatus("error");
          onError?.(event.message);
          break;
      }
    };

    listenJobStream(jobId, handleEvent)
      .then((fn) => {
        if (!mounted) {
          // Component unmounted before promise resolved - cleanup immediately
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch((err) => {
        setError(err instanceof Error ? err : new Error("Failed to listen"));
        setStatus("error");
      });

    return () => {
      mounted = false;
      unlisten?.();
    };
  }, [jobId, enabled, onComplete, onError]);

  return { text, status, error, result, reset };
}
