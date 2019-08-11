#![no_main]
#![no_std]

extern crate panic_halt;
use cortex_m_rt::entry;

use crate::hal::{delay::Delay, prelude::*, stm32};
use embedded_hal::digital::v2::OutputPin;
use stm32f0xx_hal as hal;
use vl53l0x::VL53L0x;
use hal::i2c::I2c;
use hal::stm32f0::stm32f0x2::I2C1;
use hal::gpio::gpiob::{PB6, PB7};
use hal::gpio::{Alternate, AF1};
const TOF_TIMING_BUDGET_MS: u32 = 33000;

#[entry]
fn main() -> ! {
    let (mut led, mut tof, mut delay) = config();
    loop {
        let distance = tof.read_range_continuous_millimeters_blocking().unwrap();
        let delay_ms = distance / 2;

        led.set_low().unwrap();
        delay.delay_ms(delay_ms);
        led.set_high().unwrap();
        delay.delay_ms(delay_ms);
    }
}

fn config() -> (impl OutputPin<Error = ()>, VL53L0x<I2c<I2C1, PB6<Alternate<AF1>>, PB7<Alternate<AF1>>>>, Delay) {
    let mut p = stm32::Peripherals::take().unwrap();
    let cp = cortex_m::peripheral::Peripherals::take().unwrap();

    cortex_m::interrupt::free(move |cs| {
        let mut rcc = p.RCC.configure().sysclk(8.mhz()).freeze(&mut p.FLASH);
        let gpiob = p.GPIOB.split(&mut rcc);
        let led = gpiob.pb3.into_push_pull_output(cs);
        let scl = gpiob.pb6.into_alternate_af1(cs); //D5
        let sda = gpiob.pb7.into_alternate_af1(cs); //D4
        let delay = Delay::new(cp.SYST, &rcc);

        let mut i2c = I2c::i2c1(p.I2C1, (scl, sda), 100.khz(), &mut rcc);
        let mut tof = vl53l0x::VL53L0x::new(i2c).unwrap();

        tof.set_measurement_timing_budget(TOF_TIMING_BUDGET_MS)
            .unwrap();
        tof.start_continuous(0).unwrap();

        (led, tof, delay)
    })
}
