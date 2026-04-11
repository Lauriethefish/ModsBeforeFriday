import { AdbServerClient } from "@yume-chan/adb";
import { PromiseResolver } from "@yume-chan/async";
import { MaybeConsumable, ReadableStream as ExtraReadableStream } from "@yume-chan/stream-extra";
import { Log } from "./Logging";
import { IBridge } from "./BridgeFactory";
import { ReactNode } from "react";
import { delay } from "./utilities/delay";

declare global {
    /**
     * Global ambient type declarations for the MBF Launcher WebView bridge.
     *
     * These types are available to all TypeScript files in the web project
     * without any import statement.
     */

    // ---------------------------------------------------------------------------
    // Connection handle
    // ---------------------------------------------------------------------------

    /** Handle to an open ADB TCP connection proxied through the JS bridge. */
    interface MbfAdbConnection {
        /** Unique connection identifier (UUID). */
        readonly id: string;

        /** `true` once the connection has been closed from either side. */
        readonly closed: boolean;

        /**
         * Write raw bytes to the ADB connection.
         * @param data Bytes to send (Uint8Array or ArrayBuffer).
         * @returns Promise that resolves to `true` on success, `false` if the
         *          connection ID is unknown on the native side.
         */
        write(data: Uint8Array | ArrayBuffer): Promise<boolean>;

        /**
         * Register a callback for incoming data chunks.
         *
         * The callback receives one chunk per call.  It may be async; the bridge
         * awaits completion before acknowledging the chunk, so a slow
         * callback naturally throttles the data stream.
         *
         * @returns Unsubscribe function – call it to stop receiving data.
         */
        onData(callback: (data: Uint8Array) => void | Promise<void>): () => void;

        /**
         * Register a callback for connection close events.
         * Fires when `close()` is called by JS or the ADB server closes the connection.
         * @returns Unsubscribe function.
         */
        onClose(callback: () => void): () => void;

        /**
         * Close the connection.
         * Fires `onClose` listeners immediately, then sends the native close command.
         * Safe to call multiple times; subsequent calls are no-ops.
         */
        close(): Promise<void>;
    }

    // ---------------------------------------------------------------------------
    // Bridge singleton
    // ---------------------------------------------------------------------------

    /** MBF bridge API, available as `window.__mbfBridge` inside the WebView. */
    interface MbfBridge {
        /** Always `true` – confirms the bridge context. */
        readonly isAvailable: true;

        /**
         * `true` on all platforms.  On Android this assumes a custom adbd instance
         * is reachable on the configured ADB port.
         */
        readonly isAdbAvailable: boolean;

        /**
         * Open a new ADB TCP connection via the JS bridge.
         * @throws If ADB is not available or the connection fails.
         */
        connect(): Promise<MbfAdbConnection>;
    }

    // ---------------------------------------------------------------------------
    // Window augmentation
    // ---------------------------------------------------------------------------

    interface Window {
        /** MBF bridge – present only when running inside a WebView providing the bridge. */
        __mbfBridge?: MbfBridge;
    }
}

/**
 * Implements {@link AdbServerClient.ServerConnector} over the
 * {@link window.__mbfBridge} API, routing each ADB host-protocol connection
 * through the javascript bridge interface.
 */
export class MbfAdbServerConnector implements AdbServerClient.ServerConnector {
    async connect(): Promise<AdbServerClient.ServerConnection> {
        if (!window.__mbfBridge || !window.__mbfBridge.isAdbAvailable) {
            throw new Error("ADB bridge is not available");
        }
        
        // Open a new connection through the bridge API
        const conn = await window.__mbfBridge!.connect();

        // Set up a promise that resolves when the connection is closed.
        const closed = new PromiseResolver<undefined>();
        let closedResolved = false;

        const resolveClosed = async () => {
            if (!closedResolved) {
                closedResolved = true;
                conn.close().catch(err => Log.error("Error closing connection: " + err, err));
                closed.resolve(undefined);
            }
        };

        conn.onClose(resolveClosed);

        // Create a ReadableStream for incoming data.
        const readable = new ReadableStream<Uint8Array>({
            start(controller) {
                conn.onData(chunk => controller.enqueue(chunk));
                closed.promise.then(() => {
                    try {
                        controller.close();
                    } catch (e) { }
                });
            },
            cancel() {
                // Defensive: ensure closed is resolved if stream is cancelled
                resolveClosed();
            }
        }) as ExtraReadableStream<Uint8Array>;

        // Create a WritableStream for outgoing data.
        const writable = new MaybeConsumable.WritableStream<Uint8Array>({
            write(chunk): Promise<void> {
                if (closedResolved) {
                    throw new Error("Cannot write to closed connection");
                }
                return MaybeConsumable.tryConsume(chunk, data => conn.write(data)).then(() => { });
            },
            async close(): Promise<void> {
                // Defensive: ensure closed is resolved if writable is closed
                resolveClosed();
            },
            async abort(): Promise<void> {
                resolveClosed();
            },
        });

        // Return an object that implements the ServerConnection interface.
        return {
            readable,
            writable,
            closed: closed.promise,
            close: async () => {
                resolveClosed();
            }
        };
    }

    addReverseTunnel(): never {
        throw new Error("Reverse tunnels are not supported by MbfAdbServerConnector");
    }

    removeReverseTunnel(): never {
        throw new Error("Reverse tunnels are not supported by MbfAdbServerConnector");
    }

    clearReverseTunnels(): never {
        throw new Error("Reverse tunnels are not supported by MbfAdbServerConnector");
    }
}
let bridgePromise: Promise<boolean> | null = null;

function checkForBridge(): Promise<boolean> {
    if (bridgePromise) {
        return bridgePromise;
    }
    
    bridgePromise = new Promise((resolve) => {
        const resolver = new PromiseResolver<boolean>();
        const abort = new AbortController();
        const timeout = 1000;
        let resolved = false;
        
        Promise.race([
        Promise.resolve().then(async () => {
            for (let i = 0; true; i++) {
            abort.signal.throwIfAborted();
            
            // Check for the bridge every 100ms.
            if (window.__mbfBridge) {
                Log.debug(`Bridge detected! ${i * 100}ms elapsed`);
                resolved = true;
                resolver.resolve(true);
                return;
            }
            
            await delay(100, abort.signal);
            }
        }).then(() => abort.abort()),
        delay(timeout, abort.signal).then(() => {
            abort.abort();
            if (!resolved) {
            Log.debug(`No bridge detected after ${timeout / 1000}s, assuming it's not present.`);
            
            resolver.resolve(false);
            }
        })
        ]).catch(err => Log.debug("Bridge detection process aborted.", err));
        
        resolver.promise.then(resolve);
    });
    
    return bridgePromise;
}

export class MbfBridge implements IBridge {
    BridgeSupplement(): ReactNode {
        return null;
    }
    async isAvailable(abortController?: AbortController): Promise<boolean> {        
        return await checkForBridge();
    }
    async getConnector(): Promise<AdbServerClient.ServerConnector> {
        return new MbfAdbServerConnector();
    }
    
}