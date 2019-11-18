extern crate panic_halt;

use crate::hal::{delay::Delay, prelude::*};
use embedded_hal::digital::v2::InputPin;
use embedded_hal::timer::CountDown;

use embedded_graphics::pixelcolor::PixelColorU8;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::Rect;
use ssd1306::prelude::*;

extern crate wyhash;
use wyhash::wyrng;

use stm32f0xx_hal::time::Hertz;

use nb::block;

//use cortex_m_semihosting::hprintln;

const SCREEN_WIDTH: u8 = 128;
const SCREEN_HEIGHT: u8 = 32;
const GRID_WIDTH: usize = 8;
const GRID_HEIGHT: usize = 32;
const BLOCK_SIZE: i32 = 4;
const MARGIN_X: i32 = (SCREEN_HEIGHT as i32 - BLOCK_SIZE * GRID_WIDTH as i32) / 2 + BLOCK_SIZE;
const NUM_TETROMINOES: usize = 7;

//
static I0: [[u8; 4]; 4] = [[0, 0, 0, 0], [1, 1, 1, 1], [0, 0, 0, 0], [0, 0, 0, 0]];
static I1: [[u8; 4]; 4] = [[0, 0, 1, 0], [0, 0, 1, 0], [0, 0, 1, 0], [0, 0, 1, 0]];
static I2: [[u8; 4]; 4] = [[0, 0, 0, 0], [0, 0, 0, 0], [1, 1, 1, 1], [0, 0, 0, 0]];
static I3: [[u8; 4]; 4] = [[0, 1, 0, 0], [0, 1, 0, 0], [0, 1, 0, 0], [0, 1, 0, 0]];
static I: [[[u8; 4]; 4]; 4] = [I0, I1, I2, I3];

static J0: [[u8; 3]; 3] = [[1, 0, 0], [1, 1, 1], [0, 0, 0]];
static J1: [[u8; 3]; 3] = [[0, 1, 1], [0, 1, 0], [0, 1, 0]];
static J2: [[u8; 3]; 3] = [[0, 0, 0], [1, 1, 1], [0, 0, 1]];
static J3: [[u8; 3]; 3] = [[0, 1, 0], [0, 1, 0], [1, 1, 0]];
static J: [[[u8; 3]; 3]; 4] = [J0, J1, J2, J3];

static L0: [[u8; 3]; 3] = [[0, 0, 1], [1, 1, 1], [0, 0, 0]];
static L1: [[u8; 3]; 3] = [[0, 1, 0], [0, 1, 0], [0, 1, 1]];
static L2: [[u8; 3]; 3] = [[0, 0, 0], [1, 1, 1], [1, 0, 0]];
static L3: [[u8; 3]; 3] = [[1, 1, 0], [0, 1, 0], [0, 1, 0]];
static L: [[[u8; 3]; 3]; 4] = [L0, L1, L2, L3];

static O0: [[u8; 4]; 3] = [[0, 1, 1, 0], [0, 1, 1, 0], [0, 0, 0, 0]];
static O1: [[u8; 4]; 3] = [[0, 1, 1, 0], [0, 1, 1, 0], [0, 0, 0, 0]];
static O2: [[u8; 4]; 3] = [[0, 1, 1, 0], [0, 1, 1, 0], [0, 0, 0, 0]];
static O3: [[u8; 4]; 3] = [[0, 1, 1, 0], [0, 1, 1, 0], [0, 0, 0, 0]];
static O: [[[u8; 4]; 3]; 4] = [O0, O1, O2, O3];

static S0: [[u8; 3]; 3] = [[0, 1, 1], [1, 1, 0], [0, 0, 0]];
static S1: [[u8; 3]; 3] = [[0, 1, 0], [0, 1, 1], [0, 0, 1]];
static S2: [[u8; 3]; 3] = [[0, 0, 0], [0, 1, 1], [1, 1, 0]];
static S3: [[u8; 3]; 3] = [[1, 0, 0], [1, 1, 0], [0, 1, 0]];
static S: [[[u8; 3]; 3]; 4] = [S0, S1, S2, S3];

