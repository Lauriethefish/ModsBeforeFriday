const ANDROID_NS_URI: string = "http://schemas.android.com/apk/res/android";

// Class that allows convenient modification of an APK manifest.
export class AndroidManifest {
    private features: string[] = [];
    private permissions: string[] = [];
    private document!: XMLDocument;
    private manifestEl!: Node;

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
        this.manifestEl = this.document.getElementsByTagName("manifest")[0];
        this.permissions = [];
        this.features = [];
        // Load the permissions and features already within the manifest.
        Array.from(this.document.getElementsByTagName("uses-permission")).forEach(permNode => {
            const permName = permNode.getAttribute("android:name");
            if(permName !== null) {
                this.permissions.push(permName);
            }
        })
        Array.from(this.document.getElementsByTagName("uses-feature")).forEach(permNode => {
            const featName = permNode.getAttribute("android:name");
            if(featName !== null) {
                this.features.push(featName);
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

    // Performs the default modifications to the manifest that every modded game should have, i.e:
    // - Enable MANAGE_EXTERNAL_STORAGE permission.
    // - Make app debuggable
    // - Enable hardware acceleration. (useful for any mods that make use of the WebView API)
    public applyPatchingManifestMod() {
        const application = this.document.getElementsByTagName("application")[0];
        application.setAttributeNS(ANDROID_NS_URI, "android:debuggable", "true");
        application.setAttributeNS(ANDROID_NS_URI, "android:hardwareAccelerated", "true");

        this.addPermission("android.permission.MANAGE_EXTERNAL_STORAGE");
    }

    // Adds a <uses-permission> element for the specified permission underneath the manifest tag.
    // If the permission is already specified this does nothing.
    public addPermission(perm: string) {
        if(this.permissions.includes(perm)) {
            return;
        }

        const permissionElement = this.document.createElement("uses-permission");
        permissionElement.setAttributeNS(ANDROID_NS_URI, "android:name", perm);
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
        featureElement.setAttributeNS(ANDROID_NS_URI, "android:name", feat);
        featureElement.setAttributeNS(ANDROID_NS_URI, "android:required", "false");
        this.manifestEl.appendChild(featureElement);
        this.features.push(feat);
    }

    // Removes the <uses-permission> element for the specified permission, if it exists.
    public removePermission(perm: string) {
        const permTag = Array.from(this.document.getElementsByTagName("uses-permission"))
            .find(permTag => permTag.getAttribute("android:name") === perm);
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
        .find(featTag => featTag.getAttribute("android:name") === feat);
        if(featTag !== undefined) {
            this.manifestEl.removeChild(featTag);
            const permIdx = this.features.indexOf(feat);
            if(permIdx != -1) {
                this.features.splice(permIdx, 1);
            }
        }
    }
}
