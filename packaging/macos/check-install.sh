#!/bin/sh
# Ask an *installed* Slipcase what it actually is, on the machine it is on.
#
# `build-app.sh --store` checks what it built, on the machine that built it.
# That is a different question from this one, and the gap between them is where
# a submission goes wrong: the artefact is carried to another machine, macOS
# decides something about it there, and nothing in the build says what.
#
# This exists because the Apple silicon walkthrough is an hour on a rented
# machine and the mechanical half of it should not be typed out from a list.
# Everything below is a fact a command can settle. What a command cannot settle
# — whether the window is laid out correctly, whether the icon is right, what
# Gatekeeper shows a *person* — is in `CHECKLIST.md` and needs eyes.
#
#     ./check-install.sh                    # /Applications/Slipcase.app
#     ./check-install.sh /path/to/App       # somewhere else
#
# It reports; it does not repair, and it does not stop at the first bad answer —
# an hour on a rented machine is the wrong place to learn one thing per run.
#
# Author: David M. Anderson
# Built with AI assistance (Claude, Anthropic)
set -u

app="${1:-/Applications/Slipcase.app}"
bundle_id="com.excelano.slipcase-desktop"
team="9K6W5PMFYP"

findings=0
say()  { printf '  %-46s %s\n' "$1" "$2"; }
ok()   { say "$1" "ok — $2"; }
bad()  { say "$1" "NO — $2"; findings=$((findings + 1)); }
note() { printf '  %-46s %s\n' "$1" "$2"; }

echo "check-install.sh on $(uname -m), macOS $(sw_vers -productVersion)"
echo "$app"
echo

[ -d "$app" ] || { echo "  not installed at $app"; exit 2; }

exe="${app}/Contents/MacOS/slipcase-desktop"

# 1. The architecture question the whole trip is about. A universal binary that
#    lost its arm64 slice to a bad `lipo` would install and run here under
#    Rosetta and look entirely normal while doing it.
arches=$(lipo -info "$exe" 2>/dev/null | sed 's/.*are: //')
case "$arches" in
    *arm64*) ok "the binary has an arm64 slice" "$arches" ;;
    *) bad "the binary has an arm64 slice" "${arches:-lipo could not read it}" ;;
esac

# 2. And whether the machine is actually running that slice, which is not the
#    same claim. Asked of the running process rather than the file: `vmmap`
#    prints the code type of a process you own, and Rosetta is invisible in
#    every other listing — `ps` reports the same line either way.
# Matched on the tail of the path rather than the whole of it: the process may
# have been started with a relative path, and an absolute pattern then finds
# nothing and reports "not running" about an application in front of you.
pid=$(pgrep -f "Slipcase.app/Contents/MacOS/slipcase-desktop" | head -1)
#    The comparison is against *this machine*, not against arm64. Written the
#    other way first, this reported "under Rosetta" for a process running
#    natively on the Intel Mac it was developed on — a check that is wrong
#    everywhere except the one machine it was aimed at.
if [ -n "$pid" ]; then
    code_type=$(vmmap "$pid" 2>/dev/null | sed -n 's/^Code Type: *//p' | head -1)
    case "$(uname -m),$code_type" in
        arm64,ARM64*|x86_64,X86*) ok "the running process is native" "$code_type on $(uname -m)" ;;
        arm64,X86*) bad "the running process is native" "$code_type on arm64 — under Rosetta" ;;
        *,"") note "the running process is native" "vmmap said nothing usable" ;;
        *) bad "the running process is native" "$code_type on $(uname -m)" ;;
    esac
else
    note "the running process is native" "not running — launch it and re-run"
fi

# 3. Which of the three builds this is, decided from the certificate rather
#    than from what the caller believed. Three kinds ship out of this
#    repository and they want *different* answers to the checks below — a Store
#    build must carry an application identifier and a profile, and a Developer
#    ID build must not. A script with one set of expectations reports four
#    findings against a perfectly good bundle, which is how a checklist teaches
#    people to ignore it.
auth=$(codesign -dv --verbose=2 "$app" 2>&1)
case "$auth" in
    *"Apple Distribution: Excelano LLC (${team})"*)
        kind=store
        ok "signed" "Apple Distribution, team ${team} — a Store build" ;;
    *"Developer ID Application: Excelano LLC (${team})"*)
        kind=devid
        ok "signed" "Developer ID, team ${team} — the outside-the-Store hedge" ;;
    *"Apple Development"*)
        kind=dev
        ok "signed" "Apple Development — a local test build, not shippable" ;;
    *)
        kind=unknown
        bad "signed" "$(printf '%s' "$auth" | sed -n 's/^Authority=//p' | head -1)" ;;
esac
case "$auth" in
    *"Apple Root CA"*) ok "the chain reaches the Apple Root CA" "three authorities" ;;
    *) bad "the chain reaches the Apple Root CA" "it does not" ;;
esac

if codesign --verify --deep --strict "$app" 2>/dev/null; then
    ok "the signature verifies" "--deep --strict"
else
    bad "the signature verifies" "$(codesign --verify --deep --strict "$app" 2>&1 | head -1)"
fi

# 4. The entitlements, read back out of the signature rather than off the file
#    that was fed to it — those are different things and only one of them ships.
ents=$(codesign -d --entitlements - --xml "$app" 2>/dev/null | plutil -p - 2>/dev/null)
case "$ents" in
    *'"com.apple.security.app-sandbox" => 1'*)
        ok "the sandbox is in the signature" "app-sandbox" ;;
    *) bad "the sandbox is in the signature" "absent — the build is not sandboxed" ;;
