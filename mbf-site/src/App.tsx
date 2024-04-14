import { useState } from 'react';

import './css/App.css';
import { AdbDaemonWebUsbConnection, AdbDaemonWebUsbDeviceManager } from '@yume-chan/adb-daemon-webusb';
import { AdbDaemonTransport, Adb } from '@yume-chan/adb';

import AdbWebCredentialStore from "@yume-chan/adb-credential-web";
import { DeviceModder } from './DeviceModder';
import { ErrorModal } from './components/Modal';
import { Bounce, ToastContainer } from 'react-toastify';
import 'react-toastify/dist/ReactToastify.css';



async function connect(
  setAuthing: () => void): Promise<Adb | null> {
 const device_manager = new AdbDaemonWebUsbDeviceManager(navigator.usb);
  const quest = await device_manager.requestDevice();
  if(quest === undefined) {
    return null;
  }

  let connection: AdbDaemonWebUsbConnection;
  try {
    connection = await quest.connect();
  } catch(err) {
    // Some other ADB daemon is hogging the connection, so we can't get to the Quest.
    // On Windows, this can be easily fixed with a Win + R and a command. 
    // On Mac/Linux/Android, using the terminal is harder and so we will just instruct to restart their device.
    const fixInstructions = isViewingOnWindows() ? "To fix this, close SideQuest if you have it open, press Win + R and type the following text, and finally press enter.\ntaskkill /IM adb.exe /F\nAlternatively, restart your computer." 
      : `To fix this, restart your ${isViewingOnMobile() ? "phone" : "computer"}.`

    throw new Error("Some other app is trying to access your quest, e.g. SideQuest.\n" + fixInstructions);
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

function ChooseDevice() {
  const [authing, setAuthing] = useState(false);
  const [chosenDevice, setChosenDevice] = useState(null as Adb | null);
  const [connectError, setConnectError] = useState(null as string | null);

  if(chosenDevice !== null) {
    if(chosenDevice.banner.model === "Quest") { // "Quest" not "Quest 2/3"
      return <div className='container mainContainer'>
        <h1>Quest 1 Not Supported</h1>
        <p>ModsBeforeFriday has detected that you're using a Quest 1, which is no longer supported for modding Beat Saber.</p>
      </div>
    } else {
      return <>
        <DeviceModder device={chosenDevice} quit={(err) => {
          if(err != null) {
            setConnectError(String(err));
          }
          chosenDevice.close().catch(err => console.warn("Failed to close device " + err));
          setChosenDevice(null);
        }} />
      </>
    }
  } else if(authing) {
    return <div className='container mainContainer fadeIn'>
      <h2>Allow connection in headset</h2>
      <p>Put on your headset and click <b>"Always allow from this computer"</b></p>
      <p>(You should only have to do this once.)</p>
      <h4>Already pressed allow?</h4>
      <p>Sometimes the connection fails, despite you allowing access in your Quest. <br/>If this happens, try refreshing the page and re-selecting your device.</p>
    </div>
  } else  {
    return <>
        <div className="container mainContainer">
          <Title />
          <p>To get started, plug your Quest in with a USB-C cable and click the button below.</p>

          <NoCompatibleDevices />

          <button id="chooseDevice" onClick={async () => {
            let device: Adb | null;

            try {
              device = await connect(() => setAuthing(true));
            } catch(e) {
              console.log("Failed to connect: " + e);
              setConnectError(String(e));
              return;
            }
            
            setAuthing(false);
            if(device !== null) {
              setChosenDevice(device);
              await device.transport.disconnected;
              setChosenDevice(null);
            }
          }}>Connect to Quest</button>

          <ErrorModal isVisible={connectError != null}
            title={"Failed to connect to device"}
            description={connectError!}
            onClose={() => setConnectError(null)}/>
        </div>
      </>
  }
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
    </h1>
    <p>The easiest way to install custom songs for Beat Saber on Quest!</p>
  </>
}

function AppContents() {
  if (navigator.usb === undefined) {
    return UnsupportedMessage();
  } else {
    return ChooseDevice();
  }
}

function App() {
  return <div className='main'>
    <AppContents />
    <ToastContainer
      position="bottom-right"
      theme="dark"
      autoClose={5000}
      transition={Bounce}
      hideProgressBar={true} />
  </div>
}

function isViewingOnWindows(): boolean {
  // Deprecated but still works for our purposes.
  return navigator.appVersion.indexOf("Win") != -1;
}

function isViewingOnMobile() {
  return /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(navigator.userAgent);
}

// Kindly provided by Pierre
// https://stackoverflow.com/a/9039885
function isViewingOnIos() {
  return [
    'iPad Simulator',
    'iPhone Simulator',
    'iPod Simulator',
    'iPad',
    'iPhone',
    'iPod'
    // This is deprecated but still provides a good way to detect iOS as far as the author is concerned.
    // We are also doing feature detection for WebUSB, but detecting iOS provides a good way to warn the user that no iOS browsers will work with this app.
  ].includes(navigator.platform)
  // iPad on iOS 13 detection
  || (navigator.userAgent.includes("Mac") && "ontouchend" in document)
}

function UnsupportedMessage() {
  return <div className='container mainContainer'>
    {isViewingOnIos() ? <>
      <h1>iOS is not supported</h1>
      <p>MBF has detected that you're trying to use it from an iOS device. Unfortunately, Apple does not allow WebUSB, which MBF needs to be able to interact with the Quest.</p>
      <p>To mod your game, you will need one of: </p>
      <ul>
        <li>A PC or Mac (preferred)</li>
        <li>An Android phone (still totally works)</li>
      </ul>

      <p>.... and one of the following supported browsers:</p>
    </> : <>
      <h1>Browser Unsupported</h1>
      <p>It looks like your browser doesn't support WebUSB, which this app needs to be able to access your Quest's files.</p>
    </>}

    <h2>Supported Browsers</h2>
    <SupportedBrowsers />
  </div>
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
