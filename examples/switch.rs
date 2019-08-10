#![no_main]
#![no_std]

extern crate panic_halt;
use cortex_m_rt::entry;

use crate::hal::{delay::Delay, prelude::*, stm32};
use embedded_hal::digital::v2::{OutputPin, InputPin};
use stm32f0xx_hal as hal;

#[entry]
fn main() -> ! {
    let (mut led, mut switch) = config();
    loop {
        if switch.is_high().unwrap() {led.set_high().unwrap() } else {led.set_low().unwrap()}
    }
}

//Configure MCU, and return *abstract* pin trait. The real type is PB3<Output<PushPull>>
fn config() -> (impl OutputPin<Error = ()>, impl InputPin<Error = ()>) {
    let mut p = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();

    cortex_m::interrupt::free(move |cs| {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);
        let gpiob = p.GPIOB.split(&mut rcc);
        let led = gpiob.pb3.into_push_pull_output(cs);
        let switch = gpiob.pb4.into_pull_up_input(cs); //D12

        (led, switch)
    })
}
