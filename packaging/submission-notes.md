# Submission notes

What a store submission needs from a person and no file supplies. The listing
text is `store-listing.toml`, Apple's Notes for Review and Microsoft's Notes for
certification are its `apple-review-notes` and `microsoft-review-notes`, and what
a release changed is `release-notes.toml`; the release apps push all of them. A
field edited here reaches nobody.

## Capability justification

Partner Center asks once, when `runFullTrust` is first declared, and keeps the
answer against the product. 500 characters, newlines counted.

> Slipcase Desktop is a full-trust Win32 desktop application packaged as
> MSIX. It needs this capability to run at all. It opens the container it
> was launched with, or one chosen through Open, and writes only where a
> save dialog points. It makes no network connection, needs no broad
> filesystem access, and uses no device.

## The answers a form asks

    Category                  Utilities
    Copyright and trademark   Excelano LLC
    Additional license terms  blank; the application is MIT and the
                              repository carries the license
    Privacy policy            https://excelano.com/legal/#slipcase
    Age rating                no objectionable content of any kind
    App Privacy (Apple)       Data Not Collected
    Export compliance         no encryption; it makes no network request
                              and implements no cryptography

## Screenshots

Taken by `shots-winstore` and `shots-macstore` from the recipes in
`windows/shots.ps1` and `macos/shots.sh`, over the containers
`demo-container.sh` builds, in each listing language. The Microsoft set is a
container at rest and the same container marked as downloaded, light then dark.
The Mac set shows the application in use: a value being edited, the container
that arrived from elsewhere, the kind menu open, and the edit again in dark.
Light leads on both, because both platforms run apps light by default.
