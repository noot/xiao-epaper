use ab_glyph::{FontRef, PxScale};
use chrono::{Datelike, Local, NaiveDate, Timelike};
use image::{GrayImage, Luma};
use imageproc::drawing::{draw_filled_rect_mut, draw_line_segment_mut, draw_text_mut};
use imageproc::rect::Rect;
use serde::Deserialize;

use crate::pet;

const W: u32 = 800;
const H: u32 = 480;

const BLACK: Luma<u8> = Luma([0u8]);
const WHITE: Luma<u8> = Luma([255u8]);
const GRAY: Luma<u8> = Luma([140u8]);
const LIGHT_GRAY: Luma<u8> = Luma([200u8]);
const DARK_GRAY: Luma<u8> = Luma([60u8]);
const TODAY_BG: Luma<u8> = Luma([210u8]);

const FONT_BYTES: &[u8] = include_bytes!("../../assets/font.ttf");

// ── Public API ──────────────────────────────────────────────────

pub fn render_dashboard() -> eyre::Result<GrayImage> {
    let font = FontRef::try_from_slice(FONT_BYTES).map_err(|e| eyre::eyre!("font: {e}"))?;
    let data = fetch_data();
    Ok(render(&font, &data))
}

// ── Data ────────────────────────────────────────────────────────

struct DashData {
    year: i32,
    month_name: String,
    today_day: u32,
    first_weekday: u32,
    days_in_month: u32,
    moon_phases: Vec<f64>,
    time_str: String,
    date_str: String,
    hour: u32,
    minute: u32,
    weather: Weather,
}

struct Weather {
    temp_c: f64,
    condition: String,
    weather_code: i64,
    high_c: f64,
    low_c: f64,
    sunrise: String,
    sunset: String,
}

fn fetch_data() -> DashData {
    let now = Local::now();
    let year = now.year();
    let month = now.month();
    let today_day = now.day();

    let dim = days_in_month(year, month);
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let first_weekday = first.weekday().num_days_from_sunday();
    let month_name = now.format("%B").to_string();

    let mut phases = Vec::with_capacity(dim as usize);
    for d in 1..=dim {
        let date = NaiveDate::from_ymd_opt(year, month, d).unwrap();
        phases.push(moon_phase(date));
    }

    let weather = fetch_weather();

    DashData {
        year,
        month_name,
        today_day,
        first_weekday,
        days_in_month: dim,
        moon_phases: phases,
        time_str: now.format("%H:%M").to_string(),
        date_str: now.format("%a, %b %-d").to_string(),
        hour: now.hour(),
        minute: now.minute(),
        weather,
    }
}

// ── Render ──────────────────────────────────────────────────────

