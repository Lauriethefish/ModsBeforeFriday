# MBF QMOD Android manifest requirements

MBF supports a deliberately limited, optional QMOD extension that lets a mod declare Android
packages that must be visible through `PackageManager` on Android 11 and newer. It is namespaced
to MBF and does not claim a new version of the shared QMOD format.

```json
{
  "_QPVersion": "0.1.2",
  "name": "Discord integration example",
  "id": "discord-integration-example",
  "author": "Example",
  "version": "1.0.0",
  "modFiles": ["libdiscord-integration-example.so"],
  "mbfManifestRequirements": {
    "queryPackages": ["com.discord"]
  }
}
```

If it is not already present, MBF adds:

```xml
<queries>
    <package android:name="com.discord" />
</queries>
```

Android documents this declaration as making the named app visible to matching
`PackageManager` queries. It does not grant an Android permission. See Android's
[package visibility documentation](https://developer.android.com/training/package-visibility/declaring)
and [`<queries>` reference](https://developer.android.com/guide/topics/manifest/queries-element).

## Compatibility and packaging

`mbfManifestRequirements` is optional. QMODs that omit it behave exactly as before, and the
extension works with every QMOD schema version MBF already supports. The shared `_QPVersion`
therefore remains unchanged.

The standard QMOD schema permits additional root properties, so compatible installers that do not
implement this MBF extension can ignore it. A mod that relies on the declaration should still
detect an unavailable target package or service at runtime and give the user an actionable message;
that covers installation through an older MBF release or a different installer.

QPM.CLI currently regenerates `mod.json` from its typed model and does not preserve unknown fields
from `mod.template.json`. A producing repository must therefore add this extension after
`qpm qmod zip`, then verify that the final archive contains the exact declaration and that every
non-manifest payload is byte-for-byte unchanged.

This avoids requiring coordinated changes to the QMOD specification and packaging libraries for a
feature needed by one mod. It also makes ownership explicit: this field is an MBF contract, not a
portable guarantee provided by every QMOD installer.

### Future native QPM producer support

The post-build step is a workaround for the current QPM producer toolchain, not a permanent format
requirement. QPM.CLI reads `mod.template.json` and writes the generated `mod.json` through the
`ModJson` model supplied by the separate QPM.qmod library. That model currently does not retain this
MBF-specific root property, including when QPM.CLI packages an imported manifest.

First-class producer support therefore requires both tooling layers to change:

1. QPM.qmod must preserve namespaced root extensions such as `mbfManifestRequirements`, either as
   an explicit optional field or through a generic flattened extension map.
2. QPM.CLI must update its QPM.qmod dependency and lockfile, then add round-trip coverage for
   `qmod manifest`, `qmod zip`, and `qmod zip --import`.
3. A QPM.CLI release containing that support must be available before producer repositories remove
   their verified post-build step.

This is a mod-author packaging limitation. It does not affect users installing a completed QMOD,
does not block MBF support, and does not require a new shared `_QPVersion`.

## Validation and automatic application

MBF accepts between 1 and 32 unique package IDs per QMOD. Each ID is at most 255 ASCII characters,
has at least two dot-separated components, starts every component with a letter, and otherwise
contains only letters, digits, or underscores. The Rust agent repeats these checks after JSON
Schema validation.

When a user-installed mod is enabled, the agent reads the installed binary Android manifest and verifies every
request. If declarations are missing, it returns the requesting mod IDs and exact packages without
copying those mods' files. The frontend automatically applies the manifest-only update and then
continues enabling the mod. The agent verifies the installed manifest before enabling it.
Required dependencies use the same checks, including dependencies downloaded during the operation.

Manifest-only repatches use Android's replace-existing install path and check the package manager's
result. A rejected generated APK therefore leaves the existing app installed.

Core mods from the resource index are outside this initial extension because their index metadata
does not carry QMOD manifest requirements before the initial patch. Supporting that separately
would require an explicit resource-index contract.

## Security boundary and non-goals

QMODs cannot provide raw XML or choose an insertion point. The schema rejects unknown members
inside `mbfManifestRequirements`, including permission lists. This extension intentionally does not
support:

- `uses-permission`, including `QUERY_ALL_PACKAGES`, runtime/dangerous permissions, or special
  permissions;
- components such as activities, services, receivers, or providers;
- intent/provider forms of package visibility;
- features, native libraries, metadata, attributes, or arbitrary XML.

These are separate security and compatibility decisions. Android notes that dangerous permissions
provide access to restricted data or actions, so accepting them would require a permission-specific
allowlist, prominent consent, and substantially more policy than package visibility. See Android's
[permissions overview](https://developer.android.com/guide/topics/permissions/overview).

This first version is additive. Disabling or removing a mod does not remove package visibility that
MBF previously added. Retaining a narrow package declaration is safer than deleting a declaration
that may have existed in the original game manifest or may still be shared by another mod. A future
removal feature would need persistent provenance and reference counting.
