import type { Adb } from "@yume-chan/adb";
import { PromiseResolver } from "@yume-chan/async";
import { Log } from "./Logging";

const disconnectPromises = new Map<string, Promise<void>>();
export async function waitForDisconnect(device: Adb) {
    if (disconnectPromises.has(device.serial)) {
        Log.trace(`Already waiting for ${device.serial} to disconnect`, device);
        return await disconnectPromises.get(device.serial);
    }

    Log.debug(`Waiting for ${device.serial} to disconnect`, device);
    var resolver = new PromiseResolver<void>();

    disconnectPromises.set(device.serial, resolver.promise);

    // Wait for the device to disconnect by spawning a subprocess that will block until the connection is lost.
    await device.subprocess.noneProtocol.spawnWait("read").catch(() => {});

    Log.trace(`Device ${device.serial} disconnected`, device);
    resolver.resolve();
    disconnectPromises.delete(device.serial);
}
