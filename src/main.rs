// #![deny(unsafe_code)]
#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

pub mod wiring;

// Print panic message to probe console
use defmt_rtt as _;
use panic_probe as _;

#[rtic::app(device = stm32f7xx_hal::pac, peripherals = true)]
mod app {

    use crate::wiring::*;
    use arrayvec::ArrayString;
    use defmt::info;
    use embedded_graphics::text::Text;
    use numtoa::NumToA;
    use rtic_monotonics::systick::*;
    use stm32f7xx_hal as hal;
    use stm32f7xx_hal::gpio::{Edge, ExtiPin};
    use stm32f7xx_hal::prelude::*;

    static STATIC_FONT_9X15: TextStyle = embedded_graphics::mono_font::MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_9X15)
        .text_color(embedded_graphics::pixelcolor::BinaryColor::On)
        .build();

    #[allow(dead_code)]
    static STATIC_FONT_10X20: TextStyle = embedded_graphics::mono_font::MonoTextStyleBuilder::new()
        .font(&embedded_graphics::mono_font::ascii::FONT_10X20)
        .text_color(embedded_graphics::pixelcolor::BinaryColor::On)
        .build();

    #[shared]
    struct Shared {}

    // Local resources go here
    #[local]
    struct Local {
        led: Led,
        ssd1306: Display,
        encoder: Encoder,
        enc_sender: rtic_sync::channel::Sender<'static, bool, 5>,
    }

    struct Encoder {
        counter: isize,
        button: EncoderButton,
        s1: S1,
        s2: S2,
        states: EncoderStates,
    }

    struct EncoderStates {
        pps1: bool,
        pps2: bool,
        ps1: bool,
        ps2: bool,
    }

    #[init]
    fn init(mut ctx: init::Context) -> (Shared, Local) {
        let mut rcc = ctx.device.RCC.constrain();

        let clocks = rcc.cfgr.sysclk(216_000_000.Hz()).freeze();

        let systick_mono_token = rtic_monotonics::create_systick_token!();
        Systick::start(ctx.core.SYST, 216_000_000, systick_mono_token);

        let gpiob = ctx.device.GPIOB.split();

        let led = gpiob.pb7.into_push_pull_output();

        let scl = gpiob.pb8.into_alternate_open_drain();
        let sda = gpiob.pb9.into_alternate_open_drain();
        let i2c = hal::i2c::BlockingI2c::i2c1(
            ctx.device.I2C1,
            (scl, sda),
            hal::i2c::Mode::FastPlus {
                frequency: 400_000.Hz(),
            },
            &clocks,
            &mut rcc.apb1,
            50_000,
        );

        let gpiof = ctx.device.GPIOF.split();
        let mut button = gpiof.pf13.into_pull_up_input();
        button.make_interrupt_source(&mut ctx.device.SYSCFG, &mut rcc.apb2);
        button.trigger_on_edge(&mut ctx.device.EXTI, Edge::Falling);
        button.enable_interrupt(&mut ctx.device.EXTI);

        let gpioe = ctx.device.GPIOE.split();
        let mut s1 = gpioe.pe11.into_floating_input();
        let mut s2 = gpiof.pf14.into_floating_input();

        s1.make_interrupt_source(&mut ctx.device.SYSCFG, &mut rcc.apb2);
        s1.trigger_on_edge(&mut ctx.device.EXTI, Edge::RisingFalling);
        s1.enable_interrupt(&mut ctx.device.EXTI);

        s2.make_interrupt_source(&mut ctx.device.SYSCFG, &mut rcc.apb2);
        s2.trigger_on_edge(&mut ctx.device.EXTI, Edge::RisingFalling);
        s2.enable_interrupt(&mut ctx.device.EXTI);

        // unsafe {
        //     NVIC::unmask::<interrupt>(interrupt::EXTI15_10);
        // }

        use embedded_graphics::pixelcolor::BinaryColor;
        use embedded_graphics::prelude::*;
        use embedded_graphics::text::{Baseline, Text};
        use embedded_graphics::Drawable;

        use ssd1306::rotation::DisplayRotation;
        use ssd1306::size::DisplaySize128x32;
        use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

        let interface = I2CDisplayInterface::new(i2c);
        let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate180)
            .into_buffered_graphics_mode();
        display.init().unwrap();

        display.clear(BinaryColor::Off).unwrap();
        Text::with_baseline(
            "Hello from\n deti luganska",
            Point::zero(),
            STATIC_FONT_9X15,
            Baseline::Top,
        )
        .draw(&mut display)
        .unwrap();
        display.flush().unwrap();

        let (s, r) = rtic_sync::make_channel!(bool, 5);

        wait_task::spawn(r).unwrap();

        // led_task::spawn().unwrap();

        // let (s, r) = make_channel!(&str, 5);
        (
            Shared {
               // Initialization of shared resources go here
            },
            Local {
                led,
                ssd1306: display,
                encoder: Encoder {
                    counter: 0,
                    button,
                    s1,
                    s2,
                    states: EncoderStates {
                        pps1: true,
                        pps2: true,
                        ps1: true,
                        ps2: true,
                    },
                },
                enc_sender: s,
            },
        )
    }

    #[task]
    async fn wait_task(
        _ctx: wait_task::Context,
        mut rx: rtic_sync::channel::Receiver<'static, bool, 5>,
    ) {
        use futures::{future::FutureExt, select_biased};

        loop {
            select_biased! {
                _ = rx.recv().fuse() => {}
                _ = Systick::delay(5.secs()).fuse() => {
                    let mut s = ArrayString::<64>::new();
                    s.push_str("Hello from\n deti luganska");
                    draw_on_display::spawn(STATIC_FONT_9X15, s, true).ok();
                }
            }
        }
    }

    #[task(local = [enc_sender])]
    async fn rtic_govna(ctx: rtic_govna::Context) {
        ctx.local.enc_sender.send(true).await.unwrap();
    }

    #[task(binds = EXTI15_10, local = [encoder])]
    fn exti_task(ctx: exti_task::Context) {
        if ctx.local.encoder.button.check_interrupt() {
            ctx.local.encoder.button.clear_interrupt_pending_bit();
            rtic_govna::spawn().ok();
            info!("button pressed");
            ctx.local.encoder.counter = 0;
            let mut button_str = ArrayString::<64>::new();
            button_str.push_str(ctx.local.encoder.counter.numtoa_str(10, &mut [0; 5]));
            button_str.push_str("\nButton pushed");
            draw_on_display::spawn(STATIC_FONT_9X15, button_str, true).ok();
        }
        if ctx.local.encoder.s2.check_interrupt() || ctx.local.encoder.s1.check_interrupt() {
            ctx.local.encoder.s1.clear_interrupt_pending_bit();
            ctx.local.encoder.s2.clear_interrupt_pending_bit();

            let states = &mut ctx.local.encoder.states;

            let res_s1 = ctx.local.encoder.s1.is_high();
            let res_s2 = ctx.local.encoder.s2.is_high();

            if (res_s1 == true && res_s2 == false)
                && (states.ps1 == false && states.ps2 == false)
                && (states.pps1 == false && states.pps2 == true)
            {
                // anticlockwise
                rtic_govna::spawn().ok();
                ctx.local.encoder.counter -= 1;
                let mut button_str = ArrayString::<64>::new();
                button_str.push_str(ctx.local.encoder.counter.numtoa_str(10, &mut [0; 5]));
                button_str.push_str("\nanticlockwise");
                draw_on_display::spawn(STATIC_FONT_9X15, button_str, true).ok();
            } else if (res_s1 == false && res_s2 == true)
                && (states.ps1 == false && states.ps2 == false)
                && (states.pps1 == true && states.pps2 == false)
            {
                // clockwise
                rtic_govna::spawn().ok();
                ctx.local.encoder.counter += 1;
                let mut button_str = ArrayString::<64>::new();
                button_str.push_str(ctx.local.encoder.counter.numtoa_str(10, &mut [0; 5]));
                button_str.push_str("\nclockwise");
                draw_on_display::spawn(STATIC_FONT_9X15, button_str, true).ok();
            }

            if res_s1 != states.ps1 || res_s2 != states.ps2 {
                states.pps1 = states.ps1;
                states.pps2 = states.ps2;
                states.ps1 = res_s1;
                states.ps2 = res_s2;
            }
        }
    }

    #[task(local = [led])]
    async fn led_task(ctx: led_task::Context) {
        let mut i = 0u16;

        use numtoa::NumToA;
        let mut ntabuf = [0u8; 5];
        let mut s = ArrayString::<64>::new();

        loop {
            s.push_str("Toggled ");
            s.push_str(i.numtoa_str(10, &mut ntabuf));

            ctx.local.led.toggle();
            i += 10;

            Systick::delay(500.millis()).await;

            info!("toggled {} times", i);
            draw_on_display::spawn(STATIC_FONT_9X15, s, true).ok();
            s.clear();
        }
    }

    #[task(local = [ssd1306])]
    async fn draw_on_display(
        ctx: draw_on_display::Context,
        text_style: TextStyle<'static>,
        string: ArrayString<64>,
        clear: bool,
    ) {
        use embedded_graphics::Drawable;

        if clear {
            ctx.local.ssd1306.clear_buffer();
        }
        Text::with_baseline(
            string.as_str(),
            embedded_graphics::prelude::Point::zero(),
            text_style,
            embedded_graphics::text::Baseline::Top,
        )
        .draw(ctx.local.ssd1306)
        .unwrap();
        ctx.local.ssd1306.flush().unwrap();
    }
}
