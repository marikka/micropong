#![no_main]
#![no_std]

extern crate panic_halt;
use cortex_m_rt::entry;

use crate::hal::{delay::Delay, prelude::*, stm32};
use hal::i2c::I2c;
use stm32f0xx_hal as hal;

use embedded_graphics::fonts::Font6x8;
use embedded_graphics::pixelcolor::PixelColorU8;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Rect};
use ssd1306::prelude::*;
use ssd1306::Builder;

use arrayvec::ArrayString;
use core::fmt::Write;

const PADDLE_THICKNESS: u8 = 2;
const PADDLE_WIDTH: u8 = 8;
const SCREEN_WIDTH: u8 = 128;
const SCREEN_HEIGHT: u8 = 32;

#[entry]
fn main() -> ! {
    config();
}

enum End {
    Left,
    Right,
}

struct Player {
    paddle_position: u8,
    end: End,
}

impl Player {
    fn new(end: End) -> Self {
        Player {
            paddle_position: 0,
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
        if self.y >= (SCREEN_HEIGHT - self.radius) || self.y < self.radius {
            self.vy = -self.vy;
        }

        let new_x = self.x as i8 + self.vx;
        let new_y = self.y as i8 + self.vy;

        self.x = if new_x > 0 { new_x as u8 } else { 0 };
        self.y = if new_y > 0 { new_y as u8 } else { 0 };
    }

    fn test_collision(&self, player: &Player) -> bool {
        (i8::abs(self.y as i8 - player.paddle_position as i8) as u8) <= (PADDLE_WIDTH / 2)
    }

    fn is_at_end(&self, end: End) -> bool {
        match end {
            End::Left => self.x < self.radius,
            End::Right => self.x >= (SCREEN_WIDTH - self.radius),
        }
    }

    fn bounce(&mut self) {
        self.vx = -self.vx;
    }

    fn drawable(&self) -> impl Iterator<Item = Pixel<PixelColorU8>> {
        Circle::new(Coord::new(self.x as i32, self.y as i32), self.radius as u32)
            .with_stroke(Some(1u8.into()))
            .into_iter()
    }
}

fn config() -> ! {
    let mut p = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();

    cortex_m::interrupt::free(move |cs| {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);

        let gpiob = p.GPIOB.split(&mut rcc);
        let scl = gpiob.pb6.into_alternate_af1(cs); //D5
        let sda = gpiob.pb7.into_alternate_af1(cs); //D4

        let p1_t1 = gpiob.pb0.into_pull_up_input(cs); //D3
        let p1_t2 = gpiob.pb1.into_pull_up_input(cs); //D6

        let gpioa = p.GPIOA.split(&mut rcc);
        let p2_t1 = gpioa.pa2.into_pull_up_input(cs); //A7
        let p2_t2 = gpioa.pa7.into_pull_up_input(cs); //A6

        let mut delay = Delay::new(cp.SYST, &rcc);

        let i2c = I2c::i2c1(p.I2C1, (scl, sda), 100.khz(), &mut rcc);

        let mut disp: GraphicsMode<_> = Builder::new()
            .with_size(DisplaySize::Display128x32)
            .connect_i2c(i2c)
            .into();
        disp.init().unwrap();
        disp.flush().unwrap();

        let mut player_1 = Player::new(End::Left);
        let mut player_2 = Player::new(End::Right);
        let (mut p1_score, mut p2_score) = (0, 0);

        loop {
            let mut ball = Ball::new();
            disp.clear();

            let mut score_str = ArrayString::<[u8; 20]>::new();
            write!(&mut score_str, "{} - {}", p1_score, p2_score).expect("Can't write");
            disp.draw(
                Font6x8::render_str(&score_str)
                    .with_stroke(Some(1u8.into()))
                    .translate(Coord::new(
                        SCREEN_WIDTH as i32 / 2,
                        SCREEN_HEIGHT as i32 / 2,
                    ))
                    .into_iter(),
            );

            disp.flush().unwrap();

            delay.delay_ms(2000u16);

            let (p1_points, p2_points) = loop {
                if ball.is_at_end(End::Left) {
                    if ball.test_collision(&player_1) {
                        ball.bounce();
                    } else {
                        break (0, 1);
                    }
                }
                if ball.is_at_end(End::Right) {
                    if ball.test_collision(&player_2) {
                        ball.bounce();
                    } else {
                        break (1, 0);
                    }
                }

                ball.update();
                disp.clear();

                match (p1_t1.is_low(), p1_t2.is_low()) {
                    (true, false) => {
                        player_1.move_paddle_left();
                    }
                    (false, true) => {
                        player_1.move_paddle_right();
                    }
                    _ => {}
                };

                match (p2_t1.is_low(), p2_t2.is_low()) {
                    (true, false) => {
                        player_2.move_paddle_left();
                    }
                    (false, true) => {
                        player_2.move_paddle_right();
                    }
                    _ => {}
                };

                disp.draw(ball.drawable());
                disp.draw(player_1.paddle_drawable());
                disp.draw(player_2.paddle_drawable());

                disp.flush().unwrap();
            };
            p1_score += p1_points;
            p2_score += p2_points;
        }
    })
}
