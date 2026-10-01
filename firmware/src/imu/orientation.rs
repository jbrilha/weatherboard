use defmt::{error, info};
use embassy_time::Timer;
use micromath::F32Ext;

use crate::display::{self, Orientation};
use crate::imu::{Imu, Vec3};

#[derive(Clone, Copy)]
#[allow(dead_code)]
enum Axis {
    X,
    Y,
    Z,
}

#[embassy_executor::task]
pub async fn imu_task(mut imu: Imu) {
    if imu.init().await.is_err() {
        error!("failed to init imu");
        return;
    }
    info!("imu ready");

    let mut orientation = display::requests::get_orientation()
        .await
        .expect("expected display to work??");

    loop {
        imu.wait_for_data().await;

        let accel = match imu.read().await {
            Ok(sample) => sample.accel,
            Err(_) => {
                error!("failed to read IMU");
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
            display::requests::set_orientation(orientation)
                .await
                .expect("expected orientation setter to work too...");
            info!(
                "orientation -> {} at accel {} {} {}",
                orientation.as_str(),
                accel.x,
                accel.y,
                accel.z
            );
        }

        Timer::after_millis(1000).await;
    }
}

const SCREEN_RIGHT: Axis = Axis::X;
const SCREEN_UP: Axis = Axis::Y;
const INVERT_RIGHT: bool = true;
const INVERT_UP: bool = false;
const FLAT_G: f32 = 0.3;
const RAD_TO_DEG: f32 = 57.295_78;
const SWITCH_DEG: f32 = 45.0;
const HYSTERESIS_DEG: f32 = 12.0;

fn wrap180(angle: f32) -> f32 {
    let wrapped = angle.rem_euclid(360.0);
    if wrapped > 180.0 {
        wrapped - 360.0
    } else {
        wrapped
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

fn center_of(orientation: Orientation) -> f32 {
    match orientation {
        Orientation::Portrait => 0.0,
        Orientation::Landscape => 90.0,
        Orientation::PortraitFlipped => 180.0,
        Orientation::LandscapeFlipped => -90.0,
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
