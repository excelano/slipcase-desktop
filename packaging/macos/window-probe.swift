// Ask the window server whether an application has drawn a window.
//
// Author: David M. Anderson
// Built with AI assistance (Claude, Anthropic)
//
// This exists for CI. `.github/workflows/apple-silicon.yml` runs the suite and
// the corpus natively on arm64, and the one thing no runner reached was the
// window — which is exactly the code the arm64 walkthrough exists for, since
// `src/opened_document.rs` is the only `unsafe` in this crate and macOS
// delivers a double-clicked document through an Apple Event rather than argv.
//
// **A screenshot is the wrong assertion and this repository already knows it.**
// `CHECKLIST.md` records `screencapture` returning the desktop and the menu bar
// with every window omitted, reporting no error while doing it, and two
// byte-identical empty captures as the only tell. A job asserting on pixels
// would have gone green against that. So this asks the window server directly.
//
// `CGWindowListCopyWindowInfo` gives the owner name and the bounds without
// Screen Recording permission; only the window *title* needs it, which is why
// no title is read here. That claim is the reason this prints what it saw
// rather than only its verdict: if the list is empty the API is restricted and
// the answer is unknown, and if it holds other applications' windows but none
// of ours then the application genuinely drew nothing. Those are different
// findings and a bare exit code cannot tell them apart.

import CoreGraphics
import Foundation

let wanted = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : "Slipcase"

guard
    let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID)
        as? [[String: Any]]
else {
    print("the window server returned nothing at all — the list is unavailable")
    exit(2)
}

var owners: Set<String> = []
var ours: [(Double, Double, Int)] = []

for w in windows {
    guard let owner = w[kCGWindowOwnerName as String] as? String else { continue }
    owners.insert(owner)
    guard owner == wanted else { continue }
    let layer = w[kCGWindowLayer as String] as? Int ?? -1
    guard
        let b = w[kCGWindowBounds as String] as? [String: Any],
        let width = b["Width"] as? Double,
        let height = b["Height"] as? Double
    else { continue }
    ours.append((width, height, layer))
}

print("on-screen windows: \(windows.count), from \(owners.count) applications")
print("owners: \(owners.sorted().joined(separator: ", "))")

if windows.isEmpty {
    print("VERDICT: unknown — the window server listed nothing, so this says")
    print("         nothing about \(wanted). Not a pass and not a failure.")
    exit(2)
}

// Layer 0 is an ordinary application window. A menu, a panel or a shadow sits
// elsewhere, and counting one of those as the application's window would make
// this pass against a build that draws no interface at all.
let real = ours.filter { $0.0 > 1 && $0.1 > 1 && $0.2 == 0 }
for (w, h, layer) in ours {
    print(String(format: "  %@ window: %.0f x %.0f at layer %d", wanted, w, h, layer))
}

if real.isEmpty {
    print("VERDICT: FAILED — \(windows.count) windows are listed and none of them is")
    print("         an ordinary window belonging to \(wanted).")
    exit(1)
}

print("VERDICT: passed — \(wanted) has \(real.count) ordinary window(s) on screen")
exit(0)
