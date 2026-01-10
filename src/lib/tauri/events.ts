/**
 * Tauri event subscription utilities
 *
 * This module provides typed wrappers for Tauri event listening,
 * particularly for streaming job results from the Rust backend.
 */
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

// ============================================================================
// Stream Event Types
// ============================================================================

/**
 * Streaming data chunk event
 */
export interface StreamDataEvent {
  type: "Data";
  data: number[]; // Rust Vec<u8> is received as number[]
}

/**
 * Stream end marker event
 */
export interface StreamEndEvent {
  type: "End";
}

/**
 * Final collected result event (for workflow completion)
 */
export interface StreamFinalCollectedEvent {
  type: "FinalCollected";
  status: string;
  pr_number?: number;
  pr_url?: string;
}

/**
 * Stream error event
 */
export interface StreamErrorEvent {
  type: "Error";
  message: string;
}

/**
 * Union type for all stream events
 */
export type StreamEvent =
  | StreamDataEvent
  | StreamEndEvent
  | StreamFinalCollectedEvent
  | StreamErrorEvent;

// ============================================================================
// Event Listeners
// ============================================================================

/**
 * Listen to job stream events for a specific job ID
 *
 * @param jobId - The jobworkerp job ID to listen for
 * @param callback - Function called when stream events arrive
 * @returns Promise that resolves to an unlisten function
 *
 * @example
 * ```typescript
 * const unlisten = await listenJobStream('123', (event) => {
 *   if (event.type === 'Data') {
 *     console.log('Received data:', event.data);
 *   } else if (event.type === 'End') {
 *     console.log('Stream ended');
 *   }
 * });
 *
 * // Later, to stop listening:
 * unlisten();
 * ```
 */
export function listenJobStream(
  jobId: number,
  callback: (event: StreamEvent) => void
): Promise<UnlistenFn> {
  return listen<StreamEvent>(`job-stream-${jobId}`, (event) => {
    callback(event.payload);
  });
}

/**
 * Listen to job status change events for a specific job ID
 *
 * @param jobId - The local job ID to listen for
 * @param callback - Function called when status changes
 * @returns Promise that resolves to an unlisten function
 */
export function listenJobStatus(
  jobId: number,
  callback: (status: string) => void
): Promise<UnlistenFn> {
  return listen<string>(`job-status-${jobId}`, (event) => {
    callback(event.payload);
  });
}

// ============================================================================
// Utility Functions
// ============================================================================

/**
 * Convert a number array (Rust Vec<u8>) to a Uint8Array
 */
export function toUint8Array(data: number[]): Uint8Array {
  return new Uint8Array(data);
}

/**
 * Decode a number array (Rust Vec<u8>) to a UTF-8 string
 */
export function decodeUtf8(data: number[]): string {
  const decoder = new TextDecoder("utf-8");
  return decoder.decode(toUint8Array(data));
}

/**
 * Convert accumulated stream chunks to a single string
 */
export function chunksToString(chunks: Uint8Array[]): string {
  const decoder = new TextDecoder("utf-8");
  return chunks.map((chunk) => decoder.decode(chunk)).join("");
}
