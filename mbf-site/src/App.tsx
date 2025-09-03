/// <reference types="w3c-web-usb" />
import { useEffect, useRef, useState } from 'react';

import './css/App.css';
import { AdbDaemonWebUsbConnection, AdbDaemonWebUsbDeviceManager } from '@yume-chan/adb-daemon-webusb';
import { AdbDaemonTransport, Adb, AdbServerClient } from '@yume-chan/adb';

import AdbWebCredentialStore from "@yume-chan/adb-credential-web";
import { DeviceModder } from './DeviceModder';
import { ErrorModal } from './components/Modal';
import { Bounce, ToastContainer } from 'react-toastify';
import 'react-toastify/dist/ReactToastify.css';
import { CornerMenu } from './components/CornerMenu';
import { installLoggers, setCoreModOverrideUrl } from './Agent';
import { Log } from './Logging';
import { OperationModals } from './components/OperationModals';
import { OpenLogsButton } from './components/OpenLogsButton';
import { isViewingOnMobile, isViewingOnWindows, usingOculusBrowser } from './platformDetection';
import { bridgeData, checkForBridge } from './AdbServerWebSocketConnector';
import { waitForDisconnect } from "./waitForDisconnect";
import { BridgeManager } from './BridgeManager';
import { AllowAuth, AskLaurie, DeviceInUse, NoCompatibleDevices, OculusBrowserMessage, OldOsVersion, QuestOneNotSupported, Title, UnsupportedMessage } from './AppMessages';
import { PagePinger } from './PagePinger';
import { useDeviceStore } from './DeviceStore';
import { SourceUrl } from '.';

type NoDeviceCause = "NoDeviceSelected" | "DeviceInUse";

const NON_LEGACY_ANDROID_VERSION: number = 11;

/**
 * Connects to the ADB server using the given client and device.
 * @param client The ADB server client to use for the connection.
 * @param device The device to connect to.
 * @returns
 */
async function connectAdbDevice(client: AdbServerClient, device: AdbServerClient.Device): Promise<Adb> {
  const transport = await client.createTransport(device);
  return new Adb(transport);
}

/**
 * Connects to the ADB server using WebUSB.
 * @param setAuthing A function to call when the connection is being authenticated.
 * @returns The connected ADB device or an error message.
 */
