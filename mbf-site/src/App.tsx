import { useEffect, useRef, useState } from 'react';

import './css/App.css';
import { AdbDaemonWebUsbConnection, AdbDaemonWebUsbDeviceManager } from '@yume-chan/adb-daemon-webusb';
import { AdbDaemonTransport, Adb, AdbServerClient, AdbServerTransport, } from '@yume-chan/adb';

import AdbWebCredentialStore from "@yume-chan/adb-credential-web";
import { DeviceModder } from './DeviceModder';
import { ErrorModal } from './components/Modal';
import { Bounce, ToastContainer } from 'react-toastify';
import 'react-toastify/dist/ReactToastify.css';
import { CornerMenu } from './components/CornerMenu';
import { setCoreModOverrideUrl } from './Agent';
import { Log } from './Logging';
import { OperationModals } from './components/OperationModals';
import { OpenLogsButton } from './components/OpenLogsButton';
import { isViewingOnIos, isViewingOnMobile, isViewingOnWindows, usingOculusBrowser } from './platformDetection';
import { SourceUrl } from '.';
import { AdbServerWebSocketConnector, checkForBridge } from './AdbServerWebSocketConnector';

type NoDeviceCause = "NoDeviceSelected" | "DeviceInUse";

const MIN_SUPPORTED_ANDROID_VERSION: number = 11;

