import { useRef, useState } from "react"
import { Slider } from "./Slider"

import '../css/PermissionsMenu.css'
import { EditableList } from "./EditableList";
import { AndroidManifest } from "../AndroidManifest";
import { Modal } from "./Modal";
import { toast } from "react-toastify";
import { getLang } from "../localization/shared";


// A particular toggle in the "permissions" menu.
// This option being enabled will constitute having one or more features or permissions enabled.
interface ManifestOptionInfo {
    nameKey: string,
    nameElement: ()=>JSX.Element, // Human readable
    features: string[]
    permissions: string[],
    // A dictionary of metadata element name attributes to value attributes
    // These are added within the `application` element when the option is toggled on.
    app_metadata?: { [name: string]: string },
    native_libraries?: string[]
}

const displayedOptions: ManifestOptionInfo[] = [
    {
        nameKey: "Microphone Access",
        nameElement:()=>getLang().permMicrophone,
        permissions: ["android.permission.RECORD_AUDIO"],
        features: [],
    },
    {
        nameKey: "Passthrough to headset cameras",
        nameElement:()=>getLang().permPassthrough,
        permissions: [],
        features: ["com.oculus.feature.PASSTHROUGH"],
    },
    {
        nameKey: "Body tracking",
        nameElement:()=>getLang().permBody,
        permissions: ["com.oculus.permission.BODY_TRACKING"],
        features: ["com.oculus.software.body_tracking"]
    },
    {
        nameKey: "Hand tracking",
        nameElement:()=>getLang().permHand,
        permissions: ["com.oculus.permission.HAND_TRACKING"],
        features: ["oculus.software.handtracking"],
        app_metadata: {
            "com.oculus.handtracking.frequency": "MAX",
            "com.oculus.handtracking.version": "V2.0"
        }
    },
    {
        nameKey: "Bluetooth",
        nameElement:()=>getLang().permBluetooth,
        permissions: ["android.permission.BLUETOOTH", "android.permission.BLUETOOTH_CONNECT"],
        features: []
    },
    {
        nameKey: "MRC workaround",
        nameElement:()=>getLang().permMRC,
        permissions: [],
        features: [],
        native_libraries: ["libOVRMrcLib.oculus.so"]
    }
]

// The current state of the manifest permissions/features.
interface ManifestState {
    permissions: string[],
    features: string[],
    metadata: { [name: string]: string },
    nativeLibraries: string[]
}

interface ManifestStateProps {
    // The current manifest state
    state: ManifestState,
    manifest: AndroidManifest,
    // Regenerates the state based on the AndroidManifest
    updateState: () => void
}

function getStateFromManifest(manifest: AndroidManifest): ManifestState {
    return {
        permissions: manifest.getPermissions(),
        features: manifest.getFeatures(),
        metadata: manifest.getMetadata(),
        nativeLibraries: manifest.getNativeLibraries()
    };
}

export function PermissionsMenu({ manifest }: { manifest: AndroidManifest }) {
    const [advanced, setAdvanced] = useState(false);
    const [editXml, setEditXml] = useState(false);
    const [manifestState, setManifestState] = useState(getStateFromManifest(manifest));

    function updateState() {
        setManifestState(getStateFromManifest(manifest));
    }

    return <>
        <button className="discreetButton" onClick={() => setAdvanced(!advanced)}>{advanced ? getLang().SimpleOptions : getLang().AdvancedOptions }</button>
        {advanced && <button className="discreetButton rightMargin" onClick={() => setEditXml(true)}>{getLang().EditXML}</button>}
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
            <button onClick={() => setEditXml(false)}>{getLang().backBtnText}</button>
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
                    .every(entry => state.metadata[entry[0]] == entry[1])) &&
                (permInfo.native_libraries === undefined || permInfo.native_libraries.every(lib => state.nativeLibraries.includes(lib)))

            return <span id="namedSlider" key={permInfo.nameKey}>
                <Slider on={enabled}
                    valueChanged={nowEnabled => {
                        if(nowEnabled) {
                            permInfo.permissions.forEach(perm => manifest.addPermission(perm));
                            permInfo.features.forEach(feat => manifest.addFeature(feat));
                            if(permInfo.app_metadata) {
                                Object.entries(permInfo.app_metadata).forEach(pair => manifest.setMetadata(pair[0], pair[1]))
                            }
                            permInfo.native_libraries?.forEach(lib => manifest.addNativeLibrary(lib));
    
                        }   else    {
                            permInfo.permissions.forEach(feat => manifest.removePermission(feat));
                            permInfo.features.forEach(perm => manifest.removeFeature(perm));
                            permInfo.native_libraries?.forEach(lib => manifest.removeNativeLibrary(lib));
                            if(permInfo.app_metadata) {
                                Object.keys(permInfo.app_metadata).forEach(name => manifest.removeMetadata(name))
                            }
                        }

                        updateState();
                    }} />

                <p>{permInfo.nameElement()}</p>
            </span>
        })}
    </>
}


function TextFieldMenu({ state, manifest, updateState }: ManifestStateProps) {
    return <>
        <EditableList title={getLang().permMenuPermissions} list={state.permissions} addItem={item => {
            manifest.addPermission(item);
            updateState();
        }} removeItem={item => {
            manifest.removePermission(item);
            updateState();
        }} />
        <br/>
        <EditableList title={getLang().permMenuFeatures} list={state.features} addItem={item => {
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
        {getLang().changeManifestXmlHint}
        <a href={URL.createObjectURL(xmlBlob)}
            download="AndroidManifest.xml">
            <button className="rightMargin">{getLang().downloadCurrentXML}</button>
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
            />{getLang().uploadXML}
        </button>
    </>
    
}