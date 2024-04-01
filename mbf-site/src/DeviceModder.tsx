import { Adb } from '@yume-chan/adb';
import { prepareAgent, runCommand } from "./Agent";
import { useEffect, useState } from 'react';
import { ModStatus } from './Messages';
import './DeviceModder.css';
import { ModCard } from './ModCard';
import { ReactComponent as ModIcon } from './mod-icon.svg'

interface DeviceModderProps {
    device: Adb
}

async function loadModStatus(device: Adb) {
    await prepareAgent(device);

    return await runCommand(device, {
        type: 'GetModStatus'
    }) as unknown as ModStatus;
}

async function patchApp(device: Adb) {
    await runCommand(device, {
        type: 'Patch'
    });
}

function DeviceModder(props: DeviceModderProps) {
    const [modStatus, setModStatus] = useState(null as ModStatus | null);
    const [isPatching, setIsPatching] = useState(false);

    useEffect(() => {
        loadModStatus(props.device).then(data => setModStatus(data));
    }, [props.device]);

    if(isPatching) {
        return <div className='container mainContainer'>
            <h3>App is being patched.</h3>
            <p>This should only take a few seconds, but might take longer on a very slow connection.</p>
        </div>;
    } else if(modStatus === null) {
        return <div className='container mainContainer'>
            <h2>Checking Beat Saber installation</h2>
        </div>;
    }   else if(modStatus.app_info === null) {
        return <div className='container mainContainer'>
            <h1>Beat Saber is not installed</h1>
            <p>Please install Beat Saber from the store and then refresh the page.</p>
        </div>
    }   else if (modStatus.core_mods === null) {
        return <div className='container mainContainer'>
            <h1>No internet</h1>
            <p>It seems as though <b>your Quest</b> has no internet connection.</p>
            <p>To mod Beat Saber, MBF needs to download files such as a mod loader and several essential mods. 
                <br />This occurs on your Quest's connection. Please make sure that WiFi is enabled, then refresh the page.</p>
        </div>
    }   else if(!(modStatus.core_mods.supported_versions.includes(modStatus.app_info.version))) {
        return <NotSupported version={modStatus.app_info.version} supportedVersions={modStatus.core_mods.supported_versions}/>
    }   else if(modStatus.app_info.is_modded)   {
        return <>
            <div className='container mainContainer'>
                <h1>App is modded</h1>
                <p>Beat Saber is already modded on your Quest, and the version that's installed is compatible with mods.</p>

                <InstallStatus modloaderReady={modStatus.modloader_present} coreModsReady={modStatus.core_mods.all_core_mods_installed} />
            </div>

            <div className='container horizontalCenter'>
             <h1>Installed Mods</h1><ModIcon stroke="white"/>
            </div>

            {modStatus.installed_mods.map(mod => <ModCard mod={mod} key={mod.id}/>)}
        </>
    }   else    {
        return <PatchingMenu version={modStatus.app_info.version} onPatch={async () => {
            setIsPatching(true);
            try {
                await patchApp(props.device);
                modStatus.app_info!.is_modded = true;
                setModStatus(modStatus);
            }   finally {
                setIsPatching(false);
            }
        }}/>
    }
}

interface NotSupportedProps {
    version: string,
    supportedVersions: string[]
}

function NotSupported(props: NotSupportedProps) {
    return <div className='container mainContainer'>
        <h1>Unsupported Version</h1>
        <p>You have Beat Saber v{props.version} installed, but this version has no support for mods!</p>
        <p>To install custom songs, one of the following versions is needed:</p>
        <ul>
            {props.supportedVersions.map(ver => <li>{ver}</li>)}
        </ul>
    </div>
}

interface InstallStatusProps {
    modloaderReady: boolean,
    coreModsReady: boolean
}

function InstallStatus(props: InstallStatusProps) {
    if(props.modloaderReady && props.coreModsReady) {
        return <p>Everything should be ready to go! &#9989;</p>
    }   else {
        return <div>
            <h3 className="warning">Problems found with your install:</h3>
            <p>These must be fixed before custom songs will work!</p>
            <ul>
                {!props.modloaderReady && <li>Modloader not found &#10060;</li>}
                {!props.coreModsReady && <li>Core mods missing or out of date &#10060;</li>}
            </ul>
            <button>Fix issues</button>
        </div>
    }
}

interface PatchingMenuProps {
    version: string,
    onPatch: () => void
}

function PatchingMenu(props: PatchingMenuProps) {
    return <div className='container mainContainer'>
        <h1>Install Custom Songs</h1>
        <p>Your app has version: {props.version}, which is supported by mods!</p>
        <p>To get your game ready for custom songs, ModsBeforeFriday will next patch your Beat Saber app and install some essential mods.
        Once this is done, you will be able to manage your custom songs <b>inside the game.</b></p>

        <h2 className='warning'>READ CAREFULLY</h2>
        <p>Mods and custom songs are not supported by Beat Games. You may experience bugs and crashes that you wouldn't in a vanilla game.</p>
        <b>In addition, by modding the game you will lose access to both vanilla leaderboards and vanilla multiplayer.</b> (Modded leaderboards/servers are available.)

        <button className="modButton" onClick={props.onPatch}>Mod the app</button>
    </div>
}


export default DeviceModder;