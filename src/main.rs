#![no_main]
#![no_std]

extern crate panic_halt;
use cortex_m_rt::entry;

use crate::hal::{delay::Delay, prelude::*, stm32};
use embedded_hal::digital::v2::InputPin;
use hal::gpio::gpiob::{PB6, PB7};
use hal::gpio::{Alternate, AF1};
use hal::i2c::I2c;
use hal::stm32f0::stm32f0x2::I2C1;
use stm32f0xx_hal as hal;

use embedded_graphics::fonts::Font6x12;
use embedded_graphics::pixelcolor::PixelColorU8;
use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{Circle, Rect};
use ssd1306::prelude::*;
use ssd1306::Builder;

use arrayvec::ArrayString;
use core::fmt::Write;
use libm::F32Ext;

const PADDLE_THICKNESS: u8 = 2;
const PADDLE_WIDTH: u8 = 8;
const SCREEN_WIDTH: u8 = 128;
const SCREEN_HEIGHT: u8 = 32;
const SCORE_SCREEN_DELAY_MS: u16 = 2000;

#[entry]
fn main() -> ! {
    let (mut delay, i2c, p1_t1, p1_t2, p2_t1, p2_t2) = config_hardware();

    let mut disp: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x32)
        .connect_i2c(i2c)
        .into();
    disp.init().unwrap();
    disp.flush().unwrap();

    let mut player_1 = Player::new(End::Left);
    let mut player_2 = Player::new(End::Right);
    let (mut p1_score, mut p2_score) = (0, 0);
    let mut last_winner_id = 1;

    loop {
        let vx = match last_winner_id {
            1 => 2.0,
            _ => -2.0,
        };

        let mut ball = Ball::new(vx);
        disp.clear();

        let mut score_str = ArrayString::<[u8; 20]>::new();
        write!(&mut score_str, "{} - {}", p1_score, p2_score).expect("Can't write");
        disp.draw(
            Font6x12::render_str(&score_str)
                .with_stroke(Some(1u8.into()))
                .translate(Coord::new(
                    (SCREEN_WIDTH as i32 / 2) - 3 * 8,
                    (SCREEN_HEIGHT as i32 / 2) - (16 / 2),
                ))
                .into_iter(),
        );

        disp.flush().unwrap();

        delay.delay_ms(SCORE_SCREEN_DELAY_MS);

        last_winner_id = loop {
            if ball.is_at_paddle(End::Left) {
                let collision_position = ball.test_collision(&player_1);
                if collision_position.abs() <= 1.0 {
                    ball.bounce(collision_position);
                }
            }
            if ball.is_at_paddle(End::Right) {
                let collision_position = ball.test_collision(&player_2);
                if collision_position.abs() <= 1.0 {
                    ball.bounce(collision_position);
                }
            }
            if ball.is_at_end(End::Left) {
                break 2;
            }
            if ball.is_at_end(End::Right) {
                break 1;
            }

            ball.update();
            disp.clear();

            match (p1_t1.is_low(), p1_t2.is_low()) {
                (Ok(true), Ok(false)) => {
                    player_1.move_paddle_left();
                }
                (Ok(false), Ok(true)) => {
                    player_1.move_paddle_right();
                }
                _ => {}
            };

            match (p2_t1.is_low(), p2_t2.is_low()) {
                (Ok(true), Ok(false)) => {
                    player_2.move_paddle_left();
                }
                (Ok(false), Ok(true)) => {
                    player_2.move_paddle_right();
                }
                _ => {}
            };

            disp.draw(ball.drawable());
            disp.draw(player_1.paddle_drawable());
            disp.draw(player_2.paddle_drawable());

            disp.flush().unwrap();
        };
        match last_winner_id {
            1 => p1_score += 1,
            2 => p2_score += 1,
            _ => {}
        };
    }
}

enum End {
    Left,
    Right,
}

struct Player {
    paddle_position: f32,
    end: End,
}

impl Player {
    fn new(end: End) -> Self {
        Player {
            paddle_position: 0.0,
            end: end,
        }
    }

    pub fn move_paddle_left(&mut self) {
        let new_position = self.paddle_position - 1.0;
        self.paddle_position = if new_position < 0.0 {
            0.0
        } else {
            new_position
        };
    }

    pub fn move_paddle_right(&mut self) {
        let new_position = self.paddle_position + 1.0;
        let limit = SCREEN_HEIGHT as f32 - PADDLE_WIDTH as f32 - 1.0;
        self.paddle_position = if new_position > limit {
            limit
        } else {
            new_position
        };
    }

