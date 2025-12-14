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
        <p>To get started, plug your Quest in with a USB-C cable and click the button below.</p>
    ,

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

    versionSupportedHint(version:string) {
        return <>Your app has version {version}, which is supported by mods!</>
    },

    chooseAnotherVersion: <>(choose another version)</>,

    versionSupportedInstallEssentialMods: <>
        <p>To get your game ready for custom songs, ModsBeforeFriday will next patch your Beat Saber app and install some essential mods.
            Once this is done, you will be able to manage your custom songs <b>inside the game.</b></p>
    </>,

    update: "Update",
    downgrade: "Downgrade",

    andSetupMods: <>and set up mods</>,

    modDevelopmentWarn:<>Mod development mode engaged: bypassing version check.
                This will not help you unless you are a mod developer!</>,
    
    olderThanLatestModdableHint: <>You have decided to change to a version older than the latest moddable version. <b>Only do this if you know why you want to!</b></>,

    reverseDecision: <>(reverse decision)</>,

    quest1ModHint(version:string){
        return <>MBF has detected that you're running on a Quest 1. To get the latest mods, MBF will automatically update Beat Saber to the latest moddable version ({version}).
                Even though Meta only officially supports Quest 1 up to Beat Saber v1.36.2, MBF can patch version  {version} so it still works!
            </>
    },
    doesntSupportMods: <>MBF has detected that your version of Beat Saber doesn't support mods!</>,

    canDowngrateToVersion(version:string){
        return <>Fortunately for you, your version can be downgraded automatically to the latest moddable version: {version}</>
    },

    unsupportedVersion: <>Unsupported Version</>,
    readThisMessage: <>Read this message in full before asking for help if needed!</>,
    notSupportedModsText(version:string, isLegacy:boolean){
        return <>You have Beat Saber v{version} installed, but this version has no support for {isLegacy ? "modding with MBF" : "mods"}!</>
    },

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

    noDiffMessageBody(version:string){
        return <>
            <p>You have Beat Saber v{version}, which has no support for mods.</p>
            <p>MBF is designed to downgrade (un-update) your Beat Saber version to a version with mod support <b>but the necessary patches have not yet been generated,</b> as a Beat Saber update has only just been released.</p>
            <p>Patch generation needs manual input and <b>will happen as soon as the author of MBF is available</b>, which will take from 30 minutes to 24 hours.</p>
            <p><b>PLEASE WAIT in the meanwhile.</b> You can refresh this page and reconnect to your Quest to check if the patch has been generated.</p>
        </>
    },

    incompatableModLoader(modLoader:string){
        return <>
            <h1>Incompatible Modloader</h1>
            <p>Your app is patched with {modLoader === 'QuestLoader' ? "the QuestLoader" : "an unknown"} modloader, which isn't supported by MBF.</p>
            <p>You will need to uninstall your app and reinstall the latest vanilla version so that the app can be re-patched with Scotland2.</p>
            <p>Do not be alarmed! Your custom songs will not be lost.</p>
        </>
    },

    incompatableVersionPatched(version:string){
        return <>
            <h1>Incompatible Version Patched</h1>

            <p>Your Beat Saber app has a modloader installed, but the game version ({version}) has no support for mods!</p>
            <p>To fix this, uninstall Beat Saber and reinstall the latest version. MBF can then downgrade this automatically to the latest moddable version.</p>
        </>
    },

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

    showAnimatedBackground:<>Show animated background</>
}