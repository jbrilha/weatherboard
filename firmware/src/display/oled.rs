use embassy_stm32::{
    i2c::{I2c, Master},
    mode::Async,
};
use embassy_time::Timer;
use micromath::F32Ext;

use embassy_ssd1306::Ssd1306;

#[embassy_executor::task]
pub async fn oled_task(i2c: I2c<'static, Async, Master>) {
    let mut oled = Ssd1306::new(i2c, 0x3C);

    oled.init().await.unwrap();

    // Texte avec caractères spéciaux
    oled.draw_str(0, 0, b"HELLO (WORLD)");

    // Nombre signé avec caractères
    oled.draw_str(0, 1, b"TEMP: ");
    oled.draw_i16(30, 1, -12);
    oled.draw_str(45, 1, b"C");

    // Exemples avec caractères additionnels
    oled.draw_str(0, 2, b"[STATUS]=OK!");
    oled.draw_str(0, 3, b"50% DONE <->");
    oled.draw_str(0, 4, b"TEST: A,B,C");

    // Rectangle de bordure
    oled.draw_rect(0, 0, 128, 64, true);

    oled.flush().await.unwrap();

    let calculs = [4.0, 16.0, 25.0, 23.0];
    let mut idx = 0;

    loop {
        oled.clear();
        oled.draw_rect(0, 0, 127, 63, true);

        let n: f32 = calculs[idx];
        let res_q15 = n.sqrt();
        let res_humain = (res_q15 as i32 * 181 + 16384) / 32768;

        oled.draw_str(10, 1, b"sqrt");
        oled.draw_str(65, 1, b"Resultat");

        let next_x = oled.draw_i16(10, 3, n as i16);
        oled.draw_char(next_x + 5, 3, 38); // '.'
        oled.draw_i16(80, 3, res_humain as i16);

        let _ = oled.flush().await;

        idx = (idx + 1) % calculs.len();
        Timer::after_secs(1).await;

        Timer::after_millis(1000).await;
    }
}
