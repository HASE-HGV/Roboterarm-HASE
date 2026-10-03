use std::error::Error;

use crate::kinematics::ArmSolution;

pub(crate) const TOTAL_TIME_US: u64 = 1083;
pub(crate) const PULSE_T_US: u64 = 500;
pub(crate) const GEAR_RATIO: f64 = 16.0;
pub(crate) const NUM_AXES: usize = 3;

pub(crate) const POSITION_FIELDS: usize = 8;

pub(crate) const POSITION_FIELDS_WITH_START: usize = 11;

pub(crate) const RAW_FIELDS: usize = 6;

pub(crate) const RAW_FIELDS_WITH_START: usize = 9;

pub(crate) const POSITION_FORMAT: &str = "radius_mm base_angle_deg height_mm l1_mm l2_mm steps_per_rev microstep ccw_positive [start_base_deg start_axis1_deg start_axis2_deg]";
pub(crate) const RAW_FORMAT: &str = "base_deg axis1_deg axis2_deg steps_per_rev microstep ccw_positive [start_base_deg start_axis1_deg start_axis2_deg]";

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub(crate) struct StartPosition {
    pub(crate) base_deg: f64,
    pub(crate) axis1_deg: f64,
    pub(crate) axis2_deg: f64,
}

impl StartPosition {
    pub(crate) fn is_homed(self) -> bool {
        self == StartPosition::default()
    }
}

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
    pub(crate) start: StartPosition,
}

impl MotionConfig {
    pub(crate) fn new(
        x_mm: f64,
        y_mm: f64,
        z_mm: f64,
        l1_mm: f64,
        l2_mm: f64,
        steps_per_rev: u64,
        microstep: u64,
        ccw_positive: bool,
    ) -> Self {
        Self {
            total_time_us: TOTAL_TIME_US,
            pulse_t_us: PULSE_T_US,
            x_mm,
            y_mm,
            z_mm,
            l1_mm,
            l2_mm,
            steps_per_rev,
            microstep,
            ccw_positive,
            start: StartPosition::default(),
        }
    }
}

fn parse_start_position(values: &[&str]) -> Result<StartPosition, Box<dyn Error>> {
    Ok(StartPosition {
        base_deg: values[0].parse::<f64>()?,
        axis1_deg: values[1].parse::<f64>()?,
        axis2_deg: values[2].parse::<f64>()?,
    })
}

pub(crate) fn config_from_position(values: &[&str]) -> Result<MotionConfig, Box<dyn Error>> {
    if values.len() != POSITION_FIELDS && values.len() != POSITION_FIELDS_WITH_START {
        return Err(format!(
            "Expected {POSITION_FIELDS} or {POSITION_FIELDS_WITH_START} values: \
             {POSITION_FORMAT}, got {}",
            values.len()
        )
        .into());
    }
    let start = if values.len() == POSITION_FIELDS_WITH_START {
        parse_start_position(&values[POSITION_FIELDS..])?
    } else {
        StartPosition::default()
    };
    let mut config = MotionConfig::new(
        values[0].parse::<f64>()?,
        values[1].parse::<f64>()?,
        values[2].parse::<f64>()?,
        values[3].parse::<f64>()?,
        values[4].parse::<f64>()?,
        values[5].parse::<u64>()?,
        values[6].parse::<u64>()?,
        values[7].parse::<u8>()? != 0,
    );
    config.start = start;
    Ok(config)
}

pub(crate) fn config_from_line(line: &str) -> Result<MotionConfig, Box<dyn Error>> {
    config_from_position(&line.split_whitespace().collect::<Vec<_>>())
}

pub(crate) fn raw_command(line: &str) -> Result<(MotionConfig, ArmSolution), Box<dyn Error>> {
    let values: Vec<&str> = line.split_whitespace().collect();
    if values.len() != RAW_FIELDS && values.len() != RAW_FIELDS_WITH_START {
        return Err(format!(
            "Expected {RAW_FIELDS} or {RAW_FIELDS_WITH_START} values: {RAW_FORMAT}, got {}",
            values.len()
        )
        .into());
    }
    let start = if values.len() == RAW_FIELDS_WITH_START {
        parse_start_position(&values[RAW_FIELDS..])?
    } else {
        StartPosition::default()
    };
    let solution = ArmSolution {
        theta_base_deg: values[0].parse::<f64>()?,
        theta1_deg: values[1].parse::<f64>()?,
        theta2_deg: values[2].parse::<f64>()?,
        z_eff_mm: 0.0,
    };
    let mut config = MotionConfig::new(
        0.0,
        0.0,
        0.0,
        1.0,
        1.0,
        values[3].parse::<u64>()?,
        values[4].parse::<u64>()?,
        values[5].parse::<u8>()? != 0,
    );
    config.start = start;
    Ok((config, solution))
}
