use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::State;
use axum::http::StatusCode;
use axum::http::header;
use axum::response::IntoResponse;
use axum::routing::get;
use eyre::WrapErr as _;
use tracing::info;

const WIDTH: u32 = 800;
const HEIGHT: u32 = 480;
const ROW_BYTES: usize = (WIDTH as usize) / 8;
const FB_SIZE: usize = ROW_BYTES * HEIGHT as usize;

struct AppState {
    image_path: PathBuf,
}

#[tokio::main]
async fn main() -> eyre::Result<()> {
    tracing_subscriber::fmt::init();

    let image_path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("../assets/image.png"));

    if !image_path.exists() {
        eyre::bail!("image not found: {}", image_path.display());
    }

    info!("serving {} as 1bpp framebuffer", image_path.display());

    let state = Arc::new(AppState { image_path });
    let app = axum::Router::new()
        .route("/framebuffer", get(framebuffer))
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".to_string());
    let addr = format!("0.0.0.0:{port}");
    info!("listening on {addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .wrap_err("failed to bind")?;
    axum::serve(listener, app).await.wrap_err("server error")?;

    Ok(())
}

async fn framebuffer(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    match render(&state.image_path) {
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

fn render(path: &PathBuf) -> eyre::Result<Vec<u8>> {
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

    let mut canvas = vec![255.0f32; (WIDTH * HEIGHT) as usize];
    for y in 0..sh {
        for x in 0..sw {
            let px = resized.get_pixel(x, y).0[0];
            canvas[((oy + y) * WIDTH + (ox + x)) as usize] = px as f32;
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

    let mut fb = vec![0xFFu8; FB_SIZE];
    for y in 0..h {
        for x in 0..w {
            if canvas[y * w + x] < 128.0 {
                let idx = y * ROW_BYTES + (x >> 3);
                fb[idx] &= !(0x80u8 >> (x & 7));
            }
        }
    }

    info!("rendered {}x{} -> 1bpp {}B", iw, ih, FB_SIZE);
    Ok(fb)
}