static T0: [[u8; 3]; 3] = [[0, 1, 0], [1, 1, 1], [0, 0, 0]];
static T1: [[u8; 3]; 3] = [[0, 1, 0], [0, 1, 1], [0, 1, 0]];
static T2: [[u8; 3]; 3] = [[0, 0, 0], [1, 1, 1], [0, 1, 0]];
static T3: [[u8; 3]; 3] = [[0, 1, 0], [1, 1, 0], [0, 1, 0]];
static T: [[[u8; 3]; 3]; 4] = [T0, T1, T2, T3];

static Z0: [[u8; 3]; 3] = [[1, 1, 0], [0, 1, 1], [0, 0, 0]];
static Z1: [[u8; 3]; 3] = [[0, 0, 1], [0, 1, 1], [0, 1, 0]];
static Z2: [[u8; 3]; 3] = [[0, 0, 0], [1, 1, 0], [0, 1, 1]];
static Z3: [[u8; 3]; 3] = [[0, 1, 0], [1, 1, 0], [1, 0, 0]];
static Z: [[[u8; 3]; 3]; 4] = [Z0, Z1, Z2, Z3];

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

#[derive(Clone, Copy)]
struct Tetromino {
    x: i32,
    y: i32,
    typ: TetrominoType,
    rotation: usize,
}

impl Tetromino {
    fn can_move_right(&mut self, grid: &Grid) -> bool {
        !test_collision(self.x + 1, self.y, self, grid)
    }

    fn can_move_left(&mut self, grid: &Grid) -> bool {
        !test_collision(self.x - 1, self.y, self, grid)
    }

    fn can_move_down(&mut self, grid: &Grid) -> bool {
        !test_collision(self.x, self.y + 1, self, grid)
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

    fn rotate_left(&mut self) {
        self.rotation = if self.rotation == 0 {
            3
        } else {
            self.rotation - 1
        }
    }

    fn rotate_right(&mut self) {
        self.rotation = if self.rotation == 3 {
            0
        } else {
            self.rotation + 1
        }
    }

    fn get_data(&self, y: usize, x: usize) -> u8 {
        if y > 3 || x > 3 {
            return 0;
        }

        match self.typ {
            TetrominoType::I => I[self.rotation][y][x],
            TetrominoType::J => J[self.rotation][y][x],
            TetrominoType::L => L[self.rotation][y][x],
            TetrominoType::O => O[self.rotation][y][x],
            TetrominoType::S => S[self.rotation][y][x],
            TetrominoType::T => T[self.rotation][y][x],
            TetrominoType::Z => Z[self.rotation][y][x],
        }
    }

    fn h(&self) -> usize {
        match self.typ {
            TetrominoType::I => 4,
            TetrominoType::J => 3,
            TetrominoType::L => 3,
            TetrominoType::O => 3,
            TetrominoType::S => 3,
            TetrominoType::T => 3,
            TetrominoType::Z => 3,
        }
    }

    fn w(&self) -> usize {
        match self.typ {
            TetrominoType::I => 4,
            TetrominoType::J => 3,
            TetrominoType::L => 3,
            TetrominoType::O => 4,
            TetrominoType::S => 3,
            TetrominoType::T => 3,
            TetrominoType::Z => 3,
        }
    }
}

pub fn tetris<
    E: core::fmt::Debug,
    GM: ssd1306::interface::DisplayInterface<Error = E>,
    T: CountDown<Time = Hertz>,
>(
    disp: &mut GraphicsMode<GM>,
    delay: &mut Delay,
    timer: &mut T,
    p1_t1: &mut impl InputPin<Error = ()>,
    p1_t2: &mut impl InputPin<Error = ()>,
    p2_t1: &mut impl InputPin<Error = ()>,
    p2_t2: &mut impl InputPin<Error = ()>,
    p2_t3: &mut impl InputPin<Error = ()>,
    p2_t4: &mut impl InputPin<Error = ()>,
) {
    let mut seed = 3;
    timer.start(Hertz(6));
    'game: loop {
        let mut grid: Grid = [[0; GRID_WIDTH]; GRID_HEIGHT];

        'tetromino: loop {
            if let (Ok(true), Ok(true)) = (p1_t1.is_low(), p1_t2.is_low()) {
                break;
            }

            let typ = TetrominoType::from((wyrng(&mut seed) % NUM_TETROMINOES as u64) as i32);
            let mut current_tetromino = Tetromino {
                x: 4,
                y: 0,
                typ,
                rotation: 0,
            };

            'row: loop {
                match (p2_t1.is_low(), p2_t2.is_low()) {
                    (_, Ok(true)) => current_tetromino.move_left(&grid),
                    (Ok(true), _) => current_tetromino.move_right(&grid),
                    _ => {}
                }

                match (p2_t3.is_low(), p2_t4.is_low()) {
                    (_, Ok(true)) => current_tetromino.rotate_left(),
                    (Ok(true), _) => current_tetromino.rotate_right(),
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

                disp.draw(tetromino_drawable(&current_tetromino));

                disp.flush().unwrap();

                block!(timer.wait());

                if current_tetromino.y >= GRID_HEIGHT as i32 - 4
                    || !current_tetromino.can_move_down(&grid)
                {
                    if current_tetromino.y > 5 {
                        draw_tetromino_on_grid(&current_tetromino, &mut grid);
                        break;
                    } else {
                        continue 'game;
                    }
                }

                for y in 0..grid.len() {
                    if grid[y].iter().all(|&x| x > 0) {
                        //Erase row
                        for x in grid[y].iter_mut() {
                            *x = 0;
                        }

                        //Move each above erased row down
                        for yy in (0..y).rev() {
                            for x in 0..grid[yy].len() {
                                grid[yy + 1][x] = grid[yy][x];
                            }
                        }
                    }
                }

                current_tetromino.y += 1;
            }
        }
    }
}

