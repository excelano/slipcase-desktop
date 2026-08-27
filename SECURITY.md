# Security Policy

## Reporting a vulnerability

Please report suspected vulnerabilities privately through GitHub Security
Advisories at https://github.com/excelano/slipcase-desktop/security/advisories/new.
If you would rather not use GitHub, email david.anderson@excelano.com instead. I
aim to respond within seven days.

Please do not open public issues for security problems.

## Supported versions

The latest 0.x release receives security fixes. Older versions are not
supported.

## What this application is

Slipcase opens a container someone may have sent you, shows its metadata, and
hands the payload to whatever your operating system has registered for it. It
runs locally, makes no network call of any kind, has no account and no telemetry,
and can read and write only what your operating-system user already can.

Containers are read and written entirely through `slpc`, the library in
`excelano/slpc-rust`, which has a security policy of its own covering the format
and the command-line tool. This document covers the window.

## What it stores

One line. The directory of the last container you opened is written to
`$XDG_STATE_HOME/slipcase-desktop/last-folder` — `%APPDATA%` or
`~/Library/Application Support` on the other two platforms — so the file dialog
opens somewhere useful. Nothing else is kept, nothing is sent anywhere, and
deleting that file removes it.

Payloads you press Open on are extracted to a private temporary directory, mode
0700, which is removed when the application exits.

## Two things worth stating plainly

**It does not decide whether a payload is safe to open.** The card says what the
platform said would open the payload, and the Open button hands the file to the
platform. This application ships no table mapping filenames to types, does not
inspect a payload's contents to guess at one, and does not substitute its own
judgement for the operating system's about what is dangerous. What it adds is
information: where the container came from, and whether the payload was stored
as an executable file.

**It carries provenance rather than stripping it.** A container marked by the
platform as having arrived from elsewhere — `com.apple.quarantine`, a
`Zone.Identifier` stream, `user.xdg.origin.url` — passes that mark to the payload
extracted from it and keeps it on the container when you edit and save. Without
that, an application like this one is a tool for laundering the mark, which is
the property that made container attachments a favoured delivery mechanism. Both
halves were defects here once and both are recorded in `CHECKLIST.md` and in the
`git log`.

## What a conformance verdict does not mean

`Slipcase` reports whether a container conforms to the format specification. That
is a statement about the file's structure and says nothing about whether its
payload is safe, whether the container is what it claims to be, or who produced
it. The format defines no signature, no checksum, and no encryption of its own.

## Verifying releases

Every release lists a SHA-256 for each artefact. Verify a download before
running it.

<!--
Author: David M. Anderson
Built with AI assistance (Claude, Anthropic)
-->
