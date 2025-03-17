import type { AdbIncomingSocketHandler, AdbServerClient } from "@yume-chan/adb";
import { MaybeConsumable, ReadableStream, ReadableWritablePair } from "@yume-chan/stream-extra";
import { PromiseResolver } from "@yume-chan/async";

/** WebSocket bridge endpoints. */
const bridge = new URLSearchParams(window.location.search).get("bridge") || "127.0.0.1:25037";
export const bridgeWebsocketAddress = `ws://${bridge}/bridge`;
export const bridgePingAddress = `http://${bridge}/bridge/ping`;

/**
 * Checks if the bridge is running by sending a GET request to the ping endpoint.
 *
 * @returns A promise that resolves to true if the bridge is running, false otherwise.
 */
export async function checkForBridge(): Promise<boolean> {
  try {
    const response = await fetch(bridgePingAddress);
    return response.ok;
  } catch {
    return false;
  }
}

/**
 * Interface representing a socket with readable and writable streams,
 * along with connection details.
 */
interface Socket extends ReadableWritablePair<Uint8Array, Uint8Array> {
  extensions: string;
  protocol: string;
}

/**
 * Wraps a WebSocket connection into readable and writable streams.
 */
class WebSocketConnection {
  public url: string;
  private socket: WebSocket;
  private openDeferred: PromiseResolver<Socket>;
  private closeDeferred: PromiseResolver<{ closeCode: number; reason: string }>;

  /**
   * Initializes a new WebSocket connection.
   *
   * @param url - The WebSocket URL.
   * @param options - Optional protocols.
   */
  constructor(url: string, options?: { protocols?: string | string[] }) {
    this.url = url;
    this.socket = new WebSocket(url, options?.protocols);
    this.socket.binaryType = "arraybuffer";
    this.openDeferred = new PromiseResolver<Socket>();
    this.closeDeferred = new PromiseResolver<{ closeCode: number; reason: string }>();

    let hasOpened = false;

    // When the socket opens, resolve the openDeferred with connection details.
    this.socket.onopen = () => {
      hasOpened = true;
      this.openDeferred.resolve({
        extensions: this.socket.extensions,
        protocol: this.socket.protocol,
        readable: new ReadableStream<Uint8Array>({
          start: (controller) => {
            // Forward incoming messages to the stream controller.
            this.socket.onmessage = (event: MessageEvent) => {
              if (typeof event.data === "string") {
                controller.enqueue(new TextEncoder().encode(event.data));
              } else {
                controller.enqueue(new Uint8Array(event.data));
              }
            };
            // Report errors to the stream controller.
            this.socket.onerror = () => controller.error(new Error("WebSocket error"));
            // Close the stream when the socket closes.
            this.socket.onclose = (event) => {
              try {
                controller.close();
              } catch {
                // Ignore errors during stream close.
              }
              this.closeDeferred.resolve({
                closeCode: event.code,
                reason: event.reason,
              });
            };
          },
        }),
        writable: new MaybeConsumable.WritableStream<Uint8Array>({
          write: async (chunk: Uint8Array) => {
            this.socket.send(chunk);
          },
        }),
      });
    };

    // If an error occurs before the socket opens, reject the openDeferred.
    this.socket.onerror = () => {
      if (!hasOpened) {
        this.openDeferred.reject(new Error("WebSocket connection error"));
      }
    };
  }

  /** Returns a promise that resolves when the connection is open. */
  public getOpened(): Promise<Socket> {
    return this.openDeferred.promise;
  }

  /** Returns a promise that resolves when the connection is closed. */
  public getClosed(): Promise<{ closeCode: number; reason: string }> {
    return this.closeDeferred.promise;
  }

  /** Closes the WebSocket connection. */
  public close(closeInfo?: { closeCode?: number; reason?: string }): void {
    this.socket.close(closeInfo?.closeCode, closeInfo?.reason);
  }
}

/**
 * A `AdbServerClient.ServerConnector` implementation using a WebSocket connection.
 */
export class AdbServerWebSocketConnector implements AdbServerClient.ServerConnector {
  constructor() { }

  /**
   * Connects to the ADB server bridge using a WebSocket connection.
   *
   * @returns A promise that resolves to the ADB server connection.
   */
  async connect(): Promise<AdbServerClient.ServerConnection> {
    const connection = new WebSocketConnection(bridgeWebsocketAddress);
    let timer: ReturnType<typeof setTimeout> | undefined = undefined;

    // Create a timeout promise that rejects after 5000ms.
    const timeoutPromise = new Promise<never>((_, reject) => {
      timer = setTimeout(() => {
        console.error("WebSocket connection timed out");
        reject(new Error("WebSocket connection timed out"));
      }, 5000);
    });

    // Wait for the connection to open or for the timeout.
    const connectionResult = await Promise.race([
      connection.getOpened(),
      timeoutPromise,
    ]);
    clearTimeout(timer);

    // Obtain a writer from the writable stream.
    const writer = connectionResult.writable.getWriter();
    return {
      readable: connectionResult.readable,
      writable: new MaybeConsumable.WritableStream<Uint8Array>({
        write: (chunk) => writer.write(chunk),
        close: () => writer.close(),
      }),
      close: () => connection.close(),
      closed: connection.getClosed().then(() => undefined),
    };
  }

  /**
   * Not implemented: Adds a reverse tunnel.
   *
   * @throws Method not implemented.
   */
  async addReverseTunnel(
    handler: AdbIncomingSocketHandler,
    address?: string
  ): Promise<string> {
    throw new Error("Method not implemented.");
  }

  /**
   * Not implemented: Removes a reverse tunnel.
   *
   * @throws Method not implemented.
   */
  removeReverseTunnel(address: string): void {
    throw new Error("Method not implemented.");
  }

  /**
   * Not implemented: Clears all reverse tunnels.
   *
   * @throws Method not implemented.
   */
  clearReverseTunnels(): void {
    throw new Error("Method not implemented.");
  }
}
