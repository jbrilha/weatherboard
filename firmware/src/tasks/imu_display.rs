use core::fmt::Write;

use defmt::{error, info};
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::Point,
    mono_font::{MonoTextStyle, ascii::FONT_7X14},
    pixelcolor::Rgb565,
    prelude::{RgbColor, WebColors},
    primitives::{Circle, Line, PrimitiveStyle, PrimitiveStyleBuilder, StyledDrawable},
    text::Text,
};
use heapless::String;
use micromath::F32Ext;

use crate::display::{Dimensions, Display, Orientation};
use crate::imu::{Imu, Vec3, fusion::Attitude};

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Axis {
    X,
    Y,
    Z,
}

const SCREEN_RIGHT: Axis = Axis::X;
const SCREEN_UP: Axis = Axis::Y;
const INVERT_RIGHT: bool = false;
const INVERT_UP: bool = false;

const ACCEL_BIAS: Vec3<i32> = Vec3::new(782, 170, 642);

const CALIBRATION_SAMPLES: u16 = 500;
const FRAME_EVERY: u32 = 32;
const SWITCH_DEG: f32 = 45.0;
const HYSTERESIS_DEG: f32 = 12.0;
const FLAT_G: f32 = 0.3;
const PX_PER_DEG: f32 = 2.0;
const DEG_TO_RAD: f32 = 0.017_453_292;
const RAD_TO_DEG: f32 = 57.295_78;

#[embassy_executor::task]
pub async fn imu_display_task(mut display: Display, mut imu: Imu) {
    if display.init().await.is_err() {
        error!("failed to init display");
        return;
    }

    let mut orientation = Orientation::Portrait;
    display.set_orientation(orientation).await.unwrap();
    display.clear_screen(Rgb565::BLACK).await.unwrap();

    if imu.init().await.is_err() {
        error!("failed to init imu");
        return;
    }

    info!("hold the board still, calibrating gyro");
    if imu.calibrate(CALIBRATION_SAMPLES).await.is_err() {
        error!("failed to calibrate imu");
        return;
    }

    let (gyro_bias, _) = imu.bias();
    imu.set_bias(gyro_bias, ACCEL_BIAS);
    info!(
        "imu ready, accel bias {} {} {}",
        ACCEL_BIAS.x, ACCEL_BIAS.y, ACCEL_BIAS.z
    );

    let mut n = 0u32;
    loop {
        imu.wait_for_data().await;

        let attitude = match imu.track().await {
            Ok(attitude) => attitude,
            Err(_) => {
                error!("failed to read imu");
                continue;
            }
        };

        n = n.wrapping_add(1);
        if n % FRAME_EVERY != 0 {
            continue;
        }

        let (accel, temp) = match imu.read().await {
            Ok(sample) => (sample.accel, sample.temp),
            Err(_) => {
                error!("failed to read imu");
                continue;
            }
        };

        let angle = screen_angle(accel);
        let wanted = match angle {
            Some(angle) => orientation_for(angle, orientation),
            None => orientation,
        };

        if wanted != orientation {
            orientation = wanted;
            display.set_orientation(orientation).await.unwrap();
            info!(
                "orientation -> {} at accel {} {} {}",
                orientation.as_str(),
                accel.x,
                accel.y,
                accel.z
            );
        }

        draw_frame(&mut display, attitude, orientation, angle, temp);
        display.flush().await.unwrap();
    }
}

fn wrap180(angle: f32) -> f32 {
    let wrapped = angle.rem_euclid(360.0);
    if wrapped > 180.0 {
        wrapped - 360.0
    } else {
        wrapped
    }
}

fn center_of(orientation: Orientation) -> f32 {
    match orientation {
        Orientation::Portrait => 0.0,
        Orientation::Landscape => 90.0,
        Orientation::PortraitFlipped => 180.0,
        Orientation::LandscapeFlipped => -90.0,
    }
}

fn sector_of(roll: f32) -> Orientation {
    let roll = wrap180(roll);
    if roll >= -SWITCH_DEG && roll < SWITCH_DEG {
        Orientation::Portrait
    } else if roll >= SWITCH_DEG && roll < 135.0 {
        Orientation::Landscape
    } else if roll >= -135.0 && roll < -SWITCH_DEG {
        Orientation::LandscapeFlipped
    } else {
        Orientation::PortraitFlipped
    }
}

