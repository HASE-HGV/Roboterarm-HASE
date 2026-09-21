use std::error::Error;

use crate::kinematics::ArmSolution;

pub(crate) const TOTAL_TIME_US: u64 = 1083;
pub(crate) const PULSE_T_US: u64 = 500;
pub(crate) const GEAR_RATIO: f64 = 16.0;
pub(crate) const NUM_AXES: usize = 3;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct MotionConfig {
    pub(crate) total_time_us: u64,
    pub(crate) pulse_t_us: u64,
    pub(crate) x_mm: f64,
    pub(crate) y_mm: f64,
    pub(crate) z_mm: f64,
    pub(crate) l1_mm: f64,
    pub(crate) l2_mm: f64,
    pub(crate) steps_per_rev: u64,
    pub(crate) microstep: u64,
    pub(crate) ccw_positive: bool,
}

pub(crate) fn config_from_position(values: &[&str]) -> Result<MotionConfig, Box<dyn Error>> {
    if values.len() != 8 {
        return Err("Expected: radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive, count: {values.len()}".into());
    }
    Ok(MotionConfig {
        total_time_us: TOTAL_TIME_US,
        pulse_t_us: PULSE_T_US,
        x_mm: values[0].parse::<f64>()?,
        y_mm: values[1].parse::<f64>()?,
        z_mm: values[2].parse::<f64>()?,
        l1_mm: values[3].parse::<f64>()?,
        l2_mm: values[4].parse::<f64>()?,
        steps_per_rev: values[5].parse::<u64>()?,
        microstep: values[6].parse::<u64>()?,
        ccw_positive: values[7].parse::<u8>()? != 0,
    })
}

pub(crate) fn config_from_line(line: &str) -> Result<MotionConfig, Box<dyn Error>> {
    config_from_position(&line.split_whitespace().collect::<Vec<_>>())
}

pub(crate) fn raw_command(line: &str) -> Result<(MotionConfig, ArmSolution), Box<dyn Error>> {
    let values: Vec<&str> = line.split_whitespace().collect();
    if values.len() != 6 {
        return Err(
            "Expected: base_deg axis1_deg axis2_deg steps_per_rev microstep ccw_positive".into(),
        );
    }
    let solution = ArmSolution {
        theta_base_deg: values[0].parse::<f64>()?,
        theta1_deg: values[1].parse::<f64>()?,
        theta2_deg: values[2].parse::<f64>()?,
        z_eff_mm: 0.0,
    };
    let config = MotionConfig {
        total_time_us: TOTAL_TIME_US,
        pulse_t_us: PULSE_T_US,
        x_mm: 0.0,
        y_mm: 0.0,
        z_mm: 0.0,
        l1_mm: 1.0,
        l2_mm: 1.0,
        steps_per_rev: values[3].parse::<u64>()?,
        microstep: values[4].parse::<u64>()?,
        ccw_positive: values[5].parse::<u8>()? != 0,
    };
    Ok((config, solution))
}