esac
# Restricted, and so the whole reason a Store build needs a profile and cannot
# run without one. A Developer ID or development build must *not* carry it: it
# would be refused at launch for exactly the reason the Store build is.
case "$kind,$ents" in
    store,*"${team}.${bundle_id}"*)
        ok "the application identifier is there" "${team}.${bundle_id}" ;;
    store,*)
        bad "the application identifier is there" "absent — the upload is refused" ;;
    *,*"${team}.${bundle_id}"*)
        bad "no application identifier" "present on a ${kind} build — it will not launch" ;;
    *)  ok "no application identifier" "correct for a ${kind} build" ;;
esac
# Declined deliberately: the profile grants it and this application touches no
# keychain, and a capability asked for and unused is a question at review.
case "$ents" in
    *keychain-access-groups*) bad "keychain-access-groups is declined" "it is present" ;;
    *) ok "keychain-access-groups is declined" "absent, as intended" ;;
esac

# 5. The profile has to be inside the bundle, and inside it *before* it was
#    signed. Added afterwards, macOS calls the bundle damaged — which check 3
#    would already have caught, so this one is about presence.
profile="${app}/Contents/embedded.provisionprofile"
if [ "$kind" != store ] && [ ! -f "$profile" ]; then
    ok "no provisioning profile" "correct for a ${kind} build"
elif [ -f "$profile" ]; then
    plist=$(mktemp)
    if security cms -D -i "$profile" -o "$plist" 2>/dev/null; then
        pname=$(plutil -extract Name raw -o - "$plist" 2>/dev/null)
        pexp=$(plutil -extract ExpirationDate raw -o - "$plist" 2>/dev/null)
        ok "a provisioning profile is embedded" "${pname:-unnamed}, expires ${pexp:-unknown}"
    else
        bad "a provisioning profile is embedded" "present but would not decode"
    fi
    rm -f "$plist"
else
    bad "a provisioning profile is embedded" "no embedded.provisionprofile"
fi

# 6. What the bundle claims about itself, which App Store Connect deduplicates
#    uploads by and a person reads in the About box.
short=$(/usr/libexec/PlistBuddy -c "Print :CFBundleShortVersionString" \
    "${app}/Contents/Info.plist" 2>/dev/null)
build=$(/usr/libexec/PlistBuddy -c "Print :CFBundleVersion" \
    "${app}/Contents/Info.plist" 2>/dev/null)
note "the version it declares" "${short:-?} (build ${build:-?})"

# 7. Gatekeeper's verdict, which is not the signature's — and which for a Store
#    build is *rejection*, correctly.
#
#    `spctl -a` assesses the Developer ID and notarization policy. A Mac App
#    Store build is not distributed under that policy, so it is rejected naming
#    our own certificate as the origin, and counting that as a finding would
#    make this script cry wolf on every correct Store build. Measured
#    2026-08-29; `CHECKLIST.md`'s *What a Store-signed build did when it was
#    launched* holds it. What is worth reporting is a verdict that does not
#    match the certificate the bundle carries.
gk=$(spctl -a -vvv "$app" 2>&1)
case "$kind,$gk" in
    *,*accepted*)
        note "Gatekeeper" "accepted — $(printf '%s' "$gk" | sed -n 's/.*source=//p' | head -1)" ;;
    store,*rejected*)
        note "Gatekeeper" "rejected, as a Store build correctly is" ;;
    devid,*"Unnotarized Developer ID"*)
        note "Gatekeeper" "rejected — unnotarized, which is the hedge's own step" ;;
    dev,*rejected*)
        note "Gatekeeper" "rejected, as a development build correctly is" ;;
    *)  bad "Gatekeeper" "$(printf '%s' "$gk" | tr '\n' ' ')" ;;
esac

# 7b. Whether Gatekeeper gets to decide at all, which is the variable that
#     actually governs a first launch. An unquarantined copy — anything built
#     here, or carried over by scp — is not assessed, so a bundle that would be
#     refused after a download starts without a murmur.
if xattr -p com.apple.quarantine "$app" >/dev/null 2>&1; then
    note "it carries com.apple.quarantine" "$(xattr -p com.apple.quarantine "$app" 2>/dev/null)"
else
    note "it carries com.apple.quarantine" "no — so Gatekeeper is not consulted"
fi

# 8. Whether the App Sandbox actually engaged, which is a fact about a *run*
#    rather than about the bundle. The container directory is made on first
#    launch and by nothing else, so its absence after a launch means the
#    entitlement was carried and not honoured — the failure that looks like
#    success in every static check above.
container="${HOME}/Library/Containers/${bundle_id}"
if [ -d "$container" ]; then
    ok "a sandbox container exists" "$(basename "$container")"
    state="${container}/Data/.local/state/slipcase-desktop"
    if [ -d "$state" ]; then
        note "  and it remembered a folder" "$(cat "${state}/last-folder" 2>/dev/null || echo 'no last-folder yet')"
    fi
else
    note "a sandbox container exists" "not yet — launch it once and re-run"
fi

# 9. Launch Services, which is what makes a double-click reach this application
#    at all. `-dump` is large; ask it only about our identifier.
if /System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister \
        -dump 2>/dev/null | grep -q "$bundle_id"; then
    ok "Launch Services knows the bundle" "$bundle_id"
else
    bad "Launch Services knows the bundle" "not registered — a double-click will not arrive"
fi

echo
if [ "$findings" -eq 0 ]; then
    echo "Nothing mechanical is wrong with this install."
else
    echo "${findings} thing(s) to write down in CHECKLIST.md."
fi
echo "The rest needs eyes: the layout at 2x, the icon, the frame, and what"
echo "Gatekeeper shows a person rather than what spctl reports."
exit 0
