import { AdbServerClient } from "@yume-chan/adb";
import {
  useState,
  useEffect,
  useCallback,
  createContext,
  useContext,
} from "react";
import {
  checkForBridge,
  AdbServerWebSocketConnector,
} from "../AdbServerWebSocketConnector";
import { Log } from "../Logging";

/**
 * Compares two arrays of device objects for equality.
 * Returns true if both arrays have the same length and all corresponding device objects are equal.
 *
 * @param devices1 - The first array of device objects.
 * @param devices2 - The second array of device objects.
 * @returns True if the arrays are equal, false otherwise.
 */
function areDevicesEqual(
  devices1: Record<string, any>[],
  devices2: Record<string, any>[]
): boolean {
  if (devices1.length !== devices2.length) {
    return false;
  }

  for (let i = 0; i < devices1.length; i++) {
    const device1 = devices1[i];
    const device2 = devices2[i];

    if (!areObjectsEqual(device1, device2)) {
      return false;
    }
  }

  return true;
}

/**
 * Compares two objects for shallow equality.
 * Returns true if both objects have the same keys and all corresponding values are equal.
 *
 * @param obj1 - The first object.
 * @param obj2 - The second object.
 * @returns True if the objects are equal, false otherwise.
 */
function areObjectsEqual(
  obj1: Record<string, any>,
  obj2: Record<string, any>
): boolean {
  const keys1 = Object.keys(obj1);
  const keys2 = Object.keys(obj2);

  if (keys1.length !== keys2.length) {
    return false;
  }

  for (const key of keys1) {
    if (obj1[key] !== obj2[key]) {
      return false;
    }
  }

  return true;
}

export interface BridgeManagerData {
  checkedForBridge: boolean;
  bridgeClient: AdbServerClient | null;
  adbDevices: AdbServerClient.Device[];
  bridgeError: unknown | null;
  scanning: boolean;
}

export interface BridgeManagerComponents {
  /** A component that scans for devices when mounted */
  DeviceScanner: React.FC;

  /** A context provider to provide the bridge data to the component tree */
  BridgeManagerContextProvider: React.FC<React.PropsWithChildren>;
}

const BridgeManagerContext = createContext<Readonly<BridgeManagerData> | null>(null);

/**
 * Data provided by the BridgeManager context.
 *
 * - checkedForBridge: whether an initial check for a running ADB bridge has completed.
 * - bridgeClient: an AdbServerClient instance when a bridge is available, otherwise null.
 * - adbDevices: list of devices reported by the bridge (filtered to state === "device").
 * - bridgeError: last error encountered while interacting with the bridge, if any.
 * - scanning: whether device polling is currently active.
 */
export function useBridgeManager(): Readonly<
  BridgeManagerData & BridgeManagerComponents
> {
  const [checkedForBridge, setCheckedForBridge] = useState(false);
  const [bridgeClient, setBridgeClient] = useState<AdbServerClient | null>(
    null
  );
  const [adbDevices, setAdbDevices] = useState<AdbServerClient.Device[]>([]);
  const [bridgeError, setBridgeError] = useState<unknown | null>(null);
  const [scanning, setScanning] = useState(true);
  const _DeviceScanner = useCallback<React.FC>(
    function DeviceScanner() {
      useEffect(() => {
        setScanning(true);

        return () => setScanning(false);
      }, []);

      return null;
    },
    [setScanning]
  );
  const _BridgeManagerContextProvider = useCallback<
    React.FC<React.PropsWithChildren>
  >(
    function BridgeManagerContextProvider({ children }) {
      return (
        <BridgeManagerContext.Provider
          value={{
            checkedForBridge,
            bridgeClient,
            adbDevices,
            bridgeError,
            scanning,
          }}
        >
          {children}
        </BridgeManagerContext.Provider>
      );
    },
    [checkedForBridge, bridgeClient, adbDevices, bridgeError, scanning]
  );

  const deviceUpdate = useCallback(async () => {
    try {
      const client = new AdbServerClient(new AdbServerWebSocketConnector());
      const devices = (await client.getDevices()).filter(
        (device) => device.state == "device"
      );

      if (!areDevicesEqual(devices, adbDevices)) {
        setAdbDevices(devices);
      }
      if (bridgeError !== null) {
        setBridgeError(null);
      }
    } catch (err) {
      setBridgeClient(null);
      setAdbDevices([]);
      setCheckedForBridge(false);
      setBridgeError(err);

      Log.error("Failed to get devices: " + err, err);
    }
  }, [
    bridgeError,
    setBridgeError,
    setBridgeClient,
    setAdbDevices,
    setCheckedForBridge,
    setBridgeError,
  ]);

  // Check if the bridge is running
  useEffect(() => {
    if (checkedForBridge) return;

    checkForBridge().then((haveBridge) => {
      if (haveBridge) {
        const client = new AdbServerClient(new AdbServerWebSocketConnector());
        setBridgeClient(client);
      }

      setCheckedForBridge(true);
    });
  }, [checkedForBridge]);

  // Update the available devices on an interval
  useEffect(() => {
    if (!bridgeClient || !scanning) return;

    const timer = setInterval(deviceUpdate, 1000);
    deviceUpdate();

    return () => clearInterval(timer);
  }, [bridgeClient, scanning]);

  return {
    checkedForBridge,
    bridgeClient,
    adbDevices,
    bridgeError,
    scanning,
    DeviceScanner: _DeviceScanner,
    BridgeManagerContextProvider: _BridgeManagerContextProvider,
  };
}

export function useBridgeManagerContext() {
  const context = useContext(BridgeManagerContext);

  if (!context) {
    throw new Error(
      "useBridgeManagerContext must be used within a BridgeManagerContextProvider"
    );
  }

  return context;
}
