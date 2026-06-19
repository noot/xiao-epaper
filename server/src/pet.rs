/// Kawaii tamagotchi-style pixel art pet sprites.
/// Grid-based bitmap sprites scaled up — proper chunky pixel look.
/// Modeled after classic Bandai tamagotchi aesthetic.

use image::{GrayImage, Luma};

const BLACK: Luma<u8> = Luma([0u8]);
const WHITE: Luma<u8> = Luma([255u8]);

#[derive(Debug, Clone, Copy)]
pub enum PetState {
    Happy,
    Sleeping,
    Rainy,
    Hot,
    Cold,
    Stargazing,
}

pub fn pick_state(hour: u32, weather_code: i64, temp_c: f64, moon_phase: f64) -> PetState {
    if hour >= 22 || hour < 6 {
        if moon_phase > 0.47 && moon_phase < 0.53 {
            return PetState::Stargazing;
        }
        return PetState::Sleeping;
    }
    match weather_code {
        51..=67 | 80..=82 | 95 | 96 | 99 => return PetState::Rainy,
        71..=77 | 85 | 86 => return PetState::Cold,
        _ => {}
    }
    if temp_c > 30.0 { return PetState::Hot; }
    if temp_c < 0.0 { return PetState::Cold; }
    if moon_phase > 0.47 && moon_phase < 0.53 && hour >= 18 {
        return PetState::Stargazing;
    }
    PetState::Happy
}

// Sprite: 15 wide × 16 tall
// '#' = black (outline), '.' = white (fill), ' ' = transparent
const SW: usize = 15;
const SH: usize = 16;

fn make_grid(rows: [&str; SH]) -> [[u8; SW]; SH] {
    let mut grid = [[0u8; SW]; SH];
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if x < SW {
                grid[y][x] = match ch {
                    '#' => 1,
                    '.' => 2,
                    _ => 0,
                };
            }
        }
    }
    grid
}

// Base body reference (traced from the tamagotchi image):
//
//  col: 0 1 2 3 4 5 6 7 8 9 A B C D E
//   0:      # #           # #          ears top
//   1:    # # #         # # #          ears
//   2:    # . #         # . #          ear inner
//   3:    # . # # # # # # . #          head top
//   4:    # . . . . . . . . #          head
//   5:  # . . . . . . . . . . #        body (wider)
//   6:  # . [eyes]   [eyes]  . #       face
//   7:  # . . . . . . . . . . #        face
//   8:  # . . . [mouth] . . . #        mouth row
//   9:  # . . . . . . . . . . #        body
//  10:  # . . . . . . . . . . #        body
//  11:    # . . . . . . . . #          body narrows
//  12:    # # # # # # # # # #          body bottom
//  13:    # # . # # # . # #            feet
//  14:    # # . # # # . # #            feet
//  15:      # #       # #              feet bottom

fn happy() -> [[u8; SW]; SH] {
    make_grid([
        //0123456789ABCDE
        "   ##     ##   ", // 0  ears
        "  ###    ###   ", // 1  ears
        "  #.#    #.#   ", // 2  ear white
        "  #.######.#   ", // 3  head top
        "  #........#   ", // 4  head
        " #..........#  ", // 5  body widens
        " #..#....#..#  ", // 6  eyes: wide-set
        " #...####...#  ", // 7  mouth: 4px wide
        " #..........#  ", // 8
        " #..........#  ", // 9
        " #..........#  ", // 10
        "  #........#   ", // 11 body narrows
        "  ##########   ", // 12 body bottom
        "  ##.###.##    ", // 13 feet
        "  ##.###.##    ", // 14 feet
        "   ##   ##     ", // 15 feet bottoms
    ])
}

fn sleeping() -> [[u8; SW]; SH] {
    make_grid([
        "   ##     ##   ",
        "  ###    ###   ",
        "  #.#    #.#   ",
        "  #.######.#   ",
        "  #........#   ",
        " #..........#  ",
        " #..#....#..#  ", // closed eyes (wide-set)
        " #....##....#  ", // small o mouth
        " #..........#  ",
        " #..........#  ",
        " #..........#  ",
        "  #........#   ",
        "  ##########   ",
        "  ##.###.##    ",
        "  ##.###.##    ",
        "   ##   ##     ",
    ])
}

fn rainy() -> [[u8; SW]; SH] {
    make_grid([
        "   ##     ##   ",
        "  ###    ###   ",
        "  #.#    #.#   ",
        "  #.######.#   ",
        "  #........#   ",
        " #..........#  ",
        " #..#....#..#  ", // worried eyes (wide-set)
        " #...#..#...#  ", // zigzag mouth
        " #..........#  ",
        " #..........#  ",
        " #..........#  ",
        "  #........#   ",
        "  ##########   ",
        "  ##.###.##    ",
        "  ##.###.##    ",
        "   ##   ##     ",
    ])
}

fn hot() -> [[u8; SW]; SH] {
    make_grid([
        "   ##     ##   ",
        "  ###    ###   ",
        "  #.#    #.#   ",
        "  #.######.#   ",
        "  #........#   ",
        " #..........#  ",
        " #.#.#..#.#.#  ", // X_X eyes (wide-set)
        " #...####...#  ", // open panting mouth
        " #...#..#...#  ", // mouth bottom
        " #..........#  ",
        " #..........#  ",
        "  #........#   ",
        "  ##########   ",
        "  ##.###.##    ",
        "  ##.###.##    ",
        "   ##   ##     ",
    ])
}

