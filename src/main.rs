// //! CDC-ACM serial port example using cortex-m-rtic.
// //! Target board: Blue Pill
#![feature(type_alias_impl_trait)]
#![no_main]
#![no_std]
#![allow(non_snake_case)]

#[rtic::app(device = stm32f1xx_hal::pac)]
mod app {
    use defmt::info;
    use defmt_rtt as _;
    use panic_probe as _;

    use rtic_monotonics::systick::Systick;
    use stm32f1xx_hal::prelude::*;

    #[shared]
    struct Shared {}

    #[local]
    struct Local {
        led: stm32f1xx_hal::gpio::Pin<'C', 13, stm32f1xx_hal::gpio::Output>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local) {
        info!("start");
        let mut gpioc = ctx.device.GPIOC.split();

        let led = gpioc
            .pc13
            .into_push_pull_output_with_state(&mut gpioc.crh, stm32f1xx_hal::gpio::PinState::High);

        // Take ownership over the raw flash and rcc devices and convert them into the corresponding
        // HAL structs
        let mut flash = ctx.device.FLASH.constrain();
        let rcc = ctx.device.RCC.constrain();

        // Freeze the configuration of all the clocks in the system and store the frozen frequencies in
        // `clocks`
        let _clocks = rcc
            .cfgr
            .sysclk(stm32f1xx_hal::time::Hz(72_000_000))
            .freeze(&mut flash.acr);

        let systick_mono_token = rtic_monotonics::create_systick_token!();
        rtic_monotonics::systick::Systick::start(ctx.core.SYST, 72_000_000, systick_mono_token);

        led_task::spawn().unwrap();

        (Shared {}, Local { led })
    }

    #[task(local = [led])]
    async fn led_task(ctx: led_task::Context) {
        loop {
            info!("Toggle");
            ctx.local.led.toggle();
            Systick::delay(1000.millis()).await;
        }
    }
}
