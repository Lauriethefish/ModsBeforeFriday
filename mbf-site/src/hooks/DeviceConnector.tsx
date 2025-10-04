import { Adb, AdbDaemonTransport, AdbServerClient } from "@yume-chan/adb";
import {
  createContext,
  PropsWithChildren,
  useCallback,
  useContext,
  useMemo,
  useState,
} from "react";
import { Log } from "../Logging";
import { waitForDisconnect } from "../waitForDisconnect";
import AdbWebCredentialStore from "@yume-chan/adb-credential-web";
import {
  AdbDaemonWebUsbDeviceManager,
  AdbDaemonWebUsbConnection,
} from "@yume-chan/adb-daemon-webusb";
import { installLoggers } from "../Agent";

const NON_LEGACY_ANDROID_VERSION: number = 11;

/**
 * Retrieves the Android version of the connected device.
 *
 * @param device - The ADB device instance to query for the Android version.
 * @returns A promise that resolves to the Android version as a number.
 *          The version is extracted from the device's system property `ro.build.version.release`.
 */
async function getAndroidVersion(device: Adb) {
  return Number(
    await device.subprocess.noneProtocol.spawnWaitText(
      "getprop ro.build.version.release"
    )
  );
}

/**
 * Connects to the ADB server using the given client and device.
 * @param client The ADB server client to use for the connection.
 * @param device The device to connect to.
 * @returns
 */
async function connectAdbDevice(
  client: AdbServerClient,
  device: AdbServerClient.Device
): Promise<Adb> {
  const transport = await client.createTransport(device);
  return new Adb(transport);
}

/**
 * Attempts to stop the ADB server by sending a request to the ADB killer.
 * @returns A promise that resolves when the ADB server is disconnected.
 */
async function tryDisconnectAdb() {
  try {
    await fetch("http://localhost:25898");
  } catch {
    Log.warn("ADB killer is not running. ADB will have to be killed manually");
  }
}

interface DeviceConnectorData {
  /** Indicates if the connected device is running a pre-v51 (unsupported) OS version. */
  devicePreV51: boolean;

  /** Indicates if the device is currently in use by another process. */
  deviceInUse: boolean;

  /** Indicates if the device is currently authenticating. */
  authing: boolean;

  /** The currently selected ADB device, or null if none is selected. */
  chosenDevice: Adb | null;

  /** Indicates if a connection attempt is in progress. */
  connecting: boolean;

  /** Error message if the last connection attempt failed, or null if there was no error. */
  connectError: string | null;

  /** Indicates if the device is using a bridge connection. */
  usingBridge: boolean;
}

interface DeviceConnectorCallbacks {
  connectDevice: (device?: AdbServerClient.Device) => any;
  disconnectDevice: () => void;
  DeviceConnectorContextProvider: React.FC<PropsWithChildren>;
}

const DeviceConnectorContext =
  createContext<Readonly<DeviceConnectorData> | null>(null);

type NoDeviceCause = "NoDeviceSelected" | "DeviceInUse";

