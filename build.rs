use std::path::Path;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 480;
const ROW_BYTES: usize = (WIDTH as usize) / 8;
const FB_SIZE: usize = ROW_BYTES * HEIGHT as usize;

fn main() {
    linker_be_nice();
    println!("cargo:rustc-link-arg=-Tlinkall.x");

    println!("cargo:rerun-if-changed=assets");
    convert_assets();
}

fn convert_assets() {
    let assets = Path::new("assets");
    if !assets.is_dir() {
        return;
    }

    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo");
    let out = Path::new(&out_dir);

    let entries = match std::fs::read_dir(assets) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str());
        if ext != Some("png") {
            continue;
        }

        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .expect("filename is valid utf-8");

        let raw = png_to_1bpp(&path);
        let out_path = out.join(format!("{stem}.raw"));
        std::fs::write(&out_path, &raw).expect("can write raw framebuffer");
        eprintln!("converted {path:?} -> {out_path:?}");
    }
}

/// Converts a PNG image to a 1bpp framebuffer suitable for the UC8179 display.
///
/// The image is:
/// 1. Scaled to fit within 800x480, preserving aspect ratio
/// 2. Centered on a white background
/// 3. Converted to grayscale
/// 4. Dithered using Floyd-Steinberg for good e-paper rendering
///
/// Output is 48000 bytes (800x480, 1 bit per pixel, MSB first, 0=black 1=white).
fn png_to_1bpp(path: &Path) -> Vec<u8> {
    let img =
        image::open(path).unwrap_or_else(|e| panic!("failed to open {}: {e}", path.display()));

    let (iw, ih) = (img.width(), img.height());
    let scale = f64::min(WIDTH as f64 / iw as f64, HEIGHT as f64 / ih as f64);
    let (sw, sh) = if scale < 1.0 {
        ((iw as f64 * scale) as u32, (ih as f64 * scale) as u32)
    } else {
        (iw, ih)
    };

    let resized = image::imageops::resize(
        &img.to_luma8(),
        sw,
        sh,
        image::imageops::FilterType::Lanczos3,
    );

    let ox = ((WIDTH - sw) / 2) as i32;
    let oy = ((HEIGHT - sh) / 2) as i32;

    // floyd-steinberg dithering on a padded canvas
    let mut canvas = vec![255.0f32; (WIDTH * HEIGHT) as usize];
    for y in 0..sh {
        for x in 0..sw {
            let px = resized.get_pixel(x, y).0[0];
            canvas[((oy as u32 + y) * WIDTH + (ox as u32 + x)) as usize] = px as f32;
        }
    }

    let w = WIDTH as usize;
    let h = HEIGHT as usize;
    for y in 0..h {
        for x in 0..w {
            let idx = y * w + x;
            let old = canvas[idx];
            let new = if old < 128.0 { 0.0 } else { 255.0 };
            canvas[idx] = new;
            let err = old - new;

            if x + 1 < w {
                canvas[idx + 1] += err * 7.0 / 16.0;
            }
            if y + 1 < h {
                if x > 0 {
                    canvas[(y + 1) * w + (x - 1)] += err * 3.0 / 16.0;
                }
                canvas[(y + 1) * w + x] += err * 5.0 / 16.0;
                if x + 1 < w {
                    canvas[(y + 1) * w + (x + 1)] += err * 1.0 / 16.0;
                }
            }
        }
    }

    // pack into 1bpp: 0=black, 1=white (UC8179 convention)
    let mut fb = vec![0xFFu8; FB_SIZE];
    for y in 0..h {
        for x in 0..w {
            if canvas[y * w + x] < 128.0 {
                let idx = y * ROW_BYTES + (x >> 3);
                fb[idx] &= !(0x80u8 >> (x & 7));
            }
        }
    }

    fb
}

fn linker_be_nice() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() > 1 {
        let kind = &args[1];
        let what = &args[2];

        match kind.as_str() {
            "undefined-symbol" => match what.as_str() {
                "_stack_start" => {
                    eprintln!();
                    eprintln!("is the linker script `linkall.x` missing?");
                    eprintln!();
                }
                "free" | "malloc" | "calloc" => {
                    eprintln!();
                    eprintln!("did you forget the `esp-alloc` dependency?");
                    eprintln!();
                }
                _ => (),
            },
            _ => {
                std::process::exit(1);
            }
        }

        std::process::exit(0);
    }

    println!(
        "cargo:rustc-link-arg=--error-handling-script={}",
        std::env::current_exe().unwrap().display()
    );
}
