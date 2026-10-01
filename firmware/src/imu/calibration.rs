use super::*;

impl Imu {
    pub async fn calibrate(&mut self, samples: u16) -> Result<(), ImuDriverError> {
        let mut gyro = Vec3::<i32>::default();
        let mut accel = Vec3::<i32>::default();

        for _ in 0..samples {
            self.driver.wait_for_isr().await?;
            let raw = self.driver.read_all().await?;
            gyro = gyro.zip(raw.gyro.widen(), |a, b| a + b);
            accel = accel.zip(raw.accel.widen(), |a, b| a + b);
        }

        let n = samples as i32;
        self.gyro_bias = gyro.map(|v| v / n);

        // this was making things drift
        // let one_g = self.driver.accel_fs().lsb_per_g() as i32;
        // self.accel_bias = accel.map(|v| v / n).sub(Vec3::new(0, 0, one_g));

        info!(
            "imu bias gyro: {} | accel: {}",
            self.gyro_bias, self.accel_bias
        );
        Ok(())
    }

    pub async fn calibrate_accel_screen_down(
        &mut self,
        samples: u16,
    ) -> Result<bool, ImuDriverError> {
        let mut accel = Vec3::<i32>::default();

        for _ in 0..samples {
            self.driver.wait_for_isr().await?;
            let raw = self.driver.read_all().await?;
            accel = accel.zip(raw.accel.widen(), |a, b| a + b);
        }

        let average = accel.map(|v| v / samples as i32);
        let one_g = self.driver.accel_fs().lsb_per_g() as i32;

        if average.z.abs() < one_g * 9 / 10
            || average.x.abs() > one_g / 4
            || average.y.abs() > one_g / 4
        {
            error!(
                "board is not screen down, keeping accel bias: {} {} {}",
                average.x, average.y, average.z
            );
            return Ok(false);
        }

        let up = if average.z > 0 { one_g } else { -one_g };
        self.accel_bias = average.sub(Vec3::new(0, 0, up));

        info!(
            "ACCEL_BIAS: Vec3::new({}, {}, {}),",
            self.accel_bias.x, self.accel_bias.y, self.accel_bias.z
        );
        info!(
            "screen down reference {} {} {}, 1g = {}",
            average.x, average.y, average.z, one_g
        );

        Ok(true)
    }

    pub fn bias(&self) -> (Vec3<i32>, Vec3<i32>) {
        (self.gyro_bias, self.accel_bias)
    }

    pub fn set_bias(&mut self, gyro: Vec3<i32>, accel: Vec3<i32>) {
        self.gyro_bias = gyro;
        self.accel_bias = accel;
    }
}
