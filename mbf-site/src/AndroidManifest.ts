import { Log } from "./Logging";

const ANDROID_NS_URI: string = "http://schemas.android.com/apk/res/android";

// Class that allows convenient modification of an APK manifest.
export class AndroidManifest {
    private features: string[] = [];
    private permissions: string[] = [];
    private nativeLibraries: string[] = [];
    private metadata: { [name: string]: string } = {};
    private document!: XMLDocument;
    private manifestEl!: Node;
    private applicationEl!: Node;
    private androidNsPrefix: string = "";

    // Initialises the manifest, loading it from the given XML.
    constructor(manifest_xml: string) {
        // This will set document and manifestEl
        this.loadFrom(manifest_xml);
        this.document;
    }

    // Reloads the manifest from a given XML string.
    public loadFrom(manifest_xml: string) {
        const parser = new DOMParser();
        // NB: This does NOT throw an exception upon invalid JSON
        const tentativeDoc = parser.parseFromString(manifest_xml, "text/xml");;
        const errorNode = tentativeDoc.querySelector("parsererror");
        if(errorNode !== null) {
            // Failed to parse XML
            throw new Error("Invalid XML " + errorNode);
        }

        this.document = tentativeDoc;
        const androidNsPrefix = this.document.lookupPrefix(ANDROID_NS_URI);
        if(androidNsPrefix === null) {
            throw new Error("Not a manifest, has no Android namespace URI prefix");
        }
        this.androidNsPrefix = androidNsPrefix;
        
        this.manifestEl = this.document.getElementsByTagName("manifest")[0];
        this.applicationEl = this.document.getElementsByTagName("application")[0];
        this.permissions = [];
        this.features = [];
        // Load the permissions and features already within the manifest.
        Array.from(this.document.getElementsByTagName("uses-permission")).forEach(permNode => {
            const permName = permNode.getAttribute(`${androidNsPrefix}:name`);
            if(permName !== null) {
                this.permissions.push(permName);
            }
        })
        Array.from(this.document.getElementsByTagName("uses-feature")).forEach(permNode => {
            const featName = permNode.getAttribute(`${androidNsPrefix}:name`);
            if(featName !== null) {
                this.features.push(featName);
            }
        })
        Array.from(this.document.getElementsByTagName("uses-native-library")).forEach(libNode => {
            const libName = libNode.getAttribute(`${androidNsPrefix}:name`);
            if(libName !== null) {
                this.nativeLibraries.push(libName);
            }
        })
        Array.from(this.applicationEl.childNodes).forEach(appChild => {
            if(appChild.nodeType == Node.ELEMENT_NODE && appChild.nodeName == "meta-data") {
                const metadataName = (appChild as Element).getAttribute(`${androidNsPrefix}:name`);
                const metadataValue = (appChild as Element).getAttribute(`${androidNsPrefix}:value`);

                if(metadataName !== null && metadataValue !== null) {
                    if(metadataName in this.metadata) {
                        Log.warn("Duplicate metadata key found: " + metadataName + " ..removing");
                        // Safe as we've used Array.from, so we're not modifying the array while iterating it.
                        this.applicationEl.removeChild(appChild);
                    }   else    {
                        this.metadata[metadataName] = metadataValue;
                    }
                }   else    {
                    Log.warn("Invalid metadata node: missing name or value: " + appChild);
                }
            }
        })
    }

    // Converts the manifest into a string to be sent to the backend.
    public toString(): string {
        return new XMLSerializer().serializeToString(this.document);
    }

    // Gets an array of the current permissions within the APK.
    public getPermissions(): string[] {
        return this.permissions;
    }

    // Gets an array of the current features within the APK.
    public getFeatures(): string[] {
        return this.features;
    }

    // Gets a map of metadata element names to metadata values within the application element of the manifest.
    public getMetadata(): { [name: string]: string } {
        return this.metadata;
    }

    // Gets an array of the library file names that have
    // a `uses-native-library` element within the <application> element.
    public getNativeLibraries(): string[] {
        return this.nativeLibraries;
    }

    // Performs the default modifications to the manifest that every modded game should have, i.e:
    // - Enable MANAGE_EXTERNAL_STORAGE permission.
    // - Make app debuggable
    // - Enable hardware acceleration. (useful for any mods that make use of the WebView API)
    public applyPatchingManifestMod() {
        const application = this.document.getElementsByTagName("application")[0];
        application.setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:debuggable`, "true");
        application.setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:hardwareAccelerated`, "true");

