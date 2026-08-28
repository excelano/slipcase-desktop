# Changelog

What changed in Slipcase, for the person who installed it.

This is a different document from `git log`, which records why the code is the
way it is and is written for whoever maintains it. Both stores and the Excelano
apt repository show release notes per version, and this is where those come
from: the store listing text is generated from the entry below rather than
written a second time and left to drift.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and
the project follows [semantic versioning](https://semver.org/spec/v2.0.0.html) —
where, below 1.0, a minor bump is how a change that alters what the application
does for an existing container ships.

`packaging/debian/changelog` says the same things in Debian's shape, because
`dpkg` will not read this one. `build-deb.sh` refuses to build a package whose
version that file does not name, which is what keeps the two from parting.

## [0.1.0] - unreleased

First release.

### What it does

Opens a `.slpc` container and shows what is in it: the payload's name, its size,
what the operating system says would open it, and the container's metadata as a
tree you can edit. Three buttons act on the payload — **Open** hands it to
whatever the system has registered for that kind of file, **Extract…** writes it
somewhere you choose, and **Replace…** swaps it for another file. **Save**
writes an edited metadata document back.

A container is one file holding a payload of any type together with a TOML
document describing it, so the description travels with the file rather than
living in a filename, a sidecar, or somebody else's database.

### What it tells you, and does not decide for you

**It reports rather than gates.** The card says what the platform would open the
payload with, and where the platform will not answer it says nothing rather than
guessing — this application ships no table mapping filenames to types and never
inspects a payload's contents to guess at one. The Open button hands the file to
the system either way. What you get is information, not a verdict on whether
something is safe:

- **Where the container came from.** If it was downloaded, the card says so, and
  the payload you extract carries that mark onward — so whatever opens it next
  raises the same warning the container would have. Editing and saving a
  downloaded container keeps the mark too.
- **Whether the payload was stored as an executable file**, on macOS and Linux,
  and that the copy you extract will not be. That is read out of the container
  rather than guessed from the name.
- **Whether the container conforms to the specification**, in the
  specification's own vocabulary, including the two answers that are neither a
  pass nor a failure: a container whose metadata cannot be read is
  *undetermined*, and one declaring a version this build does not implement is
  *out of scope*.

Names are shown with the Unicode characters that reorder text escaped, so a
payload called `report<U+202E>fdp.exe` cannot present itself as a PDF.

### Editing

Comments, key order, and whitespace you did not touch survive a save. Members of
the container this application does not recognise survive it too. A rewrite is
read back and checked before it replaces anything, so a save that would produce
a container the library will not accept changes nothing on disk. A container
nothing was changed in is not rewritten at all.

### What it is built on

Every read, every write, and every verdict comes from `slpc`, the library in
`excelano/slpc-rust`. This application parses no containers of its own. The
format is specified in `excelano/slipcase`, which is the authority on it.

Nothing in the dependency tree compiles C, and the Linux build links only libc,
libgcc and libm.

### Known limits

- There is no preview. The payload is handed to another application rather than
  rendered here.
- A container holds one payload. That is the format's decision, not this
  application's.
- Encrypted members, and compression methods this build was not compiled with,
  are reported rather than opened.