export function useDeviceConnector(
  serverClient: AdbServerClient | null
): Readonly<DeviceConnectorData & DeviceConnectorCallbacks> {
  const [devicePreV51, setDevicePreV51] = useState<DeviceConnectorData["devicePreV51"]>(false);
  const [deviceInUse, setDeviceInUse] = useState<DeviceConnectorData["deviceInUse"]>(false);
  const [authing, setAuthing] = useState<DeviceConnectorData["authing"]>(false);
  const [chosenDevice, setChosenDevice] = useState<DeviceConnectorData["chosenDevice"]>(null);
  const [connecting, setConnecting] = useState<DeviceConnectorData["connecting"]>(false);
  const [connectError, setConnectError] = useState<DeviceConnectorData["connectError"]>(null);
  const [usingBridge, setUsingBridge] = useState<DeviceConnectorData["usingBridge"]>(false);

  const _DeviceConnectorContextProvider = useCallback<React.FC<PropsWithChildren>>(
    function DeviceConnectorContextProvider({ children }) {
      return (
        <DeviceConnectorContext.Provider
          value={{
            devicePreV51,
            deviceInUse,
            authing,
            chosenDevice,
            connecting,
            connectError,
            usingBridge,
          }}
        >
          {children}
        </DeviceConnectorContext.Provider>
      );
    },
    [devicePreV51, deviceInUse, authing, chosenDevice, connecting, connectError]
  );

  const clearDevice = useCallback(() => {
    setChosenDevice(null);
    setConnecting(false);
    setAuthing(false);
    setDevicePreV51(false);
    setDeviceInUse(false);
    setUsingBridge(false);
  }, [setDevicePreV51, setAuthing, setChosenDevice, setConnecting]);

  /**
   * Connects to the ADB server using WebUSB.
   * @param setAuthing A function to call when the connection is being authenticated.
   * @returns The connected ADB device or an error message.
   */
  const connect = useCallback(async function connect(): Promise<Adb | NoDeviceCause> {
      const device_manager = new AdbDaemonWebUsbDeviceManager(navigator.usb);
      const quest = await device_manager.requestDevice();
      if (quest === undefined) {
        return "NoDeviceSelected";
      }

      let connection: AdbDaemonWebUsbConnection;
      try {
        if (import.meta.env.DEV) {
          Log.debug(
            "Developer build detected, attempting to disconnect ADB server before connecting to quest"
          );
          await tryDisconnectAdb();
        }

        connection = await quest.connect();
        installLoggers();
      } catch (err) {
        if (String(err).includes("The device is already in used")) {
          Log.warn("Full interface error: " + err);
          setConnectError(String(err));
          // Some other ADB daemon is hogging the connection, so we can't get to the Quest.
          return "DeviceInUse";
        } else {
          throw err;
        }
      }
      const keyStore: AdbWebCredentialStore = new AdbWebCredentialStore(
        "ModsBeforeFriday"
      );

      setAuthing(true);
      const transport: AdbDaemonTransport =
        await AdbDaemonTransport.authenticate({
          serial: quest.serial,
          connection,
          credentialStore: keyStore,
        });
      setAuthing(false);

      return new Adb(transport);
    }, [setAuthing]);

  /**
   * Connects to a Quest device using WebUSB and manages connection state.
   *
   * 1. Attempts to connect to a Quest device via WebUSB.
   * 2. Handles device selection and device-in-use errors.
   * 3. Updates authentication, device selection, and error state as appropriate.
   *
   * @param stateSetters - An object containing state setters.
   */
  const connectWebUsb = useCallback(async function connectWebUsb(): Promise<Adb | null> {
    try {
      clearDevice();
      setConnecting(true);

      const result = await connect();

      switch (result) {
        case "NoDeviceSelected": {
          break;
        }

        case "DeviceInUse": {
          clearDevice();
          setDeviceInUse(true);
          break;
        }

        default: {
          clearDevice();
          setChosenDevice(result);

          return result;
        }
      }
    } catch (error) {
      Log.error("Failed to connect: " + error);
      clearDevice();
      setConnectError(String(error));
    }

    return null;
  }, [
    connect,
    setConnecting,
    setAuthing,
    setDeviceInUse,
    setChosenDevice,
    setConnectError,
  ]);

  /**
   * Handles the process of connecting to an ADB device and managing its connection state.
   *
   * 1. Retrieves the Android version of the device.
   * 2. Sets a flag if the device is running an unsupported (pre-v51) OS version.
   * 3. Updates authentication and device selection state.
   * 4. Waits for the device to disconnect.
   * 5. Resets the selected device state after disconnection.
   *
   * @param device - The connected ADB device instance.
   * @param stateSetters - An object containing state setters.
   */
  const initializeDevice = useCallback(
    async function initializeDevice(device: Adb) {
      const androidVersion = await getAndroidVersion(device);
      Log.debug("Device android version: " + androidVersion);
      setDevicePreV51(androidVersion < NON_LEGACY_ANDROID_VERSION);
      setAuthing(false);
      setChosenDevice(device);

      await waitForDisconnect(device);
    },
    [setDevicePreV51, setAuthing, setChosenDevice, setConnecting, clearDevice]
  );

  /**
   * Connects to a device using the ADB bridge client and manages connection state.
   *
   * 1. Attempts to create an ADB connection to the specified device using the provided bridge client.
   * 2. If successful, calls `connectDevice` to handle version checks and state updates.
   * 3. Handles errors by logging, setting the connection error state, and resetting the selected device.
   *
   * @param serverClient - The ADB server client used for the bridge connection.
   * @param device - The target device to connect to.
   * @param stateSetters - An object containing state setters.
   */
  const connectBridgeDevice = useCallback(
    async function connectBridgeDevice(device: AdbServerClient.Device) {
      try {
        if (serverClient === null) {
          Log.error("Bridge client is null, cannot connect to device");
          return;
        }

        setConnecting(true);

        const adbDevice = await connectAdbDevice(serverClient, device);
        await initializeDevice(adbDevice);
      } catch (error) {
        Log.error("Failed to connect: " + error);
        setConnectError(String(error));
      } finally {
        clearDevice();
      }
    },
    [
      serverClient,
      connectAdbDevice,
      setConnectError,
      setChosenDevice,
      setConnecting,
    ]
  );

  const connectDevice = useCallback(
    async function connectDevice(device?: AdbServerClient.Device) {
      if (chosenDevice) {
        throw new Error("Device is already connected");
      }

      try {
        if (device) {
          connectBridgeDevice(device);
        } else {
          const device = await connectWebUsb();

          if (device) {
            await initializeDevice(device);
          }
        }
      } catch (err) {
        Log.error(String(err));
      } finally {
        clearDevice();
      }
    },
    [initializeDevice, connectBridgeDevice]
  );

  const disconnectDevice = useCallback(() => {
    try {
      chosenDevice?.close();
    } catch (err) {
      Log.error("Failed to disconnect device: " + err);
    } finally {
      clearDevice();
      setConnectError(null);
    }
  }, []);

  return {
    devicePreV51,
    deviceInUse,
    authing,
    chosenDevice,
    connecting,
    connectError,
    usingBridge,
    connectDevice,
    disconnectDevice,
    DeviceConnectorContextProvider: _DeviceConnectorContextProvider,
  };
}

export function useDeviceConnectorContext() {
  const context = useContext(DeviceConnectorContext);

  if (!context) {
    throw new Error(
      "useDeviceConnectorContext must be used within a DeviceConnectorContextProvider"
    );
  }

  return context;
}