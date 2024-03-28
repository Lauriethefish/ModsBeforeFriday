import React, { useEffect, useState } from 'react';

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
    alert("Please select a device");
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
      <button onClick={() => {
        setChosenDevice(null);
        setAuthing(false);
      }}>Choose another device</button>
    </>
  } else if(authing) {
    return <>
      <h2>Authenticating</h2>
      <p>Put on your headset and click "Always allow" to give ModsBeforeFriday access.</p>
    </>
  } else  {
    return <>
      <h1>ModsBeforeFriday</h1>
      <p>The easiest way to install custom songs for Beat Saber on Quest!</p>

      <p>To get started, plug your Quest in with a USB cable and click the button below.</p>
      <button onClick={async () => {
        const device = await connect(() => setAuthing(true));
        if(device !== null) {
          setChosenDevice(device);
        }
      }}>Choose device to mod</button>
    </>
  }
}

function App() {
  if (navigator.usb === undefined) {
    return UnsupportedMessage();
  } else {
    return ChooseDevice();
  }
}

function isViewingOnMobile() {
  return (navigator.maxTouchPoints ?? 0) > 0;
}

function UnsupportedMessage() {
  return <>
    <h1>Browser Unsupported</h1>
    <p>It looks like your browser doesn't support WebUSB, which this app needs to be able to access your Quest's files.</p>

    <h2>Supported Browsers</h2>
    <SupportedBrowsers />
  </>
}

function SupportedBrowsers() {
  if(isViewingOnMobile()) {
    return <>
      <ul>
        <li>Google Chrome for Android 122 or newer</li>
        <li>Opera Mobile 80 or newer</li>
      </ul>
      <h3>Firefox for Android is NOT supported</h3>
    </>
  } else  {
    return <>
      <ul>
        <li>Google Chrome 61 or newer</li>
        <li>Opera 48 or newer</li>
        <li>Microsoft Edge 79 or newer</li>
      </ul>
      <h3>Firefox is NOT supported</h3>
    </>
  }
}

export default App;