        this.setMetadata("com.oculus.supportedDevices", "quest|quest2");

        this.addPermission("android.permission.MANAGE_EXTERNAL_STORAGE");
    }

    // Adds a <uses-permission> element for the specified permission underneath the manifest tag.
    // If the permission is already specified this does nothing.
    public addPermission(perm: string) {
        if(this.permissions.includes(perm)) {
            return;
        }

        const permissionElement = this.document.createElement("uses-permission");
        permissionElement.setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:name`, perm);
        this.manifestEl.appendChild(permissionElement);
        this.permissions.push(perm);
    }

    // Adds a <uses-feature> element for the specified feature underneath the manifest tag.
    // If the feature is already specified this does nothing.
    // The feature will have the `android:required` attribute set to `false`.
    public addFeature(feat: string) {
        if(this.features.includes(feat)) {
            return;
        }

        const featureElement = this.document.createElement("uses-feature");
        featureElement.setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:name`, feat);
        featureElement.setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:required`, "false");
        this.manifestEl.appendChild(featureElement);
        this.features.push(feat);
    }

    // Adds a <uses-native-library> element for the specified library underneath the application tag.
    // If the native library is already specified, this does nothing.
    public addNativeLibrary(fileName: string) {
        if(this.nativeLibraries.includes(fileName)) {
            return;
        }

        const nativeLibElement = this.document.createElement("uses-native-library");
        nativeLibElement.setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:name`, fileName);
        nativeLibElement.setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:required`, "false");
        this.applicationEl.appendChild(nativeLibElement);
        this.nativeLibraries.push(fileName);
    }

    // Adds or updates a <meta-data> tag to set the metadata with the given name to the provided value.
    public setMetadata(name: string, value: string) {
        const matchingMetadata = Array.from(this.document.getElementsByTagName("meta-data"))
            .filter(element => element.getAttribute(`${this.androidNsPrefix}:name`) == name);

        if(matchingMetadata.length == 0) {
            // No existing element, so cannot update existing: create a new meta-data element
            const newMetaElement = this.document.createElement("meta-data");
            newMetaElement.setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:name`, name);
            newMetaElement.setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:value`, value);
            this.applicationEl.appendChild(newMetaElement);
        }   else    {
            matchingMetadata[0].setAttributeNS(ANDROID_NS_URI, `${this.androidNsPrefix}:value`, value);

            // When loading the manifest, any duplicate metadata keys get automatically removed.
        }

        this.metadata[name] = value;
    }

    // Removes any <meta-data> tag with the given name from the document, if one exists.
    public removeMetadata(name: string) {
        Array.from(this.document.getElementsByTagName("meta-data"))
            .filter(element => element.getAttribute(`${this.androidNsPrefix}:name`) == name)
            .forEach(element => element.parentElement?.removeChild(element))

        delete this.metadata[name];
    }

    // Removes the <uses-permission> element for the specified permission, if it exists.
    public removePermission(perm: string) {
        const permTag = Array.from(this.document.getElementsByTagName("uses-permission"))
            .find(permTag => permTag.getAttribute(`${this.androidNsPrefix}:name`) === perm);
        if(permTag !== undefined) {
            this.manifestEl.removeChild(permTag);
            const permIdx = this.permissions.indexOf(perm);
            if(permIdx != -1) {
                this.permissions.splice(permIdx, 1);
            }
        }
    }

    // Removes the <uses-feature> element for the specified feature, if it exists.
    public removeFeature(feat: string) {
        const featTag = Array.from(this.document.getElementsByTagName("uses-feature"))
        .find(featTag => featTag.getAttribute(`${this.androidNsPrefix}:name`) === feat);
        if(featTag !== undefined) {
            this.manifestEl.removeChild(featTag);
            const permIdx = this.features.indexOf(feat);
            if(permIdx != -1) {
                this.features.splice(permIdx, 1);
            }
        }
    }

    // Removes the <uses-native-library> element with the specified name
    public removeNativeLibrary(fileName: string) {
        const libTag = Array.from(this.document.getElementsByTagName("uses-native-library"))
            .find(tag => tag.getAttribute(`${this.androidNsPrefix}:name`) === fileName);
        if(libTag !== undefined) {
            this.applicationEl.removeChild(libTag);

            const libIdx = this.nativeLibraries.indexOf(fileName);
            if(libIdx != -1) {
                this.nativeLibraries.splice(libIdx, 1);
            }
        }
    }
}
