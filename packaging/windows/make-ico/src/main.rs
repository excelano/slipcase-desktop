//! Turn the icon SVG into the `.ico` Windows wants.
//!
//! Linux ships the SVG and lets the desktop rasterize it. Windows has no such
//! step: the shell reads a fixed set of sizes out of an icon directory, so the
//! sizes have to exist before the file is shipped. This is the step between,
//! and it is why a rasterized artifact is committed to a repository that
//! otherwise holds only sources.
//!
//! Author: David M. Anderson
//! Built with AI assistance (Claude, Anthropic)

#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};

use resvg::tiny_skia;
use resvg::usvg;

/// The sizes the Windows shell asks for.
///
/// 16, 32, and 48 are the list, the desktop, and the tile; 256 is what the
/// extra-large view and the file properties dialog use. 20, 24, 40, 64, and
/// 128 are the same three sizes again at the display scalings Windows offers,
/// and without them the shell picks a neighbour and resamples it, which is
/// visibly softer than rendering the vector at the size wanted.
const SIZES: &[u32] = &[16, 20, 24, 32, 40, 48, 64, 128, 256];

/// Above this, an entry is stored as PNG rather than as a bitmap.
///
/// PNG entries are read by Vista and later and nothing older, and a 256-pixel
/// bitmap entry costs 256 KiB where the PNG costs a few. Below the line the
/// bitmap is kept, because it is the format every consumer of an icon
/// directory has always understood and the saving there is not worth anything.
const PNG_ABOVE: u32 = 48;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut args = std::env::args_os().skip(1);
    let source = args.next().map_or_else(
        || here.join("../../linux/icons/slipcase-desktop.svg"),
        PathBuf::from,
    );
    let destination = args
        .next()
        .map_or_else(|| here.join("../slipcase.ico"), PathBuf::from);

    let svg = std::fs::read(&source)?;
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default())?;

    let mut icon = ico::IconDir::new(ico::ResourceType::Icon);
    for &size in SIZES {
        let image = render(&tree, size)?;
        let entry = if size > PNG_ABOVE {
            ico::IconDirEntry::encode_as_png(&image)?
        } else {
            ico::IconDirEntry::encode_as_bmp(&image)?
        };
        icon.add_entry(entry);
    }

    let file = std::fs::File::create(&destination)?;
    icon.write(file)?;

    println!(
        "wrote {} — {} sizes: {}",
        destination.display(),
        SIZES.len(),
        SIZES
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", ")
    );
    Ok(())
}

/// The drawing at one size, as straight rather than premultiplied alpha.
///
/// `tiny_skia` renders into premultiplied pixels and an icon directory holds
/// straight ones. Handing the pixmap's bytes over unconverted looks right
/// everywhere the drawing is opaque and wrong along every antialiased edge,
/// which on this icon is its entire outline.
fn render(tree: &usvg::Tree, size: u32) -> Result<ico::IconImage, Box<dyn std::error::Error>> {
    let mut pixmap =
        tiny_skia::Pixmap::new(size, size).ok_or("a pixmap of that size could not be made")?;

    // The SVG is drawn on a 64-unit grid; this is the only scaling involved.
    #[allow(clippy::cast_precision_loss)]
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );

    let mut rgba = Vec::with_capacity((size * size * 4) as usize);
    for pixel in pixmap.pixels() {
        let straight = pixel.demultiply();
        rgba.extend_from_slice(&[
            straight.red(),
            straight.green(),
            straight.blue(),
            straight.alpha(),
        ]);
    }
    Ok(ico::IconImage::from_rgba_data(size, size, rgba))
}
