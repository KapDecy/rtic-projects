pub type Led = stm32f7xx_hal::gpio::Pin<'B', 7, stm32f7xx_hal::gpio::Output>;
pub type Display = ssd1306::Ssd1306<
    ssd1306::prelude::I2CInterface<
        stm32f7xx_hal::i2c::BlockingI2c<
            stm32f7xx_hal::pac::I2C1,
            stm32f7xx_hal::gpio::Pin<
                'B',
                8,
                stm32f7xx_hal::gpio::Alternate<4, stm32f7xx_hal::gpio::OpenDrain>,
            >,
            stm32f7xx_hal::gpio::Pin<
                'B',
                9,
                stm32f7xx_hal::gpio::Alternate<4, stm32f7xx_hal::gpio::OpenDrain>,
            >,
        >,
    >,
    ssd1306::size::DisplaySize128x32,
    ssd1306::mode::BufferedGraphicsMode<ssd1306::size::DisplaySize128x32>,
>;
pub type TextStyle<'a> =
    embedded_graphics::mono_font::MonoTextStyle<'a, embedded_graphics::pixelcolor::BinaryColor>;
pub type EncoderButton =
    stm32f7xx_hal::gpio::Pin<'F', 13, stm32f7xx_hal::gpio::Input<stm32f7xx_hal::gpio::PullUp>>;
pub type S1 = stm32f7xx_hal::gpio::Pin<'E', 11>;
pub type S2 = stm32f7xx_hal::gpio::Pin<'F', 14>;
