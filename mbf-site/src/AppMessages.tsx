import { SourceUrl } from "."
import { isViewingOnWindows, isViewingOnMobile, isViewingOnIos } from "./platformDetection"

/**
 * Displays a message when an outdated (pre-v51) Quest OS version is detected.
 * Informs the user that mods are not supported on this OS version and prompts them to update.
 */
export function OldOsVersion() {
    return <div className="container mainContainer">
      <h1>Pre-v51 OS Detected</h1>
      <p>ModsBeforeFriday has detected that you have an outdated version of the Quest operating system installed which is no longer supported by mods.</p>
      <p>Please ensure your operating system is up to date and then refresh the page.</p>
    </div>
  }

  /**
   * Displays a message when a Quest 1 device is detected.
   * Explains that Quest 1 is not supported and provides a link to modding instructions for Quest 1.
   */
  export function QuestOneNotSupported() {
    return <div className='container mainContainer'>
      <h1>Quest 1 Not Supported</h1>
      <p>ModsBeforeFriday has detected that you're using a Quest 1, which is not supported by MBF. (and never will be)</p>
      <p>This is because Quest 1 uses different builds of the Beat Saber game and so mods are stuck forever on version 1.28.0 of the game.</p>
      <p>Follow <a href="https://bsmg.wiki/quest/modding-quest1.html">this link</a> for instructions on how to set up mods on Quest 1.</p>
    </div>
  }

  /**
   * Displays instructions for allowing ADB authentication in the headset.
   * Guides the user through troubleshooting steps if the prompt does not appear.
   */
  export function AllowAuth() {
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
  }

  /**
   * Displays troubleshooting steps when the Quest device is in use by another application.
   * Provides platform-specific instructions for resolving ADB conflicts.
   */
  export function DeviceInUse() {
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

  /**
   * Renders the main title and subtitle for the ModsBeforeFriday app.
   * Includes a link to the source code and a tagline.
   */
  export function Title() {
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

  /**
   * Displays a message when the user is accessing MBF from the built-in Quest browser.
   * Informs the user that modding the current device is not possible from its own browser,
   * and suggests using a Chromium-based browser to mod a different Quest device via USB.
   * Also lists compatible device types for modding.
   */
  export function OculusBrowserMessage() {
    return <div className="container mainContainer">
      <h1>Quest Browser Detected</h1>
      <p>MBF has detected that you're trying to use the built-in Quest browser.</p>
      <p>Unfortunately, <b>you cannot use MBF on the device you are attempting to mod.</b></p>
      <DevicesSupportingModding />

      <p>(MBF can be used on a Quest if you install a chromium browser, however this can only be used to mod <b>another Quest headset</b>, connected via USB.)</p>
    </div>
  }

  /**
   * Displays a message when the user's browser or device does not support WebUSB.
   * Shows a specific message for iOS devices (which do not support WebUSB at all),
   * and a generic message for unsupported browsers. Also lists supported browsers.
   */
  export function UnsupportedMessage() {
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

  /**
   * Lists the types of devices that can be used to mod a Quest headset with MBF.
   * Recommends a PC or Mac, but also mentions Android phones as a working option.
   */
  export function DevicesSupportingModding() {
    return <>
      <p>To mod your game, you will need one of: </p>
      <ul>
        <li>A PC or Mac (preferred)</li>
        <li>An Android phone (still totally works)</li>
      </ul>
    </>
  }

  /**
   * Lists browsers that support WebUSB and are compatible with MBF.
   * Shows different lists for mobile and desktop, and warns about unsupported browsers.
   */
  export function SupportedBrowsers() {
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

  /**
   * Displays troubleshooting steps when no compatible devices are detected.
   * Instructs the user to enable developer mode and USB debugging on their Quest,
   * and provides additional USB troubleshooting tips for Android users.
   */
  export function NoCompatibleDevices() {
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
