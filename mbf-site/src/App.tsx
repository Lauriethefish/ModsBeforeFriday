/// <reference types="w3c-web-usb" />
import { useRef, useState } from 'react';

import './css/App.css';
import { AdbDaemonWebUsbConnection, AdbDaemonWebUsbDeviceManager } from '@yume-chan/adb-daemon-webusb';
import { AdbDaemonTransport, Adb } from '@yume-chan/adb';

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
import { isViewingOnIos, isViewingOnMobile, isViewingOnWindows, usingOculusBrowser } from './platformDetection';
import { SourceUrl } from '.';
import { useDeviceStore } from './DeviceStore';
import { getLang, initLanguage } from './localization/shared';

type NoDeviceCause = "NoDeviceSelected" | "DeviceInUse";

const NON_LEGACY_ANDROID_VERSION: number = 11;

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

// Attempts to invoke mbf-adb-killer to disconnect the ADB server, avoiding the developer working on MBF having to manually do this.
async function tryDisconnectAdb() {
  try {
    await fetch("http://localhost:25898");
  } catch {
    Log.warn("ADB killer is not running. ADB will have to be killed manually");
  }
}

export async function getAndroidVersion(device: Adb) {
  return Number((await device.subprocess.noneProtocol.spawnWaitText("getprop ro.build.version.release")));
}

function ChooseDevice() {
  const [authing, setAuthing] = useState(false);
  const [connectError, setConnectError] = useState(null as string | null);
  const [deviceInUse, setDeviceInUse] = useState(false);
  const {
    devicePreV51, setDevicePreV51,
    device: chosenDevice, setDevice: setChosenDevice,
    androidVersion, setAndroidVersion
  } = useDeviceStore();

  if(chosenDevice !== null) {
    return <>
      <DeviceModder device={chosenDevice} devicePreV51={devicePreV51} quit={(err) => {
        if(err != null) {
          setConnectError(String(err));
        }
        chosenDevice.close().catch(err => Log.warn("Failed to close device " + err));
        setChosenDevice(null);
      }} />
    </>
  } else if(authing) {
    return <div className='container mainContainer fadeIn'>
      {getLang().allowConnectionInHeadSet}
    </div>
  } else  {
    return <>
        <div className="container mainContainer">
          <Title />
          {getLang().toGetStart}
          <NoCompatibleDevices />

          <div className="chooseDeviceContainer">
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

                  const androidVersion = await getAndroidVersion(device);
                  setAndroidVersion(androidVersion);

                  Log.debug("Device android version: " + androidVersion);

                  const deviceName = device.banner.model;
                  if (deviceName === "Quest") {
                    Log.debug("Device is a Quest 1, switching to pre-v51 mode");
                    setDevicePreV51(androidVersion < NON_LEGACY_ANDROID_VERSION);                  
                  }

                  setAuthing(false);
                  setChosenDevice(device);

                  await device.transport.disconnected;
                  setChosenDevice(null);
                }

              } catch(error) {
                Log.error("Failed to connect: " + error);
                setConnectError(String(error));
                setChosenDevice(null);
                return;
              }
            }}>{getLang().connectToQuest}</button>
          </div>

          <ErrorModal isVisible={connectError != null}
            title={getLang().failedToConnectDevice}
            description={connectError}
            onClose={() => setConnectError(null)}>
              {getLang().askLaurie}
          </ErrorModal>

          <ErrorModal isVisible={deviceInUse}
            onClose={() => setDeviceInUse(false)}
            title={getLang().deviceInUse}>
              <DeviceInUse />
          </ErrorModal>
        </div>
      </>
  }
}



function DeviceInUse() {
 return <>
  <p>{getLang().otherAppIsAccessQuest}</p>
  {isViewingOnWindows() ? 
    <>
      {getLang().killAdb}

      {getLang().askLaurie}
    </>
    : <p>{getLang().fixWithRestartDevice(isViewingOnMobile())}</p>}
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
    <a href={SourceUrl} target="_blank" rel="noopener noreferrer" className="mobileOnly">{getLang().sourceCode}</a>
    <p>{getLang().titleText}</p>
  </>
}

function ChooseCoreModUrl({ setSpecifiedCoreMods } : { setSpecifiedCoreMods: () => void}) {
  const inputFieldRef = useRef<HTMLInputElement | null>(null);

  return <div className='container mainContainer'>
    {getLang().chooseCoreModUrl}
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
      {getLang().confirmUrl}
    </button>
  </div>
}

function AppContents() {
  const [ hasSetCoreUrl, setSetCoreUrl ] = useState(false);

  const overrideQueryParam: string | null = new URLSearchParams(window.location.search).get("setcores");
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
  } else  if (navigator.usb === undefined) {
    return <UnsupportedMessage />
  } else if (hasSetCoreUrl || !mustEnterUrl) {
    return <ChooseDevice />
  } else  {
    return <ChooseCoreModUrl setSpecifiedCoreMods={() => setSetCoreUrl(true)}/>
  }
}

function App() {
  initLanguage()
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
    {getLang().questBrowserMessage}
    <DevicesSupportingModding />

    <p>{getLang().onlyWorkWithAnotherQuest}</p>
  </div>
}

function UnsupportedMessage() {
  return <div className='container mainContainer'>
    {isViewingOnIos() ? <>
      {getLang().iosNotSupported}
      <DevicesSupportingModding />

      <p>{getLang().supportedBrowserHintInIOS}</p>
    </> : <>
    {getLang().browserNotSupported}
    </>}

    <h2>{getLang().supportedBrowserTitle}</h2>
    <SupportedBrowsers />
  </div>
}

function DevicesSupportingModding() {
  return <>
    {getLang().deviceSupportingModding}
  </>
}

function SupportedBrowsers() {
  if(isViewingOnMobile()) {
    return <>
      {getLang().supportedBrowserMobile}
    </>
  } else  {
    return <>
      {getLang().supportedBrowserNotMobile}
    </>
  }
}

function NoCompatibleDevices() {
  return <>
    {getLang().noCompatableDevice}

    {isViewingOnMobile() && <>
      {getLang().noCompatableDeviceMobile}
    </>}
  </>
}

export default App;
