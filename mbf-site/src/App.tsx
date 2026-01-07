/// <reference types="w3c-web-usb" />
import { useEffect, useRef, useState } from 'react';
import './css/App.css';
import { DeviceModder } from './DeviceModder';
import { ErrorModal } from './components/Modal';
import { Bounce, ToastContainer } from 'react-toastify';
import 'react-toastify/dist/ReactToastify.css';
import { CornerMenu } from './components/CornerMenu';
import { setCoreModOverrideUrl } from './Agent';
import { Log } from './Logging';
import { useOperationModals } from './components/OperationModals';
import { OpenLogsButton } from './components/OpenLogsButton';
import { usingOculusBrowser } from './platformDetection';
import { bridgeData, checkForBridge } from './AdbServerWebSocketConnector';
import { useBridgeManager } from './hooks/BridgeManager';
import { AllowAuth, AskLaurie, DeviceInUse, NoCompatibleDevices, OculusBrowserMessage, Title, UnsupportedMessage } from './AppMessages';
import { PagePinger } from './PagePinger';
import { useDeviceConnector } from './hooks/DeviceConnector';

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
  const { checkedForBridge, bridgeClient, adbDevices, bridgeError, scanning, DeviceScanner, BridgeManagerContextProvider } = useBridgeManager();
  const { devicePreV51, deviceInUse, authing, chosenDevice, connecting, connectError, connectDevice, disconnectDevice, DeviceConnectorContextProvider } = useDeviceConnector(bridgeClient);
  const [modderError, setModderError] = useState<string | null>(null);

  useEffect(() => {
    // If the user is using a bridge and there is only one device, connect to it automatically.
    if (!connecting && chosenDevice == null && bridgeClient != null && adbDevices.length == 1) {
      connectDevice(adbDevices[0])
    }
  });

  if (chosenDevice !== null) {
    return (
      <DeviceConnectorContextProvider>
        {bridgeClient && <PagePinger url={bridgeData.pingAddress} interval={5000} />}
        <DeviceModder quit={(err) => {
          if (err != null) {
            setModderError(String(err));
          }
          disconnectDevice();
          }
        } />
      </DeviceConnectorContextProvider>
    )
  } else if (authing) {
    return (
      <DeviceConnectorContextProvider>
        <AllowAuth />
      </DeviceConnectorContextProvider>
    )
  } else {
    return (
      <DeviceConnectorContextProvider>
        {bridgeClient && <DeviceScanner />}
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
                  <li key={device.serial}>
                    <button onClick={() => !connecting && connectDevice(device)}>Connect to {device.serial}</button>
                  </li>
                )}
              </ul>
              <span><OpenLogsButton /></span>
            </div>
          </>}

          {!bridgeClient && navigator.usb && <>
            <div className="chooseDeviceContainer">
              <span><OpenLogsButton /></span>
              <button onClick={() => !connecting && connectDevice()}>Connect to Quest</button>
            </div>
          </>}

          {connectError && <ErrorModal isVisible={true}
            title="Failed to connect to device"
            description={connectError}
            onClose={() => disconnectDevice()}>
            <AskLaurie />
          </ErrorModal>}

          {deviceInUse && <ErrorModal isVisible={true}
            onClose={() => disconnectDevice()}
            title="Device in use">
            <DeviceInUse />
          </ErrorModal>}
        </div>
      </DeviceConnectorContextProvider>
    )
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
  const { OperationModalContextProvider, OperationModals } = useOperationModals();
  return <div className='main'>
    <OperationModalContextProvider>
      <AppContents />
      <CornerMenu />
      <OperationModals />
      <ToastContainer
        position="bottom-right"
        theme="dark"
        autoClose={5000}
        transition={Bounce}
        hideProgressBar={true} />
    </OperationModalContextProvider>
  </div>
}

export default App;
