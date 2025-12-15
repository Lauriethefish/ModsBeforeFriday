/*
How to add an item to localization text:
1. add something to Eng object
2. use getLang() from "src/localization/shared.ts" to access the localized Eng object
*/

export const Eng = {
    __proto__: null,

    sourceCode: <>Source Code</>,
    titleText: <>The easiest way to install custom songs for Beat Saber on Quest!</>,
    toGetStart:
        <>
        <p>To get started, plug your Quest in with a USB-C cable and click the button below.</p>
        <p>Want see what mods are available? You can find a full list <a href="https://mods.bsquest.xyz" target="_blank" rel="noopener noreferrer">here!</a></p>
        </>
    ,

    allowConnectionInHeadSet:<>
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
    </>,

    otherAppIsAccessQuest: <>Some other app is trying to access your Quest, e.g. SideQuest.</>,

    killAdb:<>
        <p>To fix this, close SideQuest if you have it open, press <span className="codeBox">Win + R</span> and type the following text, and finally press enter.</p>
        <span className="codeBox">taskkill /IM adb.exe /F</span>  
        <p>Alternatively, restart your computer.</p>
    </>,

    fixWithRestartDevice:(isViewingOnMobile:boolean)=>
        <>
        To fix this, restart your {isViewingOnMobile ? "phone" : "computer"}.
        </>
    ,

    questBrowserMessage:<>
        <h1>Quest Browser Detected</h1>
        <p>MBF has detected that you're trying to use the built-in Quest browser.</p>
        <p>Unfortunately, <b>you cannot use MBF on the device you are attempting to mod.</b></p>
    </>,

    onlyWorkWithAnotherQuest:<>(MBF can be used on a Quest if you install a chromium browser, however this can only be used to mod <b>another Quest headset</b>, connected via USB.)</>,

    connectToQuest:<>Connect to Quest</>,

    deviceSupportingModding:<>
        <p>To mod your game, you will need one of: </p>
        <ul>
          <li>A PC or Mac (preferred)</li>
          <li>An Android phone (still totally works)</li>
        </ul>
    </>,

    iosNotSupported:<>
        <h1>iOS is not supported</h1>
        <p>MBF has detected that you're trying to use it from an iOS device. Unfortunately, Apple does not allow WebUSB, which MBF needs to be able to interact with the Quest.</p>
    </>,

    supportedBrowserHintInIOS:<>
        .... and one of the following supported browsers:
    </>,

    browserNotSupported:<>
        <h1>Browser Unsupported</h1>
        <p>It looks like your browser doesn't support WebUSB, which this app needs to be able to access your Quest's files.</p>
    </>,
    supportedBrowserTitle:<>Supported Browsers</>,

    supportedBrowserMobile:<>
        <ul>
            <li>Google Chrome for Android 122 or newer</li>
            <li>Edge for Android 123 or newer</li>
        </ul>
        <h3 className='fireFox'>Firefox for Android is NOT supported</h3>
    </>,
    supportedBrowserNotMobile:<>
        <ul>
            <li>Google Chrome 61 or newer</li>
            <li>Opera 48 or newer</li>
            <li>Microsoft Edge 79 or newer</li>
        </ul>
        <h3 className='fireFox'>Firefox and Safari are NOT supported.</h3>
        <p>(There is no feasible way to add support for Firefox as Mozilla have chosen not to support WebUSB for security reasons.)</p>
    </>,

    noCompatableDevice:<>
        <h3>No compatible devices?</h3>
    
        <p>
          To use MBF, you must enable developer mode so that your Quest is accessible via USB.
          <br />Follow the <a href="https://developer.oculus.com/documentation/native/android/mobile-device-setup/?locale=en_GB" target="_blank" rel="noopener noreferrer">official guide</a> -
          you'll need to create a new organisation and enable USB debugging.
        </p>
    </>,
    noCompatableDeviceMobile:<>
        <h4>Using Android?</h4>
        <p>It's possible that the connection between your device and the Quest has been set up the wrong way around. To fix this:</p>
        <ul>
            <li>Swipe down from the top of the screen.</li>
            <li>Click the dialog relating to the USB connection. This might be called "charging via USB".</li>
            <li>Change "USB controlled by" to "Connected device". If "Connected device" is already selected, change it to "This device" and change it back.</li>
        </ul>
        <h4>Still not working?</h4>
        <p>Try unplugging your cable and plugging the end that's currently in your phone into your Quest.</p>
    </>,

    chooseCoreModUrl:<>
        <h1>Manually override core mod JSON</h1>
        <p>Please specify a complete URL to the raw contents of your core mod JSON</p>
    </>,

    confirmUrl:<>Confirm URL</>,

    notInstalled: <>
        <h1>Beat Saber is not installed</h1>
        <p>Please install Beat Saber from the store and then refresh the page.</p>
        <h3>Think you have Beat Saber installed?</h3>
        <p>Sometimes, it looks like Beat Saber is installed in your headset, when it actually isn't (a bug in the Meta software).</p>
        <p>This can be fixed by going to the main <b>Applications</b> menu inside your Quest, clicking the 3 dots next to Beat Saber, and clicking <b>Uninstall</b>. Finally, reinstall Beat Saber from the Meta store and refresh this page to try again.</p>
    </>,

    noInternet: <>
        <h1>No internet</h1>
        <p>It seems as though <b>your Quest</b> has no internet connection.</p>
        <p>To mod Beat Saber, MBF needs to download files such as a mod loader and several essential mods.
            <br />This occurs on your Quest's connection. Please make sure that WiFi is enabled, then refresh the page.</p>
    </>,

    modCompatable: <>Your Beat Saber install is modded, and its version is compatible with mods.</>,

    modUpdateAvaliable: <>However, an updated moddable version is available! </>,

    clickHereToUpdate: <>Click here to update</>,
    updateBeatSaberHint: <>
        <h2>Update Beat Saber</h2>
        <p>To update to the latest moddable version, simply:</p>
        <ol>
            <li>Uninstall Beat Saber with the button below.</li>
            <li>Reinstall Beat Saber in your headset.</li>
            <li>Open back up MBF to mod the version you just installed.</li>
        </ol>
    </>,

    uninstallBeatSaber: <>
        Uninstall Beat Saber
    </>,
    cancel: <>Cancel</>,

    uninstallAboutMapThings: <>
        <h3>What about my maps/mods/scores/qosmetics?</h3>
        <ul>
            <li><em>Maps and scores will remain safe</em> as they are held in a separate folder to the game files, so will not be deleted when you uninstall.</li>
            <li>Qosmetics will not be deleted, however you can only use them if the qosmetics mods are available for the new version. You can always move back to your current version later if you miss them.</li>
            <li><em>All currently installed mods will be removed.</em> (the new core mods for the updated version will be installed automatically) You can reinstall them once you have updated (if they are available for the newer version)</li>
        </ul>
    </>,

    loadingDowngradedApk: <>
        <h2>Loading downgraded APK manifest</h2>
        <p>This should only take a few seconds.</p>
    </>,

    modWarning: <>
        <h2 className='warning'>READ CAREFULLY</h2>
        <p>Mods and custom songs are not supported by Beat Games. You may experience bugs and crashes that you wouldn't in a vanilla game.</p>
    </>,

    permissions: <>Permissions</>,

    modTheApp: <>Mod the app</>,

    changePermissionHint: <>
        <h2>Change Permissions</h2>
        <p>Certain mods require particular Android permissions to be set on the Beat Saber app in order to work correctly.</p>
        <p>(You can change these permissions later, so don't worry about enabling them all now unless you know which ones you need.)</p>
    </>,
    changePermHintInOptionsMenu:<>
        <p>Certain mods require particular Android permissions to be enabled in order to work. 
            To change the permisions, you will need to re-patch your game, which can be done automatically with the button below.</p>
    </>,
    confirmPermission: <>
        Confirm permissions
    </>,

    appPatchedHint: <>
        <h1>App is being patched</h1>
        <p>This should only take a few minutes, but could take much, much longer if your internet connection is slow.</p>
    </>,

    dontDisconnectDeviceHint: <>You must not disconnect your device during this process.</>,

    chooseDifferentGameVersionHint: <>
        <h2>Choose a different game version</h2>
        <p>Using this menu, you can make MBF downgrade to a version other than the latest moddable version.</p>
        <p>This is not recommended, and you should only do it if there are specific mods you wish to use that aren't available on the latest version.</p>
        <p><b>Please note that MBF is not capable of modding Beat Saber 1.28 or lower.</b></p>
        <p>Click a version then confirm the downgrade:</p>
    </>,
    useLatestModdable: <>Use latest moddable</>,

    confirmDowngrade: <>Confirm downgrade</>,

    versionSupportedMessageTitle: <>Install Custom Songs</>,

    versionSupportedHint:(version:string)=>
        <>Your app has version {version}, which is supported by mods!</>
    ,

    chooseAnotherVersion: <>(choose another version)</>,

    versionSupportedInstallEssentialMods: <>
        <p>To get your game ready for custom songs, ModsBeforeFriday will next patch your Beat Saber app and install some essential mods.
            Once this is done, you will be able to manage your custom songs <b>inside the game.</b></p>
    </>,

    updateAndSetupMods: <>Update and set up mods</>,
    downgradeAndSetupMods: <>Downgrade and set up mods</>,

    modDevelopmentWarn:<>Mod development mode engaged: bypassing version check.
                This will not help you unless you are a mod developer!</>,
    
    olderThanLatestModdableHint: <>You have decided to change to a version older than the latest moddable version. <b>Only do this if you know why you want to!</b></>,

    reverseDecision: <>(reverse decision)</>,

    quest1ModHint: (version:string)=>
        <>MBF has detected that you're running on a Quest 1. To get the latest mods, MBF will automatically update Beat Saber to the latest moddable version ({version}).
                Even though Meta only officially supports Quest 1 up to Beat Saber v1.36.2, MBF can patch version  {version} so it still works!
            </>
    ,
    doesntSupportMods: <>MBF has detected that your version of Beat Saber doesn't support mods!</>,

    canDowngrateToVersion: (version:string)=>
        <>Fortunately for you, your version can be downgraded automatically to the latest moddable version: {version}</>
    ,

    unsupportedVersion: <>Unsupported Version</>,
    readThisMessage: <>Read this message in full before asking for help if needed!</>,
    notSupportedModsText: (version:string, isLegacy:boolean)=>
        <>You have Beat Saber v{version} installed, but this version has no support for {isLegacy ? "modding with MBF" : "mods"}!</>
    ,

    legacyUpdateRecommand:<>
        <p>While your version might work with other modding tools, it is <b>very strongly recommended</b> that you uninstall Beat Saber and update
            to the latest moddable version.</p>
        <p className="warning">Modding with versions 1.28.0 or below is <em>no longer supported by BSMG</em> - it's an ancient version and nobody should be using it anymore. Please, please, <em>PLEASE</em> update to the latest version of the game.</p>
    </>,

    normallyUpdateRecommand:<>
        <p>Normally, MBF would attempt to downgrade (un-update) your Beat Saber version to a version with mod support, but this is only possible if you have the latest version of Beat Saber installed.</p>
        <p>Please uninstall Beat Saber using the button below, then reinstall the latest version of Beat Saber using the Meta store.</p>
    </>,

    awaitingPatchGeneration: <>Awaiting patch generation</>,

    mustReadMessageFull:<>You must READ this message IN FULL.</>,

    noDiffMessageBody: (version:string)=>
        <>
            <p>You have Beat Saber v{version}, which has no support for mods.</p>
            <p>MBF is designed to downgrade (un-update) your Beat Saber version to a version with mod support <b>but the necessary patches have not yet been generated,</b> as a Beat Saber update has only just been released.</p>
            <p>Patch generation needs manual input and <b>will happen as soon as the author of MBF is available</b>, which will take from 30 minutes to 24 hours.</p>
            <p><b>PLEASE WAIT in the meanwhile.</b> You can refresh this page and reconnect to your Quest to check if the patch has been generated.</p>
        </>
    ,

    incompatableModLoader: (modLoader:string)=>
        <>
            <h1>Incompatible Modloader</h1>
            <p>Your app is patched with {modLoader === 'QuestLoader' ? "the QuestLoader" : "an unknown"} modloader, which isn't supported by MBF.</p>
            <p>You will need to uninstall your app and reinstall the latest vanilla version so that the app can be re-patched with Scotland2.</p>
            <p>Do not be alarmed! Your custom songs will not be lost.</p>
        </>
    ,

    incompatableVersionPatched: (version:string)=><>
            <h1>Incompatible Version Patched</h1>

            <p>Your Beat Saber app has a modloader installed, but the game version ({version}) has no support for mods!</p>
            <p>To fix this, uninstall Beat Saber and reinstall the latest version. MBF can then downgrade this automatically to the latest moddable version.</p>
        </>
    ,

    nextSteps: <ul>
        <li>Load up the game and look left. A menu should be visible that shows your mods.</li>
        <li>Click the <b>"SongDownloader"</b> mod and browse for custom songs in-game.</li>
        <li>Download additional mods below!</li>
    </ul>,

    obbNotPresent:<>
        <h1>OBB not present</h1>
        <p>MBF has detected that the OBB file, which contains asset files required for Beat Saber to load, is not present in the installation.</p>
        <p>This means your installation is corrupt. You will need to uninstall Beat Saber with the button below, and reinstall the latest version from the Meta store.</p>
    </>,

    checkInstall: <>Checking Beat Saber installation</>,
    mightTakeFewTimes: <>This might take a minute or so the first few times.</>,

    appIsModded: <>App is modded</>,

    coreModDisabled:<>Core mod functionality is disabled.</>,

    notSureNext:<>Not sure what to do next?</>,

    everythingReady:<>Everything should be ready to go!</>,

    problemFound:<>Problems found with your install:</>,
    problemCanFix:<>These can be easily fixed by clicking the button below.</>,
    modloaderNotFound:<>Modloader not found</>,
    modloaderNeedUpdate:<>Modloader has an available update</>,
    coremodsMissing:<>Not all the core mods are installed</>,
    coreModsNeedUpdate:<>Core mod updates need to be installed.</>,
    fixIssue:<>Fix issues</>,


    settings:<>Settings</>,
    credits:<>Credits</>,

    creditsIntro:(SourceUrl:string)=><>
        <p>Hi, it's <b>Lauriethefish</b> here, the original author of ModsBeforeFriday.</p>
        <p>MBF is an <a href={SourceUrl}>open source project</a>, and over the course of development, numerous people have stepped up to improve the app.</p>
        <p>It is important to remember that MBF is just <em>installing</em> your mods. There are many very talented people behind the core mods that MBF installs,
        and unless you've been paying close attention to the mod list, you won't even know many of their names!</p>
        <p>This menu solely focuses on people who have contributed to the MBF app.</p>
    </>,
    mbfContributors:<>MBF contributors</>,

    contributorIntroFrozenAlex:<> created the drag 'n' drop system for MBF, and has provided me with much insight on UI design. Without him, the UI would be (even more of) a cluttered mess!</>,
    contributorXoToM:<>, a good friend of mine, created the animated background that you know and love. (although your CPU might hate it!)</>,
    contributorAltyFox:<>, a member of the BSMG support team, has provided invaluable feedback regarding usability, and has helped me to pinpoint and fix bugs.</>,

    contributorLocalization:<>{/* nothing here, place holder for localization translators*/}</>,

    creditsOkBtnText:<>OK</>,

    showAnimatedBackground:<>Show animated background</>,

    Logs:<>Logs</>,

    EditXML:<>Edit XML</>,
    SimpleOptions:<>Simple options</>,
    AdvancedOptions:<>Advanced Options</>,

    changeManifestXmlHint:<>
        <h2>Change manifest XML</h2>
        <p>For development purposes, this menu will allow you to manually edit the entirety of the AndroidManifest.xml file within the APK</p>
        <p>Be careful, as erroneous edits will prevent the APK from installing properly.</p>
    </>,
    downloadCurrentXML:<>Download Current XML</>,
    uploadXML:<>Upload XML</>,
    backBtnText:<>Back</>,

    permMicrophone:<>Microphone Access</>,
    permPassthrough:<>Passthrough to headset cameras</>,
    permBody:<>Body tracking</>,
    permHand:<>Hand tracking</>,
    permBluetooth:<>Bluetooth</>,
    permMRC:<>MRC workaround</>,

    
    permMenuPermissions:<>Permissions</>,
    permMenuFeatures:<>Features</>,

    deviceInUse:<>Device in use</>,
    failedToConnectDevice:<>Failed to connect to device</>,

    // I might regret this
    askLaurie:<><p>If you can't fix the issue, PLEASE hit up <code>Lauriethefish</code> on Discord for further support. We're working on fixing connection/driver issues
      right now and can only do so with <i>your help!</i></p></>,

    failedToFetchManifestHint:"Failed to fetch AndroidManifest.xml for the selected downgrade version. Did your quest lose its internet connection suddenly?",

    yourMods:<>Your Mods</>,
    addMods:<>Add Mods</>,
    uploadFiles:<>Upload Files</>,

    installModHint:(hasUpdate:boolean, hasNewMod:boolean, modCount:number)=><>
        {hasUpdate && hasNewMod && "Install/Update "}
        {hasUpdate && !hasNewMod && "Update "}
        {!hasUpdate && hasNewMod && "Install "}
        {modCount} {modCount > 1 ? "mods" : "mod"}
    </>,

    updateBtnText:<>Update</>,
    installBtnText:<>Install</>,
    sourceCodeBtnText:<>Source Code</>,
    reportBugBtnText:<>Report bug</>,

    coreBadgeText:<>CORE</>,

    optionMenuModTools:<>Mod tools</>,
    optionMenuAdbLog:<>ADB log</>,
    optionMenuChangePerm:<>Change Permissions/Repatch</>,

    optKillBeatSaber:<>Kill Beat Saber</>,
    optKillBeatSaberDesc:<>Immediately closes the game.</>,
    beatsaberKilled:<>Successfully killed Beat Saber</>,

    optRestartBeatSaber:<>Restart Beat Saber</>,
    optRestartBeatSaberDesc:<>Immediately closes and restarts the game.</>,
    beatsaberRestarted:<>Successfully restarted Beat Saber</>,
    optReinstallCore:<>Reinstall only core mods</>,
    optReinstallCoreDesc:<>Deletes all installed mods, then installs only the core mods.</>,
    reinstallOnlyCoreMods:"Reinstalling only core mods",
    failedToReinstallOnlyCoreMods:"Failed to reinstall only core mods",
    allNonCoreRemoved:<>All non-core mods removed!</>,

    optUninstallBeatsaber:<>Uninstall Beat Saber</>,
    optUninstallBeatsaberDesc:<>Uninstalls the game: this will remove all mods and quit MBF.</>,

    optFixPlayerData:<>Fix Player Data</>,
    optFixPlayerDataDesc:<>Fixes an issue with player data permissions.</>,
    optFixPlayerDataSuccess:<>Successfully fixed player data issues</>,
    optFixPlayerDataNoData:<>No player data file found to fix</>,

    failedToKillBeatsaber:"Failed to kill Beat Saber process",
    failedToUninstall:"Failed to uninstall Beat Saber",
    failedToFixPlayerData:"Failed to fix player data",

    author_by:<>by </>,

    repatchGame:<>Repatch game</>,

    optionsMenuAdbLogHint:<>
        <p>This feature allows you to get a log of what's going on inside your Quest, useful for modders to fix bugs with their mods.</p>
        <p>Click the button below, <span className="warning">and keep your headset plugged in.</span> Open the game and do whatever it is that was causing you issues, then click the button again.</p>
    </>,

    startLogging:<>Start Logging</>,
    stopLogging:<>Stop Logging</>,
    waitingForLog:<>Please wait while the log file generates . . .</>,
    downloadLog:<>Download Log</>,
    selectSplashScreen:<>Select splash screen</>,
    usingSplash:(name:string)=><>(Using <code className="codeBox">{name}</code> as splash)</>,

    logOutput:<>Log output</>
}