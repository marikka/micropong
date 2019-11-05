extern crate panic_halt;

use crate::hal::{delay::Delay, prelude::*};
use embedded_hal::digital::v2::InputPin;

use embedded_graphics::pixelcolor::PixelColorU8;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Rect};
use ssd1306::prelude::*;

extern crate wyhash;
use wyhash::wyrng;

use arrayvec::ArrayString;
use core::fmt::Write;
const SCREEN_WIDTH: u8 = 128;
const SCREEN_HEIGHT: u8 = 32;
const SCORE_SCREEN_DELAY_MS: u16 = 2000;
const GRID_WIDTH: usize = 10;
const GRID_HEIGHT: usize = 40;
const BLOCK_SIZE: i32 = 3;
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

impl From<usize> for TetrominoType {
    fn from(x: usize) -> Self {
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
    x: usize,
    y: usize,
    typ: TetrominoType,
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

        let typ = TetrominoType::from((wyrng(&mut seed) % NUM_TETROMINOES as u64) as usize);
        let mut current_tetromino = Tetromino { x: 5, y: 0, typ };

        loop {
            match (
                p2_t1.is_low(),
                p2_t2.is_low(),
                current_tetromino.x > 0,
                current_tetromino.x < GRID_WIDTH,
            ) {
                (Ok(true), _, true, _) => current_tetromino.x -= 1,
                (_, Ok(true), _, true) => current_tetromino.x += 1,
                _ => {}
            }

            disp.clear();

            disp.draw(tetromino_drawable(
                current_tetromino.x,
                current_tetromino.y,
                current_tetromino.typ,
            ));

            disp.flush().unwrap();

            delay.delay_ms(50u16);

            current_tetromino.y += 1;
            if current_tetromino.y >= GRID_HEIGHT {
                break;
            }
        }
    }
}

fn tetromino_drawable(
    x0: usize,
    y0: usize,
    t_type: TetrominoType,
) -> impl Iterator<Item = Pixel<PixelColorU8>> {
    let t = TETROMINOES[t_type as usize];

    (0..4)
        .into_iter()
        .flat_map(|e| core::iter::repeat(e).zip((0..2).into_iter()))
        .filter_map(move |(y, x)| {
            if t[y][x] > 0 {
                Some(block_drawable(y0 + y, x0 + x))
            } else {
                None
            }
        })
        .flat_map(|e| e)
}

fn block_drawable(y: usize, x: usize) -> impl Iterator<Item = Pixel<PixelColorU8>> {
    let x_display = y as i32 * BLOCK_SIZE;
    let y_display = x as i32 * BLOCK_SIZE;
    Rect::new(
        Coord::new(x_display, y_display),
        Coord::new(x_display + BLOCK_SIZE, y_display + BLOCK_SIZE),
    )
    .with_stroke(Some(1u8.into()))
    .into_iter()
}
