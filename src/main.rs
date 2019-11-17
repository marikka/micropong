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

use ssd1306::prelude::*;
use ssd1306::Builder;

const BOOT_DELAY_MS: u32 = 100;

mod tetris;

#[entry]
fn main() -> ! {
    let (mut delay, i2c, mut p1_t1, mut p1_t2, mut p2_t1, mut p2_t2, mut p2_t3, mut p2_t4) =
        config_hardware();

    let mut disp: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x32)
        .connect_i2c(i2c)
        .into();
    disp.init().unwrap();
    disp.flush().unwrap();

    tetris::tetris(
        &mut disp, &mut delay, &mut p1_t1, &mut p1_t2, &mut p2_t1, &mut p2_t2, &mut p2_t3,
        &mut p2_t4,
    );

    loop {}
}

fn config_hardware() -> (
    Delay,
    I2c<I2C1, PB6<Alternate<AF1>>, PB7<Alternate<AF1>>>,
    impl InputPin<Error = ()>,
    impl InputPin<Error = ()>,
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

        delay.delay_ms(BOOT_DELAY_MS);

        let gpiob = p.GPIOB.split(&mut rcc);
        let scl = gpiob.pb6.into_alternate_af1(cs); //D5
        let sda = gpiob.pb7.into_alternate_af1(cs); //D4

        let gpioa = p.GPIOA.split(&mut rcc);

        let t1 = gpioa.pa0.into_pull_up_input(cs);
        //let t2 = gpioa.pa1.into_pull_up_input(cs);
        let t3 = gpioa.pa2.into_pull_up_input(cs);
        let t4 = gpioa.pa5.into_pull_up_input(cs);

        let t5 = gpioa.pa8.into_pull_up_input(cs);
        //let t6 = gpioa.pa9.into_pull_up_input(cs);
        let t7 = gpioa.pa15.into_pull_up_input(cs);
        let t8 = gpiob.pb3.into_pull_up_input(cs);

        let p1_t1 = t3; // PA2
        let p1_t2 = t5; // PA8

        let p2_t1 = t4; // PA5
        let p2_t2 = t8; // PB3
        let p2_t3 = t1;
        let p2_t4 = t7;

        let i2c = I2c::i2c1(p.I2C1, (scl, sda), 400.khz(), &mut rcc);

        (delay, i2c, p1_t1, p1_t2, p2_t1, p2_t2, p2_t3, p2_t4)
    })
}