fn cold() -> [[u8; SW]; SH] {
    make_grid([
        "   ##     ##   ",
        "  ###    ###   ",
        "  #.#    #.#   ",
        "  #.######.#   ",
        "  #........#   ",
        " #..........#  ",
        " #..#....#..#  ", // eyes (wide-set)
        " #...####...#  ", // grimace
        " #..........#  ",
        " #..........#  ",
        " #..#....#..#  ", // scarf
        "  #........#   ",
        "  ##########   ",
        "  ##.###.##    ",
        "  ##.###.##    ",
        "   ##   ##     ",
    ])
}

fn stargazing() -> [[u8; SW]; SH] {
    make_grid([
        "   ##     ##   ",
        "  ###    ###   ",
        "  #.#    #.#   ",
        "  #.######.#   ",
        "  #........#   ",
        " #..........#  ",
        " #..#....#..#  ", // big eyes (wide-set)
        " #..#....#..#  ", // (double height)
        " #....##....#  ", // o mouth (awe)
        " #..........#  ",
        "#............# ", // arms out!
        "  #........#   ",
        "  ##########   ",
        "  ##.###.##    ",
        "  ##.###.##    ",
        "   ##   ##     ",
    ])
}

// ── Drawing ─────────────────────────────────────────────────────

fn draw_grid(img: &mut GrayImage, grid: &[[u8; SW]; SH], px: i32, py: i32, scale: i32, flipped: bool) {
    let iw = img.width();
    let ih = img.height();
    for gy in 0..SH {
        for gx in 0..SW {
            let color = match grid[gy][gx] {
                1 => Some(BLACK),
                2 => Some(WHITE),
                _ => None,
            };
            if let Some(c) = color {
                let draw_gx = if flipped { (SW - 1 - gx) as i32 } else { gx as i32 };
                for dy in 0..scale {
                    for dx in 0..scale {
                        let rx = px + draw_gx * scale + dx;
                        let ry = py + (gy as i32) * scale + dy;
                        if rx >= 0 && ry >= 0 && (rx as u32) < iw && (ry as u32) < ih {
                            img.put_pixel(rx as u32, ry as u32, c);
                        }
                    }
                }
            }
        }
    }
}

fn draw_block(img: &mut GrayImage, pattern: &[&[u8]], px: i32, py: i32, scale: i32) {
    let iw = img.width();
    let ih = img.height();
    for (gy, row) in pattern.iter().enumerate() {
        for (gx, &val) in row.iter().enumerate() {
            if val == 1 {
                for dy in 0..scale {
                    for dx in 0..scale {
                        let rx = px + (gx as i32) * scale + dx;
                        let ry = py + (gy as i32) * scale + dy;
                        if rx >= 0 && ry >= 0 && (rx as u32) < iw && (ry as u32) < ih {
                            img.put_pixel(rx as u32, ry as u32, BLACK);
                        }
                    }
                }
            }
        }
    }
}

fn draw_zzz(img: &mut GrayImage, px: i32, py: i32, scale: i32) {
    let z1: &[&[u8]] = &[&[1,1,1,1], &[0,0,1,0], &[0,1,0,0], &[1,1,1,1]];
    let z2: &[&[u8]] = &[&[1,1,1], &[0,1,0], &[1,1,1]];
    let z3: &[&[u8]] = &[&[1,1], &[1,1]];
    draw_block(img, z1, px, py, scale);
    draw_block(img, z2, px + 5*scale, py - 4*scale, scale);
    draw_block(img, z3, px + 9*scale, py - 7*scale, scale);
}

fn draw_drops(img: &mut GrayImage, px: i32, py: i32, scale: i32) {
    let d: &[&[u8]] = &[&[0,1,0], &[1,1,1], &[1,1,1], &[0,1,0]];
    draw_block(img, d, px, py, scale);
    draw_block(img, d, px + 5*scale, py + 3*scale, scale);
}

fn draw_sweat(img: &mut GrayImage, px: i32, py: i32, scale: i32) {
    let d: &[&[u8]] = &[&[0,1,0], &[1,1,1], &[1,1,1], &[0,1,0]];
    draw_block(img, d, px, py, scale);
}

fn draw_stars(img: &mut GrayImage, px: i32, py: i32, scale: i32) {
    let s: &[&[u8]] = &[&[0,0,1,0,0], &[0,0,1,0,0], &[1,1,1,1,1], &[0,0,1,0,0], &[0,0,1,0,0]];
    let sm: &[&[u8]] = &[&[0,1,0], &[1,1,1], &[0,1,0]];
    draw_block(img, s, px, py, scale);
    draw_block(img, sm, px + 7*scale, py + 2*scale, scale);
}

/// Draw the pet. x,y = top-left corner. scale = pixels per grid cell.
/// If flipped, the sprite is mirrored horizontally (walking left).
pub fn draw_pet(img: &mut GrayImage, state: PetState, x: i32, y: i32, scale: i32, flipped: bool) {
    let grid = match state {
        PetState::Happy => happy(),
        PetState::Sleeping => sleeping(),
        PetState::Rainy => rainy(),
        PetState::Hot => hot(),
        PetState::Cold => cold(),
        PetState::Stargazing => stargazing(),
    };
    draw_grid(img, &grid, x, y, scale, flipped);

    // Decoration offsets flip too
    let right = if flipped { x - 6 * scale } else { x + (SW as i32 + 1) * scale };
    let left = if flipped { x + (SW as i32 + 1) * scale } else { x - 5 * scale };
    match state {
        PetState::Sleeping => draw_zzz(img, right, y + 2*scale, scale),
        PetState::Rainy => {
            draw_drops(img, left, y + 3*scale, scale);
            draw_drops(img, right + scale, y + 5*scale, scale);
        }
        PetState::Hot => draw_sweat(img, right, y + 4*scale, scale),
        PetState::Stargazing => {
            draw_stars(img, left + 2*scale, y - 4*scale, scale);
            draw_stars(img, right, y - 3*scale, scale);
        }
        _ => {}
    }
}
