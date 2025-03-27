import type { Adb } from "@yume-chan/adb";
import { PromiseResolver } from "@yume-chan/async";
import { Log } from "./Logging";

const disconnectPromises = new Map<string, Promise<void>>();
export async function waitForDisconnect(device: Adb) {
    if (disconnectPromises.has(device.serial)) {
        Log.debug(`Already waiting for ${device.serial} to disconnect`, device);
        return await disconnectPromises.get(device.serial);
    }

    Log.debug(`Waiting for ${device.serial} to disconnect`, device);
    var resolver = new PromiseResolver<void>();

    disconnectPromises.set(device.serial, resolver.promise);

    // Track if the transport disconnects early
    let disconnectedEarly = true;
    setTimeout(() => disconnectedEarly = false, 1000);

    // Wait for the transport to determine disconnect
    await device.transport.disconnected;

    // Old adb server versions don't support the wait-for-any-disconnect feautre
    // so if the transport disconnects within 1 second, we spawn a process that
    // never exits and await it instead.
    if (disconnectedEarly) {
        try {
            Log.debug(`Waiting for ${device.serial} to disconnect using subprocess`, device);
            await device.subprocess.spawnAndWait("read");
        } catch (error) {
            console.error("ADB server process exited: " + error, error);
        }
    }

    Log.debug(`Devoce ${device.serial} disconnected`, device);
    resolver.resolve();
    disconnectPromises.delete(device.serial);
}
