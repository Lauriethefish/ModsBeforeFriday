# QMOD Android manifest requirements

MBF supports a deliberately limited manifest-requirements extension for QMOD schema version
1.3.0. It lets a mod declare specific Android packages that must be visible through
`PackageManager` on Android 11 and newer.

```json
{
  "_QPVersion": "1.3.0",
  "name": "Discord integration example",
  "id": "discord-integration-example",
  "author": "Example",
  "version": "1.0.0",
  "modFiles": ["libdiscord-integration-example.so"],
  "manifestRequirements": {
    "queryPackages": ["com.discord"]
  }
}
```

This produces the following declaration if it is not already present:

```xml
<queries>
    <package android:name="com.discord" />
</queries>
```

Android documents this declaration as making the named app visible to matching
`PackageManager` queries. It does not grant an Android permission. See Android's
[package visibility documentation](https://developer.android.com/training/package-visibility/declaring)
and [`<queries>` reference](https://developer.android.com/guide/topics/manifest/queries-element).

## Validation and automatic application

`manifestRequirements` is optional. Existing QMODs through schema version 1.2.0 remain valid and
behave exactly as before. A QMOD that uses `manifestRequirements` must specify `_QPVersion` 1.3.0,
so older installers fail clearly instead of silently ignoring a requirement and enabling a broken
mod.

MBF accepts at most 32 unique package IDs per QMOD. Each ID is at most 255 ASCII characters, has at
least two dot-separated components, starts every component with a letter, and otherwise contains
only letters, digits, or underscores. The Rust agent repeats these checks after JSON Schema
validation.

When a mod is enabled, the agent reads the installed binary Android manifest and verifies every
request. If declarations are missing, it returns the requesting mod IDs and exact packages without
copying those mods' files. The frontend automatically performs a manifest-only repatch and retries
the original enable operation. This preserves MBF's one-click install flow while keeping enforcement
in the typed schema and Rust agent. The agent verifies the installed manifest again on the retry.
Required dependencies use the same checks, including dependencies downloaded during the operation.

Manifest-only repatches use Android's replace-existing install path and check the package manager's
result. A rejected generated APK therefore leaves the existing app installed.

## Security boundary and non-goals

QMODs cannot provide raw XML or choose an insertion point. The schema rejects unknown members
inside `manifestRequirements`, including permission lists. This version intentionally does not
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

## Compatibility note

Version 1.3.0 is a proposed QMOD schema addition. Before mods publish QMODs that depend on it, the
canonical QMOD model/schema and other active installers should adopt the same typed field. Keeping
the version gate is important: merely adding an optional property to version 1.2.0 would let older
installers accept the archive, ignore the requirement, and report a misleading successful install.
