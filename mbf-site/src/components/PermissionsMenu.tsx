import { useState } from "react"
import { Slider } from "./Slider"

import '../css/PermissionsMenu.css'
import { ManifestMod } from "../Models"
import { EditableList } from "./EditableList";

function includeValuesIfEnabled(valueNames: string[], enabled: boolean, inArray: string[]): string[] {
    if(enabled) {
        return Array.from(new Set([...inArray, ...valueNames]));
    }   else    {
        return inArray.filter(element => !valueNames.includes(element));
    }
}

// A particular toggle in the "permissions" menu.
// This option being enabled will constitute having one or more features or permissions enabled.
interface ManifestOptionInfo {
    name: string, // Human readable
    features: string[]
    permissions: string[]
}

const displayedOptions: ManifestOptionInfo[] = [
    {
        name: "Microphone Access",
        permissions: ["android.permission.RECORD_AUDIO"],
        features: []
    },
    {
        name: "Passthrough to headset cameras",
        permissions: [],
        features: ["com.oculus.feature.PASSTHROUGH"]
    },
    {
        name: "Body tracking",
        permissions: ["com.oculus.permission.BODY_TRACKING"],
        features: ["com.oculus.software.body_tracking"]
    }
]

interface ManifestStateProps {
    manifestMod: ManifestMod,
    setManifestMod: (mod: ManifestMod) => void
}

export function PermissionsMenu({ manifestMod, setManifestMod }: ManifestStateProps) {
    const [advanced, setAdvanced] = useState(false);

    return <>
        <p>Certain mods may find it useful for the app to request microphone permissions or access to the headset cameras. Due to the privacy implications, you need to enable permissions here if you want them.</p>
        
        {!advanced && <ToggleMenu manifestMod={manifestMod} setManifestMod={setManifestMod} />}
        {advanced && <TextFieldMenu manifestMod={manifestMod} setManifestMod={setManifestMod} />}

        <br/>
        <button onClick={() => setAdvanced(!advanced)}>{advanced ? "Simple options" : "Advanced Options"}</button>
    </>
}

export function ToggleMenu({ manifestMod, setManifestMod }: ManifestStateProps) {
    return <>
        {displayedOptions
        .map(permInfo => {
            // The option is enabled if all permissions/features are in the current manifest mod.
            const enabled = permInfo.features.every(feature => manifestMod.add_features.includes(feature)) &&
                permInfo.permissions.every(feature => manifestMod.add_permissions.includes(feature));

            return <span id="namedSlider" key={permInfo.name}>
                <Slider on={enabled}
                    valueChanged={v => {
                        setManifestMod({
                            add_features: includeValuesIfEnabled(permInfo.features, v, manifestMod.add_features),
                            add_permissions: includeValuesIfEnabled(permInfo.permissions, v, manifestMod.add_permissions)
                        })
                    }} />

                <p>{permInfo.name}</p>
            </span>
        })}
    </>
}

export function TextFieldMenu({ manifestMod, setManifestMod }: ManifestStateProps) {
    return <>
        <EditableList title="Permissions" list={manifestMod.add_permissions} setList={newPermissions => setManifestMod({
            ...manifestMod,
            add_permissions: newPermissions
        })} />
        <EditableList title="Features" list={manifestMod.add_features} setList={newFeatures => setManifestMod({
            ...manifestMod,
            add_features: newFeatures
        })} />
    </>
}