fn render(font: &FontRef, data: &DashData) -> GrayImage {
    let mut img = GrayImage::from_pixel(W, H, WHITE);

    // Layout: left panel (weather/time ~280px) | right panel (calendar)
    let left_w: i32 = 270;
    let margin: i32 = 16;
    let cal_x = left_w + 12;
    let cal_w = W as i32 - cal_x - margin;
    let cell_w = cal_w / 7;

    // ── Left panel: time, date, weather, sun ────────────────────

    // Time
    draw_text_mut(&mut img, BLACK, margin, margin, scale(52.0), font, &data.time_str);

    // Date
    draw_text_mut(&mut img, DARK_GRAY, margin, margin + 56, scale(22.0), font, &data.date_str);

    // Divider
    hline(&mut img, margin + 88, margin, left_w - margin);

    // Temp
    let temp_str = format!("{:.0}°C", data.weather.temp_c);
    draw_text_mut(&mut img, BLACK, margin, margin + 98, scale(48.0), font, &temp_str);

    // Condition
    draw_text_mut(&mut img, DARK_GRAY, margin, margin + 150, scale(20.0), font, &data.weather.condition);

    // Hi/Lo
    let hilo = format!("H {:.0}°  L {:.0}°", data.weather.high_c, data.weather.low_c);
    draw_text_mut(&mut img, GRAY, margin, margin + 176, scale(18.0), font, &hilo);

    // Divider
    hline(&mut img, margin + 204, margin, left_w - margin);

    // Sunrise/sunset
    let rise = format!("\u{2191} {}", data.weather.sunrise);
    let set = format!("\u{2193} {}", data.weather.sunset);
    draw_text_mut(&mut img, DARK_GRAY, margin, margin + 214, scale(22.0), font, &rise);
    draw_text_mut(&mut img, GRAY, margin, margin + 242, scale(22.0), font, &set);

    // Divider
    hline(&mut img, margin + 274, margin, left_w - margin);

    // Pet tamagotchi in bottom-left corner
    let today_phase = data.moon_phases[(data.today_day - 1) as usize];
    let state = pet::pick_state(data.hour, data.weather.weather_code, data.weather.temp_c, today_phase);
    // Pet walks back and forth across bottom of left panel
    let pet_scale = 2;
    let pet_w = 15 * pet_scale; // sprite width in pixels
    let walk_min = margin + 4;
    let walk_max = left_w - pet_w - 8; // stop before divider
    let walk_range = walk_max - walk_min;
    let tick = (data.hour * 60 + data.minute) as i32;
    // Bounce: tick mod (2*range) gives triangle wave
    let cycle = 2 * walk_range;
    let pos_in_cycle = tick % cycle;
    let (pet_x, flipped) = if pos_in_cycle < walk_range {
        (walk_min + pos_in_cycle, false) // walking right
    } else {
        (walk_min + cycle - pos_in_cycle, true) // walking left
    };
    let pet_y = margin + 400;
    pet::draw_pet(&mut img, state, pet_x, pet_y, pet_scale, flipped);

    // Vertical divider between panels
    vline(&mut img, left_w, 0, H as i32);

    // ── Right panel: calendar grid ──────────────────────────────

    let cal_top: i32 = margin;

    // Month/year header
    let title = format!("{} {}", data.month_name, data.year);
    draw_text_mut(&mut img, BLACK, cal_x + 8, cal_top, scale(28.0), font, &title);

    // Day-of-week labels
    let dow_y = cal_top + 36;
    let dow_labels = ["S", "M", "T", "W", "T", "F", "S"];
    for (i, label) in dow_labels.iter().enumerate() {
        let x = cal_x + (i as i32) * cell_w + cell_w / 2 - 5;
        draw_text_mut(&mut img, GRAY, x, dow_y, scale(16.0), font, label);
    }

    // Grid
    let grid_top = dow_y + 24;
    let total_slots = data.first_weekday + data.days_in_month;
    let rows = ((total_slots + 6) / 7) as i32;
    let available_h = H as i32 - grid_top - margin;
    let cell_h = available_h / rows;

    for day in 1..=data.days_in_month {
        let slot = (data.first_weekday + day - 1) as i32;
        let col = slot % 7;
        let row = slot / 7;

        let cx = cal_x + col * cell_w;
        let cy = grid_top + row * cell_h;

        let is_today = day == data.today_day;

        if is_today {
            draw_filled_rect_mut(
                &mut img,
                Rect::at(cx + 1, cy + 1).of_size((cell_w - 2) as u32, (cell_h - 2) as u32),
                TODAY_BG,
            );
        }

        // Day number
        let day_str = format!("{}", day);
        let text_color = if is_today { BLACK } else { DARK_GRAY };
        draw_text_mut(&mut img, text_color, cx + 4, cy + 2, scale(16.0), font, &day_str);

        // Moon glyph — small, top-right corner of cell
        let phase = data.moon_phases[(day - 1) as usize];
        let moon_r = 7;
        let mcx = cx + cell_w - moon_r - 4;
        let mcy = cy + moon_r + 4;
        draw_moon(&mut img, mcx, mcy, moon_r, phase);
    }

    // Grid lines
    for row in 0..=rows {
        let y = grid_top + row * cell_h;
        hline(&mut img, y, cal_x, cal_x + 7 * cell_w);
    }
    for col in 0..=7 {
        let x = cal_x + col * cell_w;
        vline(&mut img, x, grid_top, grid_top + rows * cell_h);
    }

    img
}

// ── Moon phase ──────────────────────────────────────────────────

fn moon_phase(date: NaiveDate) -> f64 {
    let ref_new_moon = 2451550.1;
    let jd = julian_day(date);
    let synodic = 29.53058770576;
    let phase = ((jd - ref_new_moon) % synodic) / synodic;
    if phase < 0.0 { phase + 1.0 } else { phase }
}

fn julian_day(date: NaiveDate) -> f64 {
    let y = date.year() as f64;
    let m = date.month() as f64;
    let d = date.day() as f64;
    let a = ((14.0 - m) / 12.0).floor();
    let y2 = y + 4800.0 - a;
    let m2 = m + 12.0 * a - 3.0;
    d + ((153.0 * m2 + 2.0) / 5.0).floor() + 365.0 * y2
        + (y2 / 4.0).floor() - (y2 / 100.0).floor() + (y2 / 400.0).floor()
        - 32045.0 + 0.5
}

fn days_in_month(year: i32, month: u32) -> u32 {
    if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap()
    .pred_opt()
    .unwrap()
    .day()
}

// ── Moon rendering ──────────────────────────────────────────────

fn draw_moon(img: &mut GrayImage, cx: i32, cy: i32, r: i32, phase: f64) {
    if r <= 0 { return; }
    for dy in -r..=r {
        for dx in -r..=r {
            if dx * dx + dy * dy > r * r { continue; }
            let px = cx + dx;
            let py = cy + dy;
            if px < 0 || py < 0 || px >= W as i32 || py >= H as i32 { continue; }
            let x_norm = dx as f64 / r as f64;
            let lit = is_pixel_lit(x_norm, phase);
            img.put_pixel(px as u32, py as u32, if lit { BLACK } else { LIGHT_GRAY });
        }
    }
    // no outline — cleaner look
}

