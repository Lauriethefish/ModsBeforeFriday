import { Adb } from '@yume-chan/adb';
import { prepareAgent, runCommand } from "./Agent";
import { useEffect, useState } from 'react';
import { ModStatus } from './Messages';

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
        return <>
            <h3>App is being patched</h3>
            <p>This should only take a few seconds</p>
        </>
    } else if(modStatus === null) {
        return <>
            <h2>Loading (please wait)</h2>
        </>;
    }   else if(modStatus.app_info === null) {
        return <>
            <h1>Beat Saber is not installed</h1>
        </>
    }   else if(modStatus.app_info.is_modded)   {
        return <>
            <h1>App is modded!</h1>
        </>
    }   else    {
        return <>
            <h1>Install Custom Songs</h1>
            <p>Your app has version: {modStatus.app_info.version}, which is supported by mods!</p>
            <p>To get your game ready for custom songs, ModsBeforeFriday will next patch your Beat Saber app and install some essential mods that will load the songs into the game. Once this is done, you will be able to manage your custom songs <b>inside the game.</b></p>

            <button onClick={async () => {
                setIsPatching(true);
                try {
                    await patchApp(props.device);
                    modStatus.app_info!.is_modded = true;
                    setModStatus(modStatus);
                }   finally {
                    setIsPatching(false);
                }
            }}>Mod the app</button>
            <br />
        </>
    }
}


export default DeviceModder;