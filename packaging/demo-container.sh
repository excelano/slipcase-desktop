#!/bin/sh
# Build the container the screenshots are taken of, the same way on every
# platform.
#
# `store-listing.md` described this container in prose — a real one-page PDF, a
# metadata document rich enough to exercise every renderer — and said rebuilding
# it was a few lines. It was not in the repository, so the four Windows
# screenshots could not be reproduced anywhere, macOS was about to invent a
# second container for its own, and the website would have invented a third.
# Three demonstrations of the same application that do not look alike is not a
# thing to discover after two listings are live.
#
#   ./packaging/demo-container.sh                    # writes ./dist/demo.pdf.slpc
#   ./packaging/demo-container.sh --out DIR
#
# On Windows, run it with the shell Git for Windows ships, the way
# `build-msix.ps1` runs `version.sh`:
#
#   & "$env:ProgramFiles\Git\bin\bash.exe" packaging/demo-container.sh
#
# It needs `zip`, which is on macOS and every Linux desktop and comes with Git
# for Windows. Nothing here is generated randomly and the archive's timestamps
# are pinned, so two platforms building it get the same bytes — checked by
# building under two timezones and comparing, not asserted. That matters now
# that the file is downloadable from excelano.com and a store submission tells a
# certification reviewer it is the container the screenshots show.
#
# **The subject is invented.** No real person, organisation, matter or date
# appears in it, because it goes in two store listings and on a website.
#
# Author: David M. Anderson
# Built with AI assistance (Claude, Anthropic)
set -eu

here=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
root=$(CDPATH= cd -- "${here}/.." && pwd)
outdir="${root}/dist"

while [ $# -gt 0 ]; do
    case "$1" in
        --out) outdir="${2:?--out needs a directory}"; shift 2 ;;
        -h|--help) sed -n '2,26p' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "demo-container.sh: unknown argument $1" >&2; exit 2 ;;
    esac
done

command -v zip >/dev/null || {
    echo "demo-container.sh: no zip on PATH" >&2
    exit 1
}

mkdir -p "$outdir"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT INT TERM

# A one-page PDF, written out rather than carried as a blob so that this whole
# container is source. It is assembled in three passes because a PDF records its
# own byte offsets: the content stream is written first and measured, then the
# objects, then a cross-reference table built from where those objects actually
# landed.
#
# **Both of those were wrong in the first version and one viewer did not care**,
# which is the reason for the passes. The stream declared `Length 92` over 87
# bytes and there was no cross-reference table at all; poppler rendered it
# anyway, because every mainstream viewer repairs a broken xref rather than
# refusing. A payload that only opens in viewers that repair is not what belongs
# in two store listings, so this computes both instead of asserting them.
stream="${stage}/content"
printf '%s\n' 'BT /F1 18 Tf 72 760 Td (Quarterly Report) Tj 0 -28 Td /F1 11 Tf (Sample document) Tj ET' > "$stream"
length=$(wc -c < "$stream" | tr -d ' ')

pdf="${stage}/quarterly-report.pdf"
{
    printf '%%PDF-1.4\n'
    printf '1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n'
    printf '2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n'
    printf '3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 595 842]/Resources<</Font<</F1 4 0 R>>>>/Contents 5 0 R>>endobj\n'
    printf '4 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n'
    printf '5 0 obj<</Length %s>>stream\n' "$length"
    cat "$stream"
    printf 'endstream endobj\n'
} > "$pdf"

# Where each object actually starts, asked of the file rather than counted by
# hand. `grep -bo` gives a byte offset, which is what an xref entry is.
xref_at=$(wc -c < "$pdf" | tr -d ' ')
{
    printf 'xref\n0 6\n'
    printf '0000000000 65535 f \n'
    for n in 1 2 3 4 5; do
        off=$(grep -bo "^${n} 0 obj" "$pdf" | head -1 | cut -d: -f1)
        [ -n "$off" ] || { echo "demo-container.sh: object $n not found in the PDF" >&2; exit 1; }
        printf '%010d 00000 n \n' "$off"
    done
    printf 'trailer<</Root 1 0 R/Size 6>>\nstartxref\n%s\n%%%%EOF\n' "$xref_at"
} >> "$pdf"

# The metadata is the thing being photographed: the tree is what this
# application is for, and the walkthrough fixtures have three keys between them.
# Every renderer `src/tree.rs` carries appears here at least once — string,
# integer, float, boolean, array, array of tables, nested table, and the two
# datetime shapes that render differently.
cat > "${stage}/slipcase.metadata.toml" <<'TOML'
slipcase_version = "1.0"

# A description written to be read, because the tree is what a person is
# looking at in a screenshot.
title = "Quarterly Report — Northwind Division"
summary = "Consolidated figures and commentary for the quarter."
reference = "NW-2026-Q2-014"
final = true
pages = 12
confidence = 0.94

keywords = ["quarterly", "consolidated", "northwind", "internal"]

[payload]
file = "quarterly-report.pdf"

[dates]
prepared = 2026-04-18
issued = 2026-04-30T09:15:00Z
review_due = 2026-07-31

[origin]
system = "Northwind Reporting"
version = "3.2.1"
department = "Finance"

[origin.contact]
name = "Reporting Desk"
mailbox = "reporting@example.invalid"

[[revisions]]
version = 1
date = 2026-04-18
note = "First draft circulated for comment."

[[revisions]]
version = 2
date = 2026-04-26
note = "Figures restated after the divisional close."

[[revisions]]
version = 3
date = 2026-04-30
note = "Issued."
TOML

out="${outdir}/quarterly-report.pdf.slpc"
rm -f "$out"

# **The archive's two timestamps are pinned, so this is the same file
# everywhere.** Without this the container records the moment it was built, and
# the Windows screenshots, the macOS screenshots, the website download and a
# certification tester's copy would be four files with four hashes — which the
# header above claimed was fine and it is not, now that the store submission
# points a reviewer at a URL and says it is what the pictures show.
#
# Measured rather than assumed, because the first check said reproducible and
# was wrong: two builds a few seconds apart matched, DOS timestamps having
# two-second granularity, and the same test with a minute between them did not.
#
# Both halves are needed. `touch` fixes the instant, and `TZ=UTC` fixes what
# ZIP writes for it: the DOS timestamp field is local time with no zone, so the
# same instant in Houston and in Tokyo is two different fields. The date is the
# one the metadata already gives as `dates.issued`, so the archive agrees with
# the document it carries rather than naming some arbitrary epoch.
touch -d '2026-04-30T09:15:00Z' \
    "${stage}/slipcase.metadata.toml" "${stage}/quarterly-report.pdf"

# The metadata first, which is the order every other container this project
# builds uses, and `zip -X` so no extra fields carry this machine's identity
# into a file that goes in two store listings.
( cd "$stage" && TZ=UTC zip -q -X "$out" slipcase.metadata.toml quarterly-report.pdf )

echo "built $out"
echo "  payload   quarterly-report.pdf ($(wc -c < "${stage}/quarterly-report.pdf") bytes)"
echo "  metadata  $(grep -c . < "${stage}/slipcase.metadata.toml") non-empty lines"
echo
echo "for the 'arrived from elsewhere' screenshot, mark a copy the way a download would:"
echo "  Linux    setfattr -n user.xdg.origin.url -v https://example.invalid/q2 <copy>"
echo "  macOS    xattr -w com.apple.quarantine '0083;68ae0000;Safari;' <copy>"
echo "  Windows  Add-Content -Path <copy>:Zone.Identifier -Value \"[ZoneTransfer]\`nZoneId=3\""
