import { AdbServerClient } from '@yume-chan/adb';
import { useState, useEffect } from 'react';
import { checkForBridge, AdbServerWebSocketConnector } from './AdbServerWebSocketConnector';
import { Log } from './Logging';

/**
 * Compares two arrays of device objects for equality.
 * Returns true if both arrays have the same length and all corresponding device objects are equal.
 *
 * @param devices1 - The first array of device objects.
 * @param devices2 - The second array of device objects.
 * @returns True if the arrays are equal, false otherwise.
 */
function areDevicesEqual(devices1: Record<string, any>[], devices2: Record<string, any>[]): boolean {
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
function areObjectsEqual(obj1: Record<string, any>, obj2: Record<string, any>): boolean {
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

/**
 * Props for the BridgeManager component.
 */
export interface BridgeManagerProps {
  /**
   * Fires when the bridge client is updated.
   * @param client - The bridge client, or null if the bridge is not running.
   * @returns
   */
  onBridgeClientUpdated?: (client: AdbServerClient | null) => void,

  /**
   * Fires when the list of devices is updated.
   * @param devices - The list of devices.
   * @returns
   */
  onAdbDevicesUpdated?: (devices: AdbServerClient.Device[]) => void,

  /**
   * Fires when the bridge is checked for.
   * @param checked - Whether the bridge was found or not.
   * @returns
   */
  onCheckedForBridge?: (checked: boolean) => void
}

/**
 * BridgeManager is a React component that manages the connection to an ADB bridge server.
 *
 * - Checks if the ADB bridge is running and establishes a client connection if available.
 * - Periodically polls for connected devices via the bridge and notifies parent components of updates.
 * - Handles bridge and device connection errors, and notifies parent components when the bridge or device list changes.
 *
 * Props:
 * - onBridgeClientUpdated: Callback fired when the bridge client is updated or becomes unavailable.
 * - onAdbDevicesUpdated: Callback fired when the list of connected devices changes.
 * - onCheckedForBridge: Callback fired when the bridge check completes.
 *
 * This component does not render any UI.
 */
export function BridgeManager({ onBridgeClientUpdated: onBridgeClientUpdate, onAdbDevicesUpdated, onCheckedForBridge }: BridgeManagerProps) {
  const [checkedForBridge, setCheckedForBridge] = useState(false);
  const [bridgeClient, setBridgeClient] = useState<AdbServerClient | null>(null);
  const [adbDevices, setAdbDevices] = useState<AdbServerClient.Device[]>([]);

  // Check if the bridge is running
  useEffect(() => {
    if (!checkedForBridge) {
      checkForBridge().then(haveBridge => {
        if (haveBridge) {
          const client = new AdbServerClient(new AdbServerWebSocketConnector());
          setBridgeClient(client);
          onBridgeClientUpdate?.(client);
        }

        setCheckedForBridge(true);
        onCheckedForBridge?.(true);
      });
    }
  });

  // Update the available devices on an interval
  useEffect(() => {
    if (bridgeClient) {
      const deviceUpdate = async () => {
        try {
          const client = new AdbServerClient(new AdbServerWebSocketConnector());
          const devices = (await client.getDevices()).filter(device => device.authenticating === false);

          if (!areDevicesEqual(devices, adbDevices)) {
            setAdbDevices(devices);
            onAdbDevicesUpdated?.(devices);
          }
        } catch (err) {
          setBridgeClient(null);
          setAdbDevices([]);
          setCheckedForBridge(false);

          onBridgeClientUpdate?.(null);
          onAdbDevicesUpdated?.([]);
          onCheckedForBridge?.(false);

          Log.error("Failed to get devices: " + err, err);
        }
      };
      const timer = setInterval(deviceUpdate, 1000);
      deviceUpdate();

      return () => clearInterval(timer);
    }
  });

  return <></>;
}
