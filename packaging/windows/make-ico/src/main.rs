//! Turn the icon SVG into the rasters Windows wants: the `.ico` the shell reads
//! and the PNGs an MSIX package declares.
//!
//! Linux ships the SVG and lets the desktop rasterize it. Windows has no such
//! step: the shell reads a fixed set of sizes out of an icon directory and the
//! Store reads named PNGs at fixed dimensions, so every size has to exist
//! before the file is shipped. This is the step between, and it is why
//! rasterized artifacts are committed to a repository that otherwise holds only
//! sources.
//!
//! The name is narrower than the job and is kept anyway: `windows.yml` runs
//! this package by name and `packaging/windows/README.md` refers to it, and
//! renaming a tool to widen its remit costs more than the sentence it saves.
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

/// One of the images `AppxManifest.xml.in` names, at the size the Store wants.
struct Asset {
    name: &'static str,
    width: u32,
    height: u32,
    /// How much of the shorter side the drawing occupies, centred.
    fill: f32,
}

/// The five images the manifest names, and nothing else.
///
/// The dimensions are the Store's and are not a choice. `fill` is: a tile is
/// drawn on a coloured plate and Microsoft's tile guidance leaves the icon
/// about two thirds of it, where an icon-shaped asset — the store logo, the
/// application list entry, the file type — is drawn at the size it is given
/// and wants the whole of it, which is also what `slipcase.ico` does at every
/// size it holds. **Only a look at real tiles settles the two thirds**, and
/// that look is in `CHECKLIST.md` rather than here.
///
/// `slipcase.png` is the file type association logo and is a single 256, not a
/// set: without a `resources.pri` the shell resolves no `targetsize-` variants,
/// so the alternative to one large image is one small one.
const ASSETS: &[Asset] = &[
    Asset { name: "StoreLogo.png", width: 50, height: 50, fill: 1.0 },
    Asset { name: "Square44x44Logo.png", width: 44, height: 44, fill: 1.0 },
    Asset { name: "Square150x150Logo.png", width: 150, height: 150, fill: 0.66 },
    Asset { name: "Wide310x150Logo.png", width: 310, height: 150, fill: 0.66 },
    Asset { name: "slipcase.png", width: 256, height: 256, fill: 1.0 },
];

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
    let assets = args
        .next()
        .map_or_else(|| here.join("../assets"), PathBuf::from);

    let svg = std::fs::read(&source)?;
    let tree = usvg::Tree::from_data(&svg, &usvg::Options::default())?;

    let mut icon = ico::IconDir::new(ico::ResourceType::Icon);
    for &size in SIZES {
        let image = icon_image(&tree, size)?;
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

    std::fs::create_dir_all(&assets)?;
    for asset in ASSETS {
        let pixmap = draw(&tree, asset.width, asset.height, asset.fill)?;
        std::fs::write(assets.join(asset.name), pixmap.encode_png()?)?;
        println!(
            "wrote {} — {}×{}",
            assets.join(asset.name).display(),
            asset.width,
            asset.height
        );
    }

    Ok(())
}

/// The drawing centred on a canvas of the given size.
///
/// Every raster here comes through this, so the wide tile is the same rendering
/// as the square one rather than a square image somebody stretched. The drawing
/// is square and two of the five canvases are not, which is the whole reason
/// this takes a width and a height rather than a size.
fn draw(
    tree: &usvg::Tree,
    width: u32,
    height: u32,
    fill: f32,
) -> Result<tiny_skia::Pixmap, Box<dyn std::error::Error>> {
    let mut pixmap =
        tiny_skia::Pixmap::new(width, height).ok_or("a pixmap of that size could not be made")?;

    // The SVG is drawn on a 64-unit grid; this is the only scaling involved.
    #[allow(clippy::cast_precision_loss)]
    let (w, h) = (width as f32, height as f32);
    let side = w.min(h) * fill;
    let scale = side / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_translate((w - side) / 2.0, (h - side) / 2.0)
            .pre_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    Ok(pixmap)
}

/// The drawing at one size, as straight rather than premultiplied alpha.
///
/// `tiny_skia` renders into premultiplied pixels and an icon directory holds
/// straight ones. Handing the pixmap's bytes over unconverted looks right
/// everywhere the drawing is opaque and wrong along every antialiased edge,
/// which on this icon is its entire outline. Its own PNG encoder does the same
/// conversion, which is why the assets above need no equivalent.
fn icon_image(
    tree: &usvg::Tree,
    size: u32,
) -> Result<ico::IconImage, Box<dyn std::error::Error>> {
    let pixmap = draw(tree, size, size, 1.0)?;
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
