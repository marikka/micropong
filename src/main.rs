#![no_main]
#![no_std]

extern crate panic_halt;
use cortex_m_rt::entry;

use crate::hal::{delay::Delay, prelude::*, stm32};
use hal::i2c::I2c;
use stm32f0xx_hal as hal;

use embedded_graphics::pixelcolor::PixelColorU8;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Rect};
use ssd1306::prelude::*;
use ssd1306::Builder;

const PADDLE_THICKNESS: u8 = 2;
const PADDLE_WIDTH: u8 = 8;
const SCREEN_WIDTH: u8 = 128;
const SCREEN_HEIGHT: u8 = 32;

#[entry]
fn main() -> ! {
    let (mut delay) = config();
    loop {
        let delay_ms = 100;
    }
}

enum End {
    Left,
    Right,
}

struct Player {
    paddle_position: u8,
    score: u32,
    end: End,
}

impl Player {
    fn new(end: End) -> Self {
        Player {
            paddle_position: 0,
            score: 0,
            end: end,
        }
    }

    pub fn move_paddle_left(&mut self) {
        let new_position = self.paddle_position as i8 - 1;
        self.paddle_position = if new_position < 0 {
            0
        } else {
            new_position as u8
        };
    }

    pub fn move_paddle_right(&mut self) {
        let new_position = self.paddle_position as i8 + 1;
        let limit = SCREEN_HEIGHT - PADDLE_WIDTH - 1;
        self.paddle_position = if new_position > limit as i8 {
            limit
        } else {
            new_position as u8
        };
    }

    pub fn paddle_drawable(&self) -> impl Iterator<Item = Pixel<PixelColorU8>> {
        let x = match self.end {
            End::Left => 0,
            End::Right => SCREEN_WIDTH - PADDLE_THICKNESS,
        };
        Rect::new(
            Coord::new(x as i32, self.paddle_position as i32),
            Coord::new(
                (x + PADDLE_THICKNESS) as i32,
                (self.paddle_position + PADDLE_WIDTH) as i32,
            ),
        )
        .with_fill(Some(1u8.into()))
        .into_iter()
    }
}

struct Ball {
    radius: u8,
    x: u8,
    y: u8,
    vx: i8,
    vy: i8,
}

impl Ball {
    fn new() -> Self {
        Ball {
            radius: 3,
            x: SCREEN_WIDTH / 2,
            y: SCREEN_HEIGHT / 2,
            vx: 2,
            vy: 1,
        }
    }

    fn update(&mut self) {
        if self.x >= (SCREEN_WIDTH - self.radius) || self.x < self.radius {
            self.vx = -self.vx;
        }

        if self.y >= (SCREEN_HEIGHT - self.radius) || self.y < self.radius {
            self.vy = -self.vy;
        }

        let new_x = self.x as i8 + self.vx;
        let new_y = self.y as i8 + self.vy;

        self.x = if new_x > 0 { new_x as u8 } else { 0 };
        self.y = if new_y > 0 { new_y as u8 } else { 0 };
    }

    fn drawable(&self) -> impl Iterator<Item = Pixel<PixelColorU8>> {
        Circle::new(Coord::new(self.x as i32, self.y as i32), self.radius as u32)
            .with_stroke(Some(1u8.into()))
            .into_iter()
    }
}

fn config() -> (Delay) {
    let mut p = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();

    cortex_m::interrupt::free(move |cs| {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);

        let gpiob = p.GPIOB.split(&mut rcc);
        let scl = gpiob.pb6.into_alternate_af1(cs); //D5
        let sda = gpiob.pb7.into_alternate_af1(cs); //D4

        let gpioa = p.GPIOA.split(&mut rcc);
        let p1_t1 = gpioa.pa2.into_pull_up_input(cs);
        let p1_t2 = gpioa.pa7.into_pull_up_input(cs);
        let delay = Delay::new(cp.SYST, &rcc);

        let i2c = I2c::i2c1(p.I2C1, (scl, sda), 100.khz(), &mut rcc);

        let mut disp: GraphicsMode<_> = Builder::new()
            .with_size(DisplaySize::Display128x32)
            .connect_i2c(i2c)
            .into();
        disp.init().unwrap();
        disp.flush().unwrap();

        let mut player_1 = Player::new(End::Left);
        let mut player_2 = Player::new(End::Right);
        let mut ball = Ball::new();

        loop {
            ball.update();

            disp.clear();

            disp.draw(ball.drawable());

            match (p1_t1.is_low(), p1_t2.is_low()) {
                (true, false) => {
                    player_1.move_paddle_left();
                    player_2.move_paddle_left();
                }
                (false, true) => {
                    player_1.move_paddle_right();
                    player_2.move_paddle_right();
                }
                _ => {}
            };

            disp.draw(player_1.paddle_drawable());
            disp.draw(player_2.paddle_drawable());

            disp.flush().unwrap();
        }

        disp.flush().unwrap();

        (delay)
    })
}
