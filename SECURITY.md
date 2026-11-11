# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately, using the **Report a vulnerability** button on the Security tab of this repository. If that button is not there, or you would rather not go through GitHub, email david.anderson@excelano.com instead. I aim to respond within seven days.

Please do not open public issues for security problems.

## Supported versions

The latest release receives security fixes. Older versions are not supported.
A fix ships in a new release; update the way you installed.

## What Slipcase can access

Slipcase opens a container someone may have sent you, shows its flyleaf, and
hands the content file to whatever your operating system has registered for it.
It runs locally, makes no network call of any kind, has no account and no
telemetry, and can read and write only what your operating-system user already
can. It writes where you point it: a container you save or make, and a content
file you extract.

Containers are read and written entirely through `slpc`, the library in
`excelano/slpc-rust`, which has a security policy of its own covering the format
and the command-line tool. This document covers the window.

**It does not decide whether a content file is safe to open.** The card says
what the platform said would open the content file, and the Open button hands
the file to the platform. This application ships no table mapping filenames to
types, does not inspect a content file's contents to guess at one, and does not
substitute its own judgment for the operating system's about what is dangerous.
What it adds is information: where the container came from, and whether the
content file was stored as an executable file.

**It carries provenance rather than stripping it.** A container marked by the
platform as having arrived from elsewhere (`com.apple.quarantine`, a
`Zone.Identifier` stream, `user.xdg.origin.url`) passes that mark to the content
file extracted from it and keeps it on the container when you edit and save.
Without that, an application like this one is a tool for laundering the mark,
which is the property that made container attachments a favored delivery
mechanism.

## What Slipcase stores

The directory of the last container you opened is written to
`$XDG_STATE_HOME/slipcase-desktop/last-folder`, or to
`~/.local/state/slipcase-desktop/last-folder` where that variable is unset, so
the file dialog opens somewhere useful. Under the macOS sandbox `~` is the
application's own container. On Windows the file is written only where `HOME`
is set. Nothing else is kept, nothing is sent anywhere, and deleting that file
removes it.

Content files you press Open on are extracted to a private temporary directory,
mode 0700 on Linux and macOS, which is removed when the application exits. On
Linux, asking what would open a content file writes an empty file carrying its
name into another private temporary directory, removed as soon as the answer
is in.

## What a conformance verdict does not mean

Slipcase reports whether a container conforms to the format specification. That
is a statement about the file's structure and says nothing about whether its
content file is safe, whether the container is what it claims to be, or who
produced it. The format defines no signature, no checksum, and no encryption of
its own.

## Verifying releases

Every GitHub release carries the `.deb` packages, the MSIX and the macOS binary,
built by `build-release`, the release program in excelano/shipping, from the
tagged commit using the packaging under `packaging/` in this repository, which
is public and auditable. The release attaches no checksum file, so a download
from it is vouched for only by where it came from. Packages served from the
Excelano apt repository are signed; https://excelano.com/apt/ has the key and
the one-line setup.
