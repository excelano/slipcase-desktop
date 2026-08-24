# Checklist: the things only a hand can test

`CLAUDE.md` and both handoff briefs have referred to this file since before it
existed. It did not: `git log --diff-filter=A` finds no commit that ever added
it, and the seven defects the Linux walkthrough is said to have found are
recorded in commit messages rather than here. This is the first one, and it
starts with Windows because that is the platform that had never drawn a frame.

A section per platform. Each item says what to do, what should happen, and —
where a run found something — what actually happened.

---

## Windows

Run against a release build, because the console-window item below is a
property of the release profile and passes vacuously in debug.

    cargo build --release
    powershell -ExecutionPolicy Bypass -File packaging\windows\install.ps1

### The association

1. **Explorer draws the icon.** Put a `.slpc` in a folder and look at it in
   Large Icons and in Details. Both should show the card-in-a-case, not the
   blank page Windows gives an unknown extension.
2. **The type reads as a name.** The Type column should say `Slipcase
   Container`, not `SLPC File`.
3. **Double-clicking opens the container, not an empty window.** This is the
   argument path, and until Windows ran it, it had only ever been exercised on
   Linux. The window should show the container's name, its path, `conformant`,
   the payload card, and the metadata tree — not the empty state.
4. **The card names an application for the payload.** A container whose payload
   is `report.pdf` should say what would open it. Silence is a legitimate
   answer for a payload nothing is registered for, so use a name the machine
   has an association for.
5. **The Start menu entry works** and carries the icon.
6. **Uninstalling leaves nothing.** Before uninstalling, open a `.slpc` with
   "always open with" so a `UserChoice` exists — that is the key that outranks
   everything else and the one an uninstaller forgets. Afterwards, no key under
   `HKCU\Software\Classes` for the type or the extension, no `FileExts\.slpc`,
   no Add/Remove Programs entry, no shortcut. Explorer should go back to
   calling it `SLPC File`.

### What the first run found

Two defects, both invisible to 61 tests and both fixed in the same commit.

**A console window opened behind the application.** The executable was built
console-subsystem, so a file manager launching it — which is not attached to a
terminal — got a black console window with the executable's path as its title,
sitting behind the window a person actually wanted. Fixed with
`windows_subsystem = "windows"`, off in debug builds so a panic still has
somewhere to go.

**The window had no icon.** The title bar and the task bar showed the generic
default. `APP_ID` is what answers this on Linux and it does nothing here —
`with_app_id` is Wayland's, and neither egui, eframe, nor winit turns it into
anything on Windows. Fixed by embedding the `.ico` and handing its 64-pixel
entry to the window at startup, because compiling a resource the normal way
needs a tool `DESIGN.md` §2 keeps out of the build.

A third was in the packaging rather than the application: `install.ps1` closed
by telling the reader to check with `assoc` and `ftype`, and both report no
association at all after a successful per-user install. They read the
machine-wide half of the class root only.

### Not yet done by hand

- **A high-density display.** Every size above was checked at 100%. The icon
  carries 20, 40, and 64 for the scaled sizes and none of them has been looked
  at on a display that would ask for one.
- **A second user account**, to confirm a per-user install is invisible to one.
- **An upgrade over an existing install**, rather than a first install.

---

## Linux

Not written. The walkthrough happened — seven defects in layout geometry, font
coverage, frame timing, and controls that were drawn but did nothing — and the
record of it is in `git log` rather than in a file. Reconstructing it from
there is a job for whoever next has the platform in front of them.

---

## macOS

Nothing has run. `HANDOFF-macos.md` has the brief.
