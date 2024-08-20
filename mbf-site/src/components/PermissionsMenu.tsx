import { useRef, useState } from "react"
import { Slider } from "./Slider"

import '../css/PermissionsMenu.css'
import { EditableList } from "./EditableList";
import { AndroidManifest } from "../AndroidManifest";
import { Modal } from "./Modal";
import { toast } from "react-toastify";


// A particular toggle in the "permissions" menu.
// This option being enabled will constitute having one or more features or permissions enabled.
interface ManifestOptionInfo {
    name: string, // Human readable
    features: string[]
    permissions: string[],
    // A dictionary of metadata element name attributes to value attributes
    // These are added within the `application` element when the option is toggled on.
    app_metadata?: { [name: string]: string },
}

const displayedOptions: ManifestOptionInfo[] = [
    {
        name: "Microphone Access",
        permissions: ["android.permission.RECORD_AUDIO"],
        features: [],
    },
    {
        name: "Passthrough to headset cameras",
        permissions: [],
        features: ["com.oculus.feature.PASSTHROUGH"],
    },
    {
        name: "Body tracking",
        permissions: ["com.oculus.permission.BODY_TRACKING"],
        features: ["com.oculus.software.body_tracking"]
    },
    {
        name: "Hand tracking",
        permissions: ["com.oculus.permission.HAND_TRACKING"],
        features: ["oculus.software.handtracking"],
        app_metadata: {
            "com.oculus.handtracking.frequency": "MAX",
            "com.oculus.handtracking.version": "V2.0"
        }
    },
    {
        name: "Bluetooth",
        permissions: ["android.permission.BLUETOOTH", "android.permission.BLUETOOTH_CONNECT"],
        features: []
    }
]

// The current state of the manifest permissions/features.
interface ManifestState {
    permissions: string[],
    features: string[],
    metadata: { [name: string]: string }
}

interface ManifestStateProps {
    // The current manifest state
    state: ManifestState,
    manifest: AndroidManifest,
    // Regenerates the state based on the AndroidManifest
    updateState: () => void
}

export function PermissionsMenu({ manifest }: { manifest: AndroidManifest }) {
    const [advanced, setAdvanced] = useState(false);
    const [editXml, setEditXml] = useState(false);
    const [manifestState, setManifestState] = useState({
        permissions: manifest.getPermissions(),
        features: manifest.getFeatures(),
        metadata: manifest.getMetadata(),
    });

    function updateState() {
        setManifestState({
            permissions: manifest.getPermissions(),
            features: manifest.getFeatures(),
            metadata: manifest.getMetadata(),
        })
    }

    return <>
        <button className="discreetButton" onClick={() => setAdvanced(!advanced)}>{advanced ? "Simple options" : "Advanced Options"}</button>
        {advanced && <button className="discreetButton rightMargin" onClick={() => setEditXml(true)}>Edit XML</button>}
        {!advanced && <ToggleMenu manifest={manifest} state={manifestState} updateState={updateState} />}
        {advanced && <TextFieldMenu manifest={manifest} state={manifestState} updateState={updateState} />}
        <Modal isVisible={editXml}>
            <XmlEditor manifestXml={manifest.toString()} setManifestXml={manifestString => {
                try {
                    manifest.loadFrom(manifestString);
                    updateState();
                    setEditXml(false);
                    toast.success("Successfully updated manifest XML");
                    return true;
                }   catch(e) {
                    toast.error("Provided file was not a valid XML manifest");
                    return false;
                }
            }}/>
            <button onClick={() => setEditXml(false)}>Back</button>
        </Modal>
    </>
}

function ToggleMenu({ state, manifest, updateState }: ManifestStateProps) {
    return <>
        {displayedOptions
        .map(permInfo => {
            // The option is enabled if all permissions/features are in the current manifest mod.
            const enabled = permInfo.features.every(feature => state.features.includes(feature)) &&
                permInfo.permissions.every(feature => state.permissions.includes(feature)) &&
                (permInfo.app_metadata === undefined || Object.entries(permInfo.app_metadata)
                    .every(entry => state.metadata[entry[0]] == entry[1]));

            return <span id="namedSlider" key={permInfo.name}>
                <Slider on={enabled}
                    valueChanged={nowEnabled => {
                        if(nowEnabled) {
                            permInfo.permissions.forEach(perm => manifest.addPermission(perm));
                            permInfo.features.forEach(feat => manifest.addFeature(feat));
                            if(permInfo.app_metadata) {
                                Object.entries(permInfo.app_metadata).forEach(pair => manifest.setMetadata(pair[0], pair[1]))
                            }
                            
                        }   else    {
                            permInfo.permissions.forEach(feat => manifest.removePermission(feat));
                            permInfo.features.forEach(perm => manifest.removeFeature(perm));
                            if(permInfo.app_metadata) {
                                Object.keys(permInfo.app_metadata).forEach(name => manifest.removeMetadata(name))
                            }
                        }

                        updateState();
                    }} />

                <p>{permInfo.name}</p>
            </span>
        })}
    </>
}


function TextFieldMenu({ state, manifest, updateState }: ManifestStateProps) {
    return <>
        <EditableList title="Permissions" list={state.permissions} addItem={item => {
            manifest.addPermission(item);
            updateState();
        }} removeItem={item => {
            manifest.removePermission(item);
            updateState();
        }} />
        <br/>
        <EditableList title="Features" list={state.features} addItem={item => {
            manifest.addFeature(item);
            updateState();
        }} removeItem={item => {
            manifest.removeFeature(item);
            updateState();
        }} />
    </>
}

function XmlEditor({ manifestXml, setManifestXml }: {
    manifestXml: string,
    setManifestXml: (xml: string) => boolean // True iff the manifest was valid XML
}) {
    const xmlBlob = new Blob([manifestXml], { type: "text/xml" });
    const inputFile = useRef<HTMLInputElement | null>(null);

    return <>
        <h2>Change manifest XML</h2>
        <p>For development purposes, this menu will allow you to manually edit the entirety of the AndroidManifest.xml file within the APK</p>
        <p>Be careful, as erroneous edits will prevent the APK from installing properly.</p>
        <a href={URL.createObjectURL(xmlBlob)}
            download="AndroidManifest.xml">
            <button className="rightMargin">Download Current XML</button>
        </a>

        <button className="rightMargin" onClick={() => inputFile.current?.click()}>
            <input type="file"
                id="file"
                multiple={false}
                ref={inputFile}
                style={{display: 'none'}}
                onChange={async ev => {
                    const files = ev.target.files;
                    if(files !== null) {
                        const fileUploaded = files[0];
                        setManifestXml(await fileUploaded.text());
                    }
                    ev.target.value = "";
                }}
            />Upload XML
        </button>
    </>
    
}