extern crate panic_halt;

use crate::hal::{delay::Delay, prelude::*};
use embedded_hal::digital::v2::InputPin;

use embedded_graphics::pixelcolor::PixelColorU8;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rect;
use ssd1306::prelude::*;

extern crate wyhash;
use wyhash::wyrng;

//use cortex_m_semihosting::hprintln;

const SCREEN_WIDTH: u8 = 128;
const SCREEN_HEIGHT: u8 = 32;
const GRID_WIDTH: usize = 10;
const GRID_HEIGHT: usize = 40;
const BLOCK_SIZE: i32 = 3;
const MARGIN_X: i32 = (SCREEN_HEIGHT as i32 - BLOCK_SIZE * GRID_WIDTH as i32) / 2 + BLOCK_SIZE;
const NUM_TETROMINOES: usize = 7;

static I: TetrominoData = [[1, 0], [1, 0], [1, 0], [1, 0]];
static T: TetrominoData = [[1, 0], [1, 1], [1, 0], [0, 0]];
static S: TetrominoData = [[1, 0], [1, 1], [0, 1], [0, 0]];
static Z: TetrominoData = [[0, 1], [1, 1], [1, 0], [0, 0]];
static O: TetrominoData = [[1, 1], [1, 1], [0, 0], [0, 0]];
static L: TetrominoData = [[1, 0], [1, 0], [1, 1], [0, 0]];
static J: TetrominoData = [[0, 1], [0, 1], [1, 1], [0, 0]];

static TETROMINOES: [TetrominoData; NUM_TETROMINOES] = [I, T, S, Z, O, L, J];

#[derive(Clone, Copy)]
enum TetrominoType {
    I = 0,
    T = 1,
    S = 2,
    Z = 3,
    O = 4,
    L = 5,
    J = 6,
}

impl From<i32> for TetrominoType {
    fn from(x: i32) -> Self {
        match x {
            0 => TetrominoType::I,
            1 => TetrominoType::T,
            2 => TetrominoType::S,
            3 => TetrominoType::Z,
            4 => TetrominoType::O,
            5 => TetrominoType::L,
            6 => TetrominoType::J,
            _ => TetrominoType::I,
        }
    }
}

type Grid = [[usize; GRID_WIDTH]; GRID_HEIGHT];

type TetrominoData = [[usize; 2]; 4];

struct Tetromino {
    x: i32,
    y: i32,
    typ: TetrominoType,
}

impl Tetromino {
    fn can_move_right(&mut self, grid: &Grid) -> bool {
        !test_collision(self.x + 1, self.y, self.typ, grid)
    }

    fn can_move_left(&mut self, grid: &Grid) -> bool {
        !test_collision(self.x - 1, self.y, self.typ, grid)
    }

    fn can_move_down(&mut self, grid: &Grid) -> bool {
        !test_collision(self.x, self.y + 1, self.typ, grid)
    }

    fn move_left(&mut self, grid: &Grid) {
        if self.can_move_left(grid) {
            self.x -= 1;
        }
    }

    fn move_right(&mut self, grid: &Grid) {
        if self.can_move_right(grid) {
            self.x += 1;
        }
    }
}

pub fn tetris<E: core::fmt::Debug, GM: ssd1306::interface::DisplayInterface<Error = E>>(
    disp: &mut GraphicsMode<GM>,
    delay: &mut Delay,
    p1_t1: &mut impl InputPin<Error = ()>,
    p1_t2: &mut impl InputPin<Error = ()>,
    p2_t1: &mut impl InputPin<Error = ()>,
    p2_t2: &mut impl InputPin<Error = ()>,
) {
    let mut grid: Grid = [[0; GRID_WIDTH]; GRID_HEIGHT];
    let mut seed = 3;

    loop {
        if let (Ok(true), Ok(true)) = (p1_t1.is_low(), p1_t2.is_low()) {
            break;
        }

        let typ = TetrominoType::from((wyrng(&mut seed) % NUM_TETROMINOES as u64) as i32);
        let mut current_tetromino = Tetromino { x: 5, y: 0, typ };

        loop {
            match (p2_t1.is_low(), p2_t2.is_low()) {
                (_, Ok(true)) => current_tetromino.move_left(&grid),
                (Ok(true), _) => current_tetromino.move_right(&grid),
                _ => {}
            }

            disp.clear();

            for y in 0..GRID_HEIGHT {
                for x in 0..GRID_WIDTH {
                    if grid[y][x] > 0 {
                        disp.draw(block_drawable(y as i32, x as i32));
                    }
                }
            }

            disp.draw(tetromino_drawable(
                current_tetromino.x,
                current_tetromino.y,
                current_tetromino.typ,
            ));

            disp.flush().unwrap();

            delay.delay_ms(50u16);

            if current_tetromino.y >= GRID_HEIGHT as i32 - 4
                || !current_tetromino.can_move_down(&grid)
            {
                draw_tetromino_on_grid(
                    current_tetromino.x,
                    current_tetromino.y,
                    current_tetromino.typ,
                    &mut grid,
                );
                break;
            }
            current_tetromino.y += 1;
        }
    }
}

fn tetromino_drawable(
    x0: i32,
    y0: i32,
    t_type: TetrominoType,
) -> impl Iterator<Item = Pixel<PixelColorU8>> {
    let t = TETROMINOES[t_type as usize];

    (0..4)
        .into_iter()
        .flat_map(|e| core::iter::repeat(e).zip((0..2).into_iter()))
        .filter_map(move |(y, x)| {
            if t[y][x] > 0 {
                Some(block_drawable(y0 + y as i32, x0 + x as i32))
            } else {
                None
            }
        })
        .flat_map(|e| e)
}

fn draw_tetromino_on_grid(x0: i32, y0: i32, t_type: TetrominoType, g: &mut Grid) {
    let t = TETROMINOES[t_type as usize];
    for y in 0..4 {
        for x in 0..2 {
            let yi = y0 + y;
            let xi = x0 + x;
            if yi > 0 && yi < GRID_HEIGHT as i32 && xi >= 0 && xi < GRID_WIDTH as i32 {
                g[yi as usize][xi as usize] |= t[y as usize][x as usize];
            }
        }
    }
}

fn block_drawable(y: i32, x: i32) -> impl Iterator<Item = Pixel<PixelColorU8>> {
    let x_display = y * BLOCK_SIZE;
    let y_display = SCREEN_HEIGHT as i32 - (x as i32 * BLOCK_SIZE) - MARGIN_X;
    Rect::new(
        Coord::new(x_display, y_display),
        Coord::new(x_display + BLOCK_SIZE, y_display + BLOCK_SIZE),
    )
    .with_stroke(Some(1u8.into()))
    .into_iter()
}

fn test_collision(x0: i32, y0: i32, t_type: TetrominoType, g: &Grid) -> bool {
    let t = TETROMINOES[t_type as usize];
    for y in 0..4 as usize {
        for x in 0..2 as usize {
            let yi = y0 + y as i32;
            let xi = x0 + x as i32;

            if t[y][x] == 1 {
                if yi > 0 && yi < GRID_HEIGHT as i32 && xi >= 0 && xi < GRID_WIDTH as i32 {
                    if g[yi as usize][xi as usize] == 1 {
                        return true;
                    }
                } else {
                    return true;
                }
            }
        }
    }
    false
}