    pub fn paddle_drawable(&self) -> impl Iterator<Item = Pixel<PixelColorU8>> {
        let x = match self.end {
            End::Left => 0.0,
            End::Right => SCREEN_WIDTH as f32 - PADDLE_THICKNESS as f32,
        };
        Rect::new(
            Coord::new(x as i32, self.paddle_position as i32),
            Coord::new(
                (x + PADDLE_THICKNESS as f32) as i32,
                self.paddle_position.round() as i32 + PADDLE_WIDTH as i32,
            ),
        )
        .with_fill(Some(1u8.into()))
        .into_iter()
    }
}

struct Ball {
    radius: f32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
}

impl Ball {
    fn new(vx: f32) -> Self {
        Ball {
            radius: 3.0,
            x: SCREEN_WIDTH as f32 / 2.0,
            y: SCREEN_HEIGHT as f32 / 2.0,
            vx: vx,
            vy: 0.1,
        }
    }

    fn update(&mut self) {
        if self.y >= (SCREEN_HEIGHT as f32 - self.radius) || self.y < self.radius {
            self.vy = -self.vy;
        }

        let new_x = self.x + self.vx;
        let new_y = self.y + self.vy;

        self.x = if new_x > 0.0 { new_x } else { 0.0 };
        self.y = if new_y > 0.0 { new_y } else { 0.0 };
    }

    fn test_collision(&self, player: &Player) -> f32 {
        let hit_area = PADDLE_WIDTH as f32 / 2.0 + self.radius / 2.0;
        let paddle_center = player.paddle_position + hit_area;
        let diff = paddle_center - self.y;
        (diff / hit_area)
    }

    fn is_at_end(&self, end: End) -> bool {
        match end {
            End::Left => self.x < self.radius,
            End::Right => self.x >= (SCREEN_WIDTH as f32 - self.radius),
        }
    }

    fn is_at_paddle(&self, end: End) -> bool {
        match end {
            End::Left => self.x < self.radius + PADDLE_THICKNESS as f32,
            End::Right => self.x >= (SCREEN_WIDTH as f32 - self.radius - PADDLE_THICKNESS as f32),
        }
    }

    fn bounce(&mut self, collision_position: f32) {
        self.vx = -self.vx * 1.01;
        self.vy = -collision_position;
    }

    fn drawable(&self) -> impl Iterator<Item = Pixel<PixelColorU8>> {
        Circle::new(
            Coord::new(self.x.round() as i32, self.y.round() as i32),
            self.radius.round() as u32,
        )
        .with_stroke(Some(1u8.into()))
        .into_iter()
    }
}

fn config_hardware() -> (
    Delay,
    I2c<I2C1, PB6<Alternate<AF1>>, PB7<Alternate<AF1>>>,
    impl InputPin<Error = ()>,
    impl InputPin<Error = ()>,
    impl InputPin<Error = ()>,
    impl InputPin<Error = ()>,
) {
    let mut p = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();

    cortex_m::interrupt::free(move |cs| {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);
        let mut delay = Delay::new(cp.SYST, &rcc);

        delay.delay_ms(100u32);

        let gpiob = p.GPIOB.split(&mut rcc);
        let scl = gpiob.pb6.into_alternate_af1(cs); //D5
        let sda = gpiob.pb7.into_alternate_af1(cs); //D4

        let gpioa = p.GPIOA.split(&mut rcc);

        //let t1 = gpioa.pa0.into_pull_up_input(cs);
        //let t2 = gpioa.pa1.into_pull_up_input(cs);
        let t3 = gpioa.pa2.into_pull_up_input(cs);
        let t4 = gpioa.pa5.into_pull_up_input(cs);

        let t5 = gpioa.pa8.into_pull_up_input(cs);
        //let t6 = gpioa.pa9.into_pull_up_input(cs);
        //let t7 = gpioa.pa15.into_pull_up_input(cs);
        let t8 = gpiob.pb3.into_pull_up_input(cs);

        let p1_t1 = t3; // OK PA2
        let p1_t2 = t5; // OK PA8

        let p2_t1 = t4; //OK PA5
        let p2_t2 = t8; //OK PB3

        let i2c = I2c::i2c1(p.I2C1, (scl, sda), 400.khz(), &mut rcc);

        (delay, i2c, p1_t1, p1_t2, p2_t1, p2_t2)
    })
}
