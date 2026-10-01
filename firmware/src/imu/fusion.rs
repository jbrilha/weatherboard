use defmt::Format;
use micromath::F32Ext;

use crate::imu::Vec3;

const RAD_TO_DEG: f32 = 57.295_78;

/// weight of the integrated gyro rate
const DEFAULT_ALPHA: f32 = 0.98;

/// accelerometer magnitude in g
const TRUST_BAND: f32 = 0.15;

/// minimum gravity in the roll plane, in g, below which roll is undefined
const ROLL_MIN_G: f32 = 0.1;

#[derive(Debug, Clone, Copy, Default, Format)]
pub struct Attitude {
    /// deg, rotation about x
    pub roll: f32,
    /// deg, rotation about y
    pub pitch: f32,
    /// deg, rotation about z, gyro only
    pub yaw: f32,
}

/// extra filter over the accelerometer and gyroscope
pub struct Fusion {
    attitude: Attitude,
    alpha: f32,
}

impl Default for Fusion {
    fn default() -> Self {
        Self::new()
    }
}

impl Fusion {
    pub const fn new() -> Self {
        Self::with_alpha(DEFAULT_ALPHA)
    }

    pub const fn with_alpha(alpha: f32) -> Self {
        Self {
            attitude: Attitude {
                roll: 0.0,
                pitch: 0.0,
                yaw: 0.0,
            },
            alpha,
        }
    }

    pub fn attitude(&self) -> Attitude {
        self.attitude
    }

    pub fn reset(&mut self) {
        self.attitude = Attitude::default();
    }

    /// accel in g, gyro in deg/s, dt in seconds
    pub fn update(&mut self, accel: Vec3<f32>, gyro: Vec3<f32>, dt: f32) -> Attitude {
        let roll_observable = accel.y * accel.y + accel.z * accel.z > ROLL_MIN_G * ROLL_MIN_G;

        if dt <= 0.0 {
            let measured = tilt(accel);
            self.attitude = Attitude {
                roll: if roll_observable {
                    measured.roll
                } else {
                    self.attitude.roll
                },
                pitch: measured.pitch,
                yaw: self.attitude.yaw,
            };
            return self.attitude;
        }

        let roll = wrap180(self.attitude.roll + gyro.x * dt);
        let pitch = self.attitude.pitch + gyro.y * dt;
        let yaw = wrap180(self.attitude.yaw + gyro.z * dt);

        let magnitude = (accel.x * accel.x + accel.y * accel.y + accel.z * accel.z).sqrt();
        self.attitude = if (magnitude - 1.0).abs() > TRUST_BAND {
            Attitude { roll, pitch, yaw }
        } else {
            let measured = tilt(accel);
            let correction = 1.0 - self.alpha;
            Attitude {
                roll: if roll_observable {
                    wrap180(roll + correction * wrap180(measured.roll - roll))
                } else {
                    roll
                },
                pitch: pitch + correction * (measured.pitch - pitch),
                yaw,
            }
        };

        self.attitude
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

fn tilt(accel: Vec3<f32>) -> Attitude {
    Attitude {
        roll: accel.y.atan2(accel.z) * RAD_TO_DEG,
        pitch: (-accel.x).atan2((accel.y * accel.y + accel.z * accel.z).sqrt()) * RAD_TO_DEG,
        yaw: 0.0,
    }
}
