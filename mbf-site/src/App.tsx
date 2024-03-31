import { useState } from 'react';

import './App.css';
import { AdbDaemonWebUsbConnection, AdbDaemonWebUsbDeviceManager } from '@yume-chan/adb-daemon-webusb';
import { AdbDaemonTransport, Adb } from '@yume-chan/adb';

import AdbWebCredentialStore from "@yume-chan/adb-credential-web";
import DeviceModder from './DeviceModder';

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
    alert("Some other app is trying to access your quest, e.g. SideQuest." + 
    " Please restart your computer to close any SideQuest background processes, then try again. \n(" + err + ")");
    return null;
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

  if(chosenDevice !== null) {
    return <>
      <DeviceModder device={chosenDevice} />
    </>
  } else if(authing) {
    return <div className='container'>
      <h2>Authenticating</h2>
      <p>Put on your headset and check the box to give MBF access to your Quest, then press "OK"</p>
      <p>You will only have to do this once.</p>
      <img src="allowDebugging.png"/>
    </div>
  } else  {
    return <>
        <div className="container mainContainer">
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
          <p>To get started, plug your Quest in with a USB-C cable and click the button below.</p>

          <h3>No compatible devices?</h3>

          <p>
            To use MBF, you must enable developer mode so that your Quest is accessible via USB.
            <br />Follow the <a href="https://developer.oculus.com/documentation/native/android/mobile-device-setup/?locale=en_GB">official guide</a> -
            you'll need to create a new organisation and enable USB debugging.
          </p>

          <button id="chooseDevice" onClick={async () => {
            const device = await connect(() => setAuthing(true));
            if(device !== null) {
              setChosenDevice(device);
            }
          }}>Connect to Quest</button>
        </div>
      </>
  }
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
  </div>
}

function isViewingOnMobile() {
  return (navigator.maxTouchPoints ?? 0) > 0;
}

function UnsupportedMessage() {
  return <div className='container'>
    <h1>Browser Unsupported</h1>
    <p>It looks like your browser doesn't support WebUSB, which this app needs to be able to access your Quest's files.</p>

    <h2>Supported Browsers</h2>
    <SupportedBrowsers />
  </div>
}

function SupportedBrowsers() {
  if(isViewingOnMobile()) {
    return <>
      <ul>
        <li>Google Chrome for Android 122 or newer</li>
        <li>Opera Mobile 80 or newer</li>
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
      <h3 className='fireFox'>Firefox is NOT supported.</h3>
      <p>(There is no feasible way to add support for Firefox as Mozilla have chosen not to support WebUSB for security reasons.)</p>
    </>
  }
}

export default App;
