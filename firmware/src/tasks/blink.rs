use defmt::info;
use embassy_stm32::gpio::Output;
use embassy_time::Timer;

#[embassy_executor::task]
pub async fn blink_task(mut led: Output<'static>) {
    let millis = 1000;
    loop {
        // info!("high");
        led.set_high();
        Timer::after_millis(millis).await;
        // info!("low");
        led.set_low();
        Timer::after_millis(millis).await;
    }
}