async function connectAdbDevice(client: AdbServerClient, device: AdbServerClient.Device): Promise<Adb> {
  const transport = await client.createTransport(device);
  return new Adb(transport);
}

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
  } catch(err) {
    if(String(err).includes("Unable to claim interface")) {
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

// Attempts to invoke mbf-adb-killer to disconnect the ADB server, avoiding the developer working on MBF having to manually do this.
async function tryDisconnectAdb() {
  try {
    await fetch("http://localhost:25898");
  } catch {
    Log.warn("ADB killer is not running. ADB will have to be killed manually");
  }
}

export async function getAndroidVersion(device: Adb) {
  const result = await device.subprocess.spawnAndWait("getprop ro.build.version.release");
  return Number(result.stdout.trim());
}

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

function ChooseDevice() {
  const [authing, setAuthing] = useState(false);
  const [chosenDevice, setChosenDevice] = useState(null as Adb | null);
  const [connectError, setConnectError] = useState(null as string | null);
  const [devicePreV51, setdevicePreV51] = useState(false);
  const [deviceInUse, setDeviceInUse] = useState(false);
  const [checkedForBridge, setCheckedForBridge] = useState(false);
  const [bridgeClient, setBridgeClient] = useState<AdbServerClient | null>(null);
  const [adbDevices, setAdbDevices] = useState<AdbServerClient.Device[]>([]);

  // Check if the bridge is running
  useEffect(() => {
    if (!checkedForBridge) {
      checkForBridge().then(haveBridge => {
        if (haveBridge) {
          setBridgeClient(new AdbServerClient(new AdbServerWebSocketConnector()));
        }

        setCheckedForBridge(true);
      });
    }
  });

  // Update the available devices on an interval
  useEffect(() => {
    if (bridgeClient && !chosenDevice) {
      const deviceUpdate = async () => {
        try {
          const client = new AdbServerClient(new AdbServerWebSocketConnector())
          const devices = (await client.getDevices()).filter(device => device.authenticating === false);

          if (!areDevicesEqual(devices, adbDevices)) {
            setAdbDevices(devices);
          }
        } catch (err) {
          setBridgeClient(null);
          setAdbDevices([]);
          setCheckedForBridge(false);
          Log.error("Failed to get devices: " + err);
          console.error("Failed to get devices: ", err);
        }
      }
      const timer = setInterval(deviceUpdate, 1000);
      deviceUpdate();

      return () => clearInterval(timer);
    }
  })

  async function connectDevice(device: Adb) {
    const androidVersion = await getAndroidVersion(device);
    Log.debug("Device android version: " + androidVersion);
    setdevicePreV51(androidVersion < MIN_SUPPORTED_ANDROID_VERSION);
    setAuthing(false);
    setChosenDevice(device);

    await device.transport.disconnected;
    setChosenDevice(null);
  }

  if(chosenDevice !== null) {
    Log.debug("Device model: " + chosenDevice.banner.model);
    if(chosenDevice.banner.model === "Quest") { // "Quest" not "Quest 2/3"
      return <div className='container mainContainer'>
        <h1>Quest 1 Not Supported</h1>
        <p>ModsBeforeFriday has detected that you're using a Quest 1, which is not supported by MBF. (and never will be)</p>
        <p>This is because Quest 1 uses different builds of the Beat Saber game and so mods are stuck forever on version 1.28.0 of the game.</p>
        <p>Follow <a href="https://bsmg.wiki/quest/modding-quest1.html">this link</a> for instructions on how to set up mods on Quest 1.</p>
      </div>
    } else if(devicePreV51 && chosenDevice.banner.model?.includes("Quest")) {
      return <div className="container mainContainer">
        <h1>Pre-v51 OS Detected</h1>
        <p>ModsBeforeFriday has detected that you have an outdated version of the Quest operating system installed which is no longer supported by mods.</p>
        <p>Please ensure your operating system is up to date and then refresh the page.</p>
      </div>
    } else  {
      return <>
        <DeviceModder device={chosenDevice} usingBridge={bridgeClient != null} quit={(err) => {
          if(err != null) {
            setConnectError(String(err));
          }
          chosenDevice.close().catch(err => Log.warn("Failed to close device " + err));
          setChosenDevice(null);
        }} />
      </>
    }
  } else if(authing) {
    return <div className='container mainContainer fadeIn'>
      <h2>Allow connection in headset</h2>
      <p>Put on your headset and click <b>"Always allow from this computer"</b></p>
      <p>(You should only have to do this once.)</p>
      <h4>Prompt doesn't show up?</h4>
      <ol>
        <li>Refresh the page.</li>
        <li>Put your headset <b>on your head</b>.</li>
        <li>Attempt to connect to your quest again.</li>
      </ol>
      <p>(Sometimes the quest only shows the prompt if the headset is on your head.)</p>
      <p>If these steps do not work, <b>reboot your quest and try once more.</b></p>
    </div>
  } else  {
    return <>
        <div className="container mainContainer">
          <Title />
          <p>To get started, plug your Quest in with a USB-C cable and click the button below.</p>

          <NoCompatibleDevices />

          {(() => {
            console.log(bridgeClient, adbDevices);
            if(bridgeClient && adbDevices.length > 0) {
              return <div className="connectedDevicesContainer">
                <h2>Connected devices</h2>
                <ul>
                  {adbDevices.map(device => {
                    return <li key={device.serial}>
                      <button onClick={async () => {
                        try {
                          const adbDevice = await connectAdbDevice(bridgeClient, device);
                          await connectDevice(adbDevice)
                        } catch(error) {
                          Log.error("Failed to connect: " + error);
                          setConnectError(String(error));
                          setChosenDevice(null);
                        }
                      }}>Connect to {device.serial}</button>
                    </li>
                  })}
                </ul>
              </div>
            }

            return <div className="chooseDeviceContainer">
              <span><OpenLogsButton /></span>
              <button onClick={async () => {
                  let device: Adb | null;

                  try {
                    const result = await connect(() => setAuthing(true));
                    if(result === "NoDeviceSelected") {
                      device = null;
                    } else if(result === "DeviceInUse") {
                      setDeviceInUse(true);
                      return;
                    } else  {
                      device = result;

                      setChosenDevice(device);
                    }

                  } catch(error) {
                    Log.error("Failed to connect: " + error);
                    setConnectError(String(error));
                    setChosenDevice(null);
                    return;
                  }
                }}>Connect to Quest</button>
            </div>
          })()}

          <ErrorModal isVisible={connectError != null}
            title="Failed to connect to device"
            description={connectError}
            onClose={() => setConnectError(null)}/>

          <ErrorModal isVisible={deviceInUse}
            onClose={() => setDeviceInUse(false)}
            title="Device in use">
              <DeviceInUse />
          </ErrorModal>
        </div>
      </>
  }
}

function DeviceInUse() {
 return <>
  <p>Some other app is trying to access your Quest, e.g. SideQuest.</p>
  {isViewingOnWindows() ?
    <>
      <p>To fix this, close SideQuest if you have it open, press <span className="codeBox">Win + R</span> and type the following text, and finally press enter.</p>
      <span className="codeBox">taskkill /IM adb.exe /F</span>
      <p>Alternatively, restart your computer.</p>
    </>
    : <p>To fix this, restart your {isViewingOnMobile() ? "phone" : "computer"}.</p>}
 </>
}

function Title() {
  return <>
    <h1>
      <span className="initial">M</span>
      <span className="title">ods</span>
      <span className="initial">B</span>
      <span className="title">efore</span>
      <span className="initial">F</span>
      <span className="title">riday</span>
      <span className="initial">!</span>
      <p className="williamGay">william gay</p>
    </h1>
    <a href={SourceUrl} target="_blank" rel="noopener noreferrer" className="mobileOnly">Source Code</a>
    <p>The easiest way to install custom songs for Beat Saber on Quest!</p>
  </>
}

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

  if (usingOculusBrowser()) {
    return <OculusBrowserMessage />
  } else  if (navigator.usb === undefined && !hasBridge) {
    return <UnsupportedMessage />
  } else if (hasSetCoreUrl || !mustEnterUrl) {
    return <ChooseDevice />
  } else  {
    return <ChooseCoreModUrl setSpecifiedCoreMods={() => setSetCoreUrl(true)}/>
  }
}

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



function OculusBrowserMessage() {
  return <div className="container mainContainer">
    <h1>Quest Browser Detected</h1>
    <p>MBF has detected that you're trying to use the built-in Quest browser.</p>
    <p>Unfortunately, <b>you cannot use MBF on the device you are attempting to mod.</b></p>
    <DevicesSupportingModding />

    <p>(MBF can be used on a Quest if you install a chromium browser, however this can only be used to mod <b>another Quest headset</b>, connected via USB.)</p>
  </div>
}

function UnsupportedMessage() {
  return <div className='container mainContainer'>
    {isViewingOnIos() ? <>
      <h1>iOS is not supported</h1>
      <p>MBF has detected that you're trying to use it from an iOS device. Unfortunately, Apple does not allow WebUSB, which MBF needs to be able to interact with the Quest.</p>
      <DevicesSupportingModding />

      <p>.... and one of the following supported browsers:</p>
    </> : <>
      <h1>Browser Unsupported</h1>
      <p>It looks like your browser doesn't support WebUSB, which this app needs to be able to access your Quest's files.</p>
    </>}

    <h2>Supported Browsers</h2>
    <SupportedBrowsers />
  </div>
}

function DevicesSupportingModding() {
  return <>
    <p>To mod your game, you will need one of: </p>
    <ul>
      <li>A PC or Mac (preferred)</li>
      <li>An Android phone (still totally works)</li>
    </ul>
  </>
}

function SupportedBrowsers() {
  if(isViewingOnMobile()) {
    return <>
      <ul>
        <li>Google Chrome for Android 122 or newer</li>
        <li>Edge for Android 123 or newer</li>
      </ul>
      <h3 className='fireFox'>Firefox for Android is NOT supported</h3>
    </>
  } else  {
    return <>
      <ul>
        <li>Google Chrome 61 or newer</li>
        <li>Opera 48 or newer</li>
        <li>Microsoft Edge 79 or newer</li>
      </ul>
      <h3 className='fireFox'>Firefox and Safari are NOT supported.</h3>
      <p>(There is no feasible way to add support for Firefox as Mozilla have chosen not to support WebUSB for security reasons.)</p>
    </>
  }
}

function NoCompatibleDevices() {
  return <>
    <h3>No compatible devices?</h3>

    <p>
      To use MBF, you must enable developer mode so that your Quest is accessible via USB.
      <br />Follow the <a href="https://developer.oculus.com/documentation/native/android/mobile-device-setup/?locale=en_GB" target="_blank" rel="noopener noreferrer">official guide</a> -
      you'll need to create a new organisation and enable USB debugging.
    </p>

    {isViewingOnMobile() && <>
      <h4>Using Android?</h4>
      <p>It's possible that the connection between your device and the Quest has been set up the wrong way around. To fix this:</p>
      <ul>
        <li>Swipe down from the top of the screen.</li>
        <li>Click the dialog relating to the USB connection. This might be called "charging via USB".</li>
        <li>Change "USB controlled by" to "Connected device". If "Connected device" is already selected, change it to "This device" and change it back.</li>
      </ul>
      <h4>Still not working?</h4>
      <p>Try unplugging your cable and plugging the end that's currently in your phone into your Quest.</p>
    </>}
  </>
}

export default App;