fn orientation_for(angle: f32, current: Orientation) -> Orientation {
    let from_center = wrap180(wrap180(angle) - center_of(current)).abs();
    if from_center <= SWITCH_DEG + HYSTERESIS_DEG {
        current
    } else {
        sector_of(angle)
    }
}

fn component(accel: Vec3<f32>, axis: Axis, invert: bool) -> f32 {
    let value = match axis {
        Axis::X => accel.x,
        Axis::Y => accel.y,
        Axis::Z => accel.z,
    };
    if invert { -value } else { value }
}

fn screen_angle(accel: Vec3<f32>) -> Option<f32> {
    let right = component(accel, SCREEN_RIGHT, INVERT_RIGHT);
    let up = component(accel, SCREEN_UP, INVERT_UP);

    if (right * right + up * up).sqrt() < FLAT_G {
        None
    } else {
        Some(right.atan2(-up) * RAD_TO_DEG)
    }
}

fn stroke(color: Rgb565, width: u32) -> PrimitiveStyle<Rgb565> {
    PrimitiveStyleBuilder::new()
        .stroke_color(color)
        .stroke_width(width)
        .build()
}

fn draw_frame(
    display: &mut Display,
    attitude: Attitude,
    orientation: Orientation,
    angle: Option<f32>,
    temp: f32,
) {
    let w = display.width() as i32;
    let h = display.height() as i32;

    display.clear(Rgb565::BLACK).unwrap();

    let label = MonoTextStyle::new(&FONT_7X14, Rgb565::WHITE);
    let mut line: String<32> = String::new();

    let _ = write!(line, "roll  {:>7.1}", attitude.roll);
    Text::new(line.as_str(), Point::new(8, 18), label)
        .draw(display)
        .unwrap();

    line.clear();
    let _ = write!(line, "pitch {:>7.1}", attitude.pitch);
    Text::new(line.as_str(), Point::new(8, 34), label)
        .draw(display)
        .unwrap();

    line.clear();
    let _ = write!(line, "yaw   {:>7.1}", attitude.yaw);
    Text::new(line.as_str(), Point::new(8, 50), label)
        .draw(display)
        .unwrap();

    line.clear();
    match angle {
        Some(angle) => {
            let _ = write!(line, "screen{:>7.1}", angle);
        }
        None => {
            let _ = write!(line, "screen   flat");
        }
    }
    Text::new(line.as_str(), Point::new(8, 66), label)
        .draw(display)
        .unwrap();

    line.clear();
    let _ = write!(line, "temp  {:>7.1} C", temp);
    Text::new(line.as_str(), Point::new(8, 82), label)
        .draw(display)
        .unwrap();

    line.clear();
    let _ = write!(line, "{}", orientation.as_str());
    Text::new(
        line.as_str(),
        Point::new(8, 98),
        MonoTextStyle::new(&FONT_7X14, Rgb565::CSS_GREEN_YELLOW),
    )
    .draw(display)
    .unwrap();

    let cx = w as f32 / 2.0;
    let cy = h as f32 / 2.0;

    Line::new(Point::new(w / 2 - 12, h / 2), Point::new(w / 2 + 12, h / 2))
        .draw_styled(&stroke(Rgb565::CSS_DIM_GRAY, 1), display)
        .unwrap();

    let radians = attitude.roll * DEG_TO_RAD;
    let (sin, cos) = (radians.sin(), radians.cos());
    let half = w as f32 * 0.4;
    let shifted = cy + attitude.pitch * PX_PER_DEG;

    Line::new(
        Point::new((cx - half * cos) as i32, (shifted + half * sin) as i32),
        Point::new((cx + half * cos) as i32, (shifted - half * sin) as i32),
    )
    .draw_styled(&stroke(Rgb565::CSS_ORANGE, 2), display)
    .unwrap();

    let needle_center = Point::new(w / 2, h - 56);
    let radius = 34.0;

    Circle::with_center(needle_center, (radius * 2.0) as u32)
        .draw_styled(&stroke(Rgb565::CSS_DIM_GRAY, 1), display)
        .unwrap();

    let yaw_radians = attitude.yaw * DEG_TO_RAD;
    Line::new(
        needle_center,
        Point::new(
            needle_center.x + (radius * yaw_radians.sin()) as i32,
            needle_center.y - (radius * yaw_radians.cos()) as i32,
        ),
    )
    .draw_styled(&stroke(Rgb565::CSS_CYAN, 2), display)
    .unwrap();
}