fn tetromino_drawable(t: &Tetromino) -> impl Iterator<Item = Pixel<PixelColorU8>> {
    let tc = t.clone();
    let w = tc.w();
    let h = tc.h();
    (0..h)
        .into_iter()
        .flat_map(move |e| core::iter::repeat(e).zip((0..w).into_iter()))
        .filter_map(move |(y, x)| {
            if tc.get_data(y, x) > 0 {
                Some(block_drawable(tc.y + y as i32, tc.x + x as i32))
            } else {
                None
            }
        })
        .flat_map(|e| e)
}

fn draw_tetromino_on_grid(t: &Tetromino, g: &mut Grid) {
    for y in 0..t.h() {
        for x in 0..t.w() {
            let yi = t.y + y as i32;
            let xi = t.x + x as i32;
            if yi > 0 && yi < GRID_HEIGHT as i32 && xi >= 0 && xi < GRID_WIDTH as i32 {
                g[yi as usize][xi as usize] |= t.get_data(y as usize, x as usize) as usize;
            }
        }
    }
}

fn block_drawable(y: i32, x: i32) -> impl Iterator<Item = Pixel<PixelColorU8>> {
    let x_display = y * BLOCK_SIZE;
    let y_display = SCREEN_HEIGHT as i32 - (x as i32 * BLOCK_SIZE) - MARGIN_X;
    Rect::new(
        Coord::new(x_display, y_display),
        Coord::new(x_display + BLOCK_SIZE - 1, y_display + BLOCK_SIZE - 1),
    )
    .with_stroke(Some(1u8.into()))
    .into_iter()
}

fn test_collision(x0: i32, y0: i32, t: &Tetromino, g: &Grid) -> bool {
    for y in 0..t.h() as usize {
        for x in 0..t.w() as usize {
            let yi = y0 + y as i32;
            let xi = x0 + x as i32;

            if t.get_data(y, x) == 1 {
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