fn is_pixel_lit(x_norm: f64, phase: f64) -> bool {
    if phase <= 0.0 || phase >= 1.0 { return false; }
    if phase <= 0.5 {
        // Waxing: right side lit, growing. At 0 = nothing, at 0.5 = full
        let threshold = 1.0 - 4.0 * phase; // 1.0 → -1.0
        x_norm > threshold
    } else {
        // Waning: left side stays lit, shrinking from right. At 0.5 = full, at 1.0 = nothing
        let threshold = -1.0 + 4.0 * (phase - 0.5); // -1.0 → 1.0
        x_norm < -threshold  // i.e. x_norm < (1.0 - 4*(phase-0.5))
    }
}

fn draw_circle_outline(img: &mut GrayImage, cx: i32, cy: i32, r: i32, color: Luma<u8>) {
    let mut x = r;
    let mut y = 0;
    let mut err = 1 - r;
    while x >= y {
        for &(px, py) in &[
            (cx+x,cy+y),(cx-x,cy+y),(cx+x,cy-y),(cx-x,cy-y),
            (cx+y,cy+x),(cx-y,cy+x),(cx+y,cy-x),(cx-y,cy-x),
        ] {
            if px >= 0 && py >= 0 && (px as u32) < W && (py as u32) < H {
                img.put_pixel(px as u32, py as u32, color);
            }
        }
        y += 1;
        if err < 0 { err += 2 * y + 1; } else { x -= 1; err += 2 * (y - x) + 1; }
    }
}

// ── Weather ─────────────────────────────────────────────────────

#[derive(Deserialize)]
struct OpenMeteoResp {
    current: Option<CurrentData>,
    daily: Option<DailyData>,
}

#[derive(Deserialize)]
struct CurrentData {
    temperature_2m: Option<f64>,
    weather_code: Option<i64>,
}

#[derive(Deserialize)]
struct DailyData {
    temperature_2m_max: Option<Vec<f64>>,
    temperature_2m_min: Option<Vec<f64>>,
    sunrise: Option<Vec<String>>,
    sunset: Option<Vec<String>>,
}

fn fetch_weather() -> Weather {
    match fetch_weather_live() {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("weather fetch failed: {e}");
            Weather {
                temp_c: 0.0, condition: "N/A".into(), weather_code: 0,
                high_c: 0.0, low_c: 0.0,
                sunrise: "--:--".into(), sunset: "--:--".into(),
            }
        }
    }
}

fn fetch_weather_live() -> eyre::Result<Weather> {
    let url = "https://api.open-meteo.com/v1/forecast\
        ?latitude=49.4928&longitude=-117.2948\
        &current=temperature_2m,weather_code\
        &daily=temperature_2m_max,temperature_2m_min,sunrise,sunset\
        &timezone=America/Vancouver\
        &forecast_days=1";

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let resp: OpenMeteoResp = client.get(url).send()?.json()?;
    let current = resp.current.ok_or_else(|| eyre::eyre!("no current"))?;
    let daily = resp.daily.ok_or_else(|| eyre::eyre!("no daily"))?;

    let code = current.weather_code.unwrap_or(0);

    let sunrise = daily.sunrise.and_then(|v| v.into_iter().next())
        .map(|s| s.split('T').nth(1).unwrap_or(&s).to_string())
        .unwrap_or_else(|| "--:--".into());
    let sunset = daily.sunset.and_then(|v| v.into_iter().next())
        .map(|s| s.split('T').nth(1).unwrap_or(&s).to_string())
        .unwrap_or_else(|| "--:--".into());

    Ok(Weather {
        temp_c: current.temperature_2m.unwrap_or(0.0),
        condition: wmo_description(code).into(),
        weather_code: code,
        high_c: daily.temperature_2m_max.and_then(|v| v.first().copied()).unwrap_or(0.0),
        low_c: daily.temperature_2m_min.and_then(|v| v.first().copied()).unwrap_or(0.0),
        sunrise,
        sunset,
    })
}

fn wmo_description(code: i64) -> &'static str {
    match code {
        0 => "Clear", 1 => "Mostly clear", 2 => "Partly cloudy", 3 => "Overcast",
        45 | 48 => "Fog", 51..=55 => "Drizzle", 56 | 57 => "Freezing drizzle",
        61 => "Light rain", 63 => "Rain", 65 => "Heavy rain",
        66 | 67 => "Freezing rain", 71 => "Light snow", 73 => "Snow",
        75 => "Heavy snow", 77 => "Snow grains", 80..=82 => "Showers",
        85 | 86 => "Snow showers", 95 => "Thunderstorm", 96 | 99 => "T-storm + hail",
        _ => "Unknown",
    }
}

// ── Drawing helpers ─────────────────────────────────────────────

fn scale(size: f32) -> PxScale { PxScale::from(size) }

fn hline(img: &mut GrayImage, y: i32, x0: i32, x1: i32) {
    if y >= 0 && y < H as i32 {
        draw_line_segment_mut(img, (x0 as f32, y as f32), (x1 as f32, y as f32), LIGHT_GRAY);
    }
}

fn vline(img: &mut GrayImage, x: i32, y0: i32, y1: i32) {
    if x >= 0 && x < W as i32 {
        draw_line_segment_mut(img, (x as f32, y0 as f32), (x as f32, y1 as f32), LIGHT_GRAY);
    }
}
