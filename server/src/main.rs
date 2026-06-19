use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use eyre::WrapErr as _;
use image::GrayImage;
use tracing::info;

mod dashboard;
mod pet;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 480;
const ROW_BYTES: usize = (WIDTH as usize) / 8;
const FB_SIZE: usize = ROW_BYTES * HEIGHT as usize;

struct AppState {
    image_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let image_path = std::env::args().nth(1).map(PathBuf::from);

    if let Some(ref p) = image_path {
        if !p.exists() {
            eyre::bail!("image not found: {}", p.display());
        }
        info!("serving {} at /framebuffer", p.display());
    }

    info!("dashboard available at /dashboard");

    let state = Arc::new(AppState { image_path });

    let mut app = axum::Router::new()
        .route("/dashboard", get(serve_dashboard));

    // Only add /framebuffer if an image path was provided
    if state.image_path.is_some() {
        app = app.route("/framebuffer", get(serve_framebuffer));
    }

    let app = app.with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .wrap_err("failed to bind")?;
    axum::serve(listener, app).await.wrap_err("server error")?;

    Ok(())
}

// ── /dashboard — live-rendered calendar + weather ───────────────

async fn serve_dashboard() -> impl IntoResponse {
    let result = tokio::task::spawn_blocking(|| {
        let img = dashboard::render_dashboard()?;
        Ok::<_, eyre::Report>(grayscale_to_framebuffer(&img))
    })
    .await
    .unwrap();

    match result {
        Ok(fb) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/octet-stream")],
            fb,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("dashboard render failed: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

// ── /framebuffer — static image from file ───────────────────────

async fn serve_framebuffer(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let path = match &state.image_path {
        Some(p) => p.clone(),
        None => return StatusCode::NOT_FOUND.into_response(),
    };

    match render_file(&path) {
        Ok(fb) => (
            StatusCode::OK,
            [(header::CONTENT_TYPE, "application/octet-stream")],
            fb,
        )
            .into_response(),
        Err(e) => {
            tracing::error!("render failed: {e:?}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

fn render_file(path: &PathBuf) -> eyre::Result<Vec<u8>> {
    let img = image::open(path).wrap_err("failed to open image")?;

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

    let ox = (WIDTH - sw) / 2;
    let oy = (HEIGHT - sh) / 2;

    let mut canvas = GrayImage::from_pixel(WIDTH, HEIGHT, image::Luma([255u8]));
    for y in 0..sh {
        for x in 0..sw {
            canvas.put_pixel(ox + x, oy + y, *resized.get_pixel(x, y));
        }
    }

    info!("rendered {}x{} -> 1bpp {}B", iw, ih, FB_SIZE);
    Ok(grayscale_to_framebuffer(&canvas))
}

// ── Shared: Floyd-Steinberg dither + 1bpp pack ──────────────────

fn grayscale_to_framebuffer(img: &GrayImage) -> Vec<u8> {
    let w = img.width() as usize;
    let h = img.height() as usize;

    let mut canvas = vec![0.0f32; w * h];
    for y in 0..h {
        for x in 0..w {
            canvas[y * w + x] = img.get_pixel(x as u32, y as u32).0[0] as f32;
        }
    }

    // Floyd-Steinberg dithering
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

    // Pack to 1bpp
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
