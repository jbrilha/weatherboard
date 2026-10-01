use embassy_stm32::{Config, Peri, bind_interrupts, dma, exti, i2c, interrupt, peripherals};

bind_interrupts!(pub struct Irqs {
    I2C1_EV => i2c::EventInterruptHandler<peripherals::I2C1>;
    I2C1_ER => i2c::ErrorInterruptHandler<peripherals::I2C1>;
    GPDMA1_CHANNEL0 => dma::InterruptHandler<peripherals::GPDMA1_CH0>;
    GPDMA1_CHANNEL1 => dma::InterruptHandler<peripherals::GPDMA1_CH1>;
    GPDMA1_CHANNEL2 => dma::InterruptHandler<peripherals::GPDMA1_CH2>;
    EXTI11 => exti::InterruptHandler<interrupt::typelevel::EXTI11>;
    EXTI12 => exti::InterruptHandler<interrupt::typelevel::EXTI12>;
});

macro_rules! board {
    ($(
        $(#[$group_meta:meta])*
        $board_field:ident : $group:ident {
            $($(#[$field_meta:meta])* $field:ident : $periph:ident),* $(,)?
        }
    ),* $(,)?) => {
        $(
            $(#[$group_meta])*
            pub struct $group {
                $($(#[$field_meta])* pub $field: Peri<'static, peripherals::$periph>,)*
            }
        )*

        pub struct Board {
            $(pub $board_field: $group,)*
        }

        impl Board {
            pub fn init() -> Self {
                let p = embassy_stm32::init(clock_config());
                Self {
                    $($board_field: $group {
                        $($field: p.$periph,)*
                    },)*
                }
            }
        }
    };
}

board! {
    /// 4.2" tri-color SSD1683 e-paper, TX-only SPI
    epd: EpdPeripherals {
        spi: SPI1,
        sck: PB4,
        mosi: PA15,
        dma_tx: GPDMA1_CH2,
        cs: PC13,
        dc: PB3,
        reset: PB8,
        busy_pin: PA12,
        busy: EXTI12,
    },

    imu: ImuPeripherals {
        i2c: I2C1,
        sda: PB1,
        scl: PB2,
        dma_tx: GPDMA1_CH0,
        dma_rx: GPDMA1_CH1,
        irq_pin: PA11,
        irq: EXTI11,
    },
}

pub fn clock_config() -> Config {
    let mut config = Config::default();
    config.rcc.mux.sai1sel = embassy_stm32::rcc::mux::Sai1sel::HSI;
    config
}