async function connect(
  setAuthing: () => void): Promise<Adb | NoDeviceCause> {
  const device_manager = new AdbDaemonWebUsbDeviceManager(navigator.usb);
  const quest = await device_manager.requestDevice();
  if(quest === undefined) {
    return "NoDeviceSelected";
  }

  let connection: AdbDaemonWebUsbConnection;
  try {
    if(import.meta.env.DEV) {
      Log.debug("Developer build detected, attempting to disconnect ADB server before connecting to quest");
      await tryDisconnectAdb();
    }

    connection = await quest.connect();
    installLoggers();
  } catch(err) {
    if(String(err).includes("The device is already in used")) {
      Log.warn("Full interface error: " + err);
      // Some other ADB daemon is hogging the connection, so we can't get to the Quest.
      return "DeviceInUse";
    } else  {
      throw err;
    }
  }
  const keyStore: AdbWebCredentialStore = new AdbWebCredentialStore("ModsBeforeFriday");

  setAuthing();
  const transport: AdbDaemonTransport = await AdbDaemonTransport.authenticate({
    serial: quest.serial,
    connection,
    credentialStore: keyStore
  });

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

/**
 * Retrieves the Android version of the connected device.
 *
 * @param device - The ADB device instance to query for the Android version.
 * @returns A promise that resolves to the Android version as a number.
 *          The version is extracted from the device's system property `ro.build.version.release`.
 */
export async function getAndroidVersion(device: Adb) {
  return Number((await device.subprocess.noneProtocol.spawnWaitText("getprop ro.build.version.release")));
}

type SetStateAction<T> = React.Dispatch<React.SetStateAction<T>>;

enum ConnectedState {
  NotConnected, WebUsb, Bridge
}

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
async function connectDevice(device: Adb, { setDevicePreV51, setAuthing, setChosenDevice, setConnecting }: {
  /** State setter for indicating if the device is pre-v51 (unsupported). */
  setDevicePreV51: (isPreV51: boolean) => void

  /** State setter for authentication status. */
  setAuthing: SetStateAction<boolean>

  /** State setter for the currently selected device. */
  setChosenDevice: (adb: Adb | null) => void

  /** State setter for connecting status. */
  setConnecting: SetStateAction<boolean>
}) {
  const androidVersion = await getAndroidVersion(device);
  Log.debug("Device android version: " + androidVersion);
  setDevicePreV51(androidVersion < NON_LEGACY_ANDROID_VERSION);
  setAuthing(false);
  setChosenDevice(device);

  await waitForDisconnect(device);

  setChosenDevice(null);
  setConnecting(false);
}

/**
 * Connects to a device using the ADB bridge client and manages connection state.
 *
 * 1. Attempts to create an ADB connection to the specified device using the provided bridge client.
 * 2. If successful, calls `connectDevice` to handle version checks and state updates.
 * 3. Handles errors by logging, setting the connection error state, and resetting the selected device.
 *
 * @param bridgeClient - The ADB server client used for the bridge connection.
 * @param device - The target device to connect to.
 * @param stateSetters - An object containing state setters.
 */
async function connectBridgeDevice(bridgeClient: AdbServerClient, device: AdbServerClient.Device, { setDevicePreV51, setAuthing, setChosenDevice, setConnectError, setConnecting }: {
  /** State setter for indicating if the device is pre-v51 (unsupported). */
  setDevicePreV51: (isPreV51: boolean) => void

  /** State setter for authentication status. */
  setAuthing: SetStateAction<boolean>

  /** State setter for the currently selected device. */
  setChosenDevice: (adb: Adb | null) => void

  /** State setter for connection error messages. */
  setConnectError: SetStateAction<string | null>

  /** State setter for connecting status. */
  setConnecting: SetStateAction<boolean>
}) {
  try {
    if (bridgeClient === null) {
      Log.error("Bridge client is null, cannot connect to device");
      return;
    }
    
    setConnecting(true);

    const adbDevice = await connectAdbDevice(bridgeClient, device);
    await connectDevice(adbDevice, { setDevicePreV51, setAuthing, setChosenDevice, setConnecting });
  } catch(error) {
    Log.error("Failed to connect: " + error, error);
    setConnectError(String(error));
    setChosenDevice(null);
    setConnecting(false);
  }
}

/**
 * Connects to a Quest device using WebUSB and manages connection state.
 *
 * 1. Attempts to connect to a Quest device via WebUSB.
 * 2. Handles device selection and device-in-use errors.
 * 3. Updates authentication, device selection, and error state as appropriate.
 *
 * @param stateSetters - An object containing state setters.
 */
async function connectWebUsb({ setAuthing, setDeviceInUse, setChosenDevice, setConnectError, setConnecting }: {
  /** State setter for authentication status. */
  setAuthing: SetStateAction<boolean>

  /** State setter for indicating if the device is in use by another process. */
  setDeviceInUse: SetStateAction<boolean>

  /** State setter for the currently selected device. */
  setChosenDevice: (adb: Adb | null) => void

  /** State setter for connection error messages. */
  setConnectError: SetStateAction<string | null>

  /** State setter for connecting status. */
  setConnecting: SetStateAction<boolean>
}) {
  let device: Adb | null;

  try {
    setConnecting(true);
    const result = await connect(() => setAuthing(true));
    if(result === "NoDeviceSelected") {
      device = null;
    } else if(result === "DeviceInUse") {
      setDeviceInUse(true);
      setConnecting(false);
      return;
    } else  {
      device = result;
      setChosenDevice(device);
      setConnecting(false);
    }

  } catch(error) {
    Log.error("Failed to connect: " + error);
    setConnectError(String(error));
    setChosenDevice(null);
    setConnecting(false);
    return;
  }
}

/**
 * Main component for device selection and connection flow.
 *
 * Handles all UI and state for connecting to a Quest device via WebUSB or bridge.
 * - Manages authentication, device selection, error, and compatibility states.
 * - If a device is connected, checks for Quest 1 or unsupported OS and shows the appropriate message.
 * - If authenticating, shows instructions for allowing ADB authorization.
 * - Otherwise, shows device selection UI, detected devices, and error modals.
 *
 * State:
 * - authing: Whether authentication is in progress.
 * - chosenDevice: The currently connected ADB device, or null.
 * - connectError: Any connection error message.
 * - devicePreV51: True if the device is running an unsupported (pre-v51) OS.
 * - deviceInUse: True if another app is using the device.
 * - bridgeClient: The current ADB bridge client, if available.
 * - adbDevices: List of detected ADB devices via bridge.
 */
function ChooseDevice() {
  const [authing, setAuthing] = useState(false);
  const [connectError, setConnectError] = useState(null as string | null);
  const [connecting, setConnecting] = useState(false);
  const [deviceInUse, setDeviceInUse] = useState(false);
  const {
    devicePreV51, setDevicePreV51,
    device: chosenDevice, setDevice: setChosenDevice, usingBridge, setUsingBridge,
    androidVersion, setAndroidVersion
  } = useDeviceStore();
  const [bridgeClient, setBridgeClient] = useState<AdbServerClient | null>(null);
  const [adbDevices, setAdbDevices] = useState<AdbServerClient.Device[]>([]);
  const stateSetters = {
    setAuthing,
    setChosenDevice,
    setConnectError,
    setDevicePreV51,
    setDeviceInUse,
    setBridgeClient,
    setAdbDevices,
    setConnecting
  }
  
  useEffect(() => {
    // If the user is using a bridge and there is only one device, connect to it automatically.
    if (!connecting && chosenDevice == null && bridgeClient != null && adbDevices.length == 1) {
      connectBridgeDevice(bridgeClient, adbDevices[0],  stateSetters).catch(err => Log.error("Failed to connect to device: " + err, err));
    }
  });

  if(chosenDevice !== null) {
    return <>
      { bridgeClient && <PagePinger url={bridgeData.pingAddress} interval={5000} /> }
      <DeviceModder quit={(err) => {
        if(err != null) {
          setConnectError(String(err));
        }
        chosenDevice.close().catch(err => Log.error("Failed to close device " + err, err));
        setChosenDevice(null);
        setConnecting(false);
      }} />
    </>
  } else if(authing) {
    return <AllowAuth />
  } else  {
    return <>
      <BridgeManager onBridgeClientUpdated={setBridgeClient} onAdbDevicesUpdated={setAdbDevices} />
      <div className="container mainContainer">
        <Title />
        <p>To get started, plug your Quest in with a USB-C cable and click the button below.</p>
          <p>Want see what mods are available? You can find a full list <a href="https://mods.bsquest.xyz" target="_blank" rel="noopener noreferrer">here!</a></p>
         
        <NoCompatibleDevices />

        {bridgeClient && <>
          <div className="connectedDevicesContainer">
            <h2>Detected devices</h2>
            <ul>
              {adbDevices.map(device =>
                <>
                  <li key={device.serial}>
                    <button onClick={() => !connecting && connectBridgeDevice(bridgeClient, device, stateSetters)}>Connect to {device.serial}</button>
                  </li>
                </>)}
            </ul>
            <span><OpenLogsButton /></span>
          </div>
        </>}
        {!bridgeClient && navigator.usb && <>
          <div className="chooseDeviceContainer">
            <span><OpenLogsButton /></span>
            <button onClick={() => !connecting && connectWebUsb(stateSetters)}>Connect to Quest</button>
          </div>
        </>}

          <ErrorModal isVisible={connectError != null}
            title="Failed to connect to device"
            description={connectError}
            onClose={() => setConnectError(null)}>
              <AskLaurie />
          </ErrorModal>

        <ErrorModal isVisible={deviceInUse}
          onClose={() => setDeviceInUse(false)}
          title="Device in use">
            <DeviceInUse />
        </ErrorModal>
      </div>
    </>
  }
}

/**
 * Renders a UI for manually overriding the core mod JSON URL.
 * Allows the user to input a URL to the raw core mod JSON, updates the app state and URL,
 * and triggers the callback to indicate the core mods have been specified.
 *
 * @param setSpecifiedCoreMods - Callback to update state when the core mods URL is set.
 */
function ChooseCoreModUrl({ setSpecifiedCoreMods } : { setSpecifiedCoreMods: () => void}) {
  const inputFieldRef = useRef<HTMLInputElement | null>(null);

  return <div className='container mainContainer'>
    <h1>Manually override core mod JSON</h1>
    <p>Please specify a complete URL to the raw contents of your core mod JSON</p>
    <input type="text" ref={inputFieldRef}/>
    <br/><br/>
    <button onClick={() => {
      if(inputFieldRef.current !== null) {
        const inputField = inputFieldRef.current;
        Log.warn("Overriding core mods URL to " + inputField.value)
        setCoreModOverrideUrl(inputField.value);
        const searchParams = new URLSearchParams(window.location.search);
        searchParams.set("setcores", inputField.value);
        window.history.replaceState({}, "ModsBeforeThursday", "?" + searchParams.toString());

        setSpecifiedCoreMods();
      }
    }}>
      Confirm URL
    </button>
  </div>
}

/**
 * Main application logic for determining which UI to show based on state and environment.
 *
 * - Checks for a core mod override URL in the query string and updates state accordingly.
 * - Detects if the user is using an unsupported browser or the Oculus browser and shows appropriate messages.
 * - If a core mod URL is set or not required, shows the device selection flow.
 * - Otherwise, prompts the user to enter a core mod URL.
 */
function AppContents() {
  const [ hasSetCoreUrl, setSetCoreUrl ] = useState(false);
  const [ hasBridge, setHasBridge ] = useState(false);
  const overrideQueryParam: string | null = new URLSearchParams(window.location.search).get("setcores");
  useEffect(() => {
    checkForBridge().then((hasBridge) => {
      setHasBridge(hasBridge);
      console.log("Bridge running: " + hasBridge);
    });
  });

  let mustEnterUrl = false;
  if(overrideQueryParam !== "prompt" && overrideQueryParam !== null) {
    if(!hasSetCoreUrl) {
      Log.warn("Setting core mod URL to " + overrideQueryParam);
      setCoreModOverrideUrl(overrideQueryParam);
      setSetCoreUrl(true);
    }
  } else if(overrideQueryParam !== null) {
    Log.debug("Prompting user to specify core mod URL");
    mustEnterUrl = true;
  }

  if (usingOculusBrowser() && !hasBridge) {
    return <OculusBrowserMessage />
  } else  if (navigator.usb === undefined && !hasBridge) {
    return <UnsupportedMessage />
  } else if (hasSetCoreUrl || !mustEnterUrl) {
    return <ChooseDevice />
  } else  {
    return <ChooseCoreModUrl setSpecifiedCoreMods={() => setSetCoreUrl(true)}/>
  }
}

/**
 * Root application component. Renders the main app contents, corner menu, operation modals, and toast notifications.
 */
function App() {
  return <div className='main'>
    <AppContents />
    <CornerMenu />
    <OperationModals />
    <ToastContainer
      position="bottom-right"
      theme="dark"
      autoClose={5000}
      transition={Bounce}
      hideProgressBar={true} />
  </div>
}

export default App;
