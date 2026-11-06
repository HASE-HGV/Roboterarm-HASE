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

pub(crate) fn validate_motor_settings(steps_per_rev: u64, microstep: u64) -> Result<(), String> {
    if steps_per_rev == 0 {
        return Err("steps_per_rev must be positive".to_owned());
    }
    if !matches!(microstep, 1 | 2 | 4 | 8 | 16) {
        return Err("microstep must be one of 1, 2, 4, 8, or 16".to_owned());
    }
    Ok(())
}

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

fn parse_finite_float(value: &str, field: &str) -> Result<f64, Box<dyn Error>> {
    let parsed = value.parse::<f64>()?;
    if !parsed.is_finite() {
        return Err(format!("{field} must be finite, got {value:?}").into());
    }
    Ok(parsed)
}

fn parse_start_position(values: &[&str]) -> Result<StartPosition, Box<dyn Error>> {
    Ok(StartPosition {
        base_deg: parse_finite_float(values[0], "start_base_deg")?,
        axis1_deg: parse_finite_float(values[1], "start_axis1_deg")?,
        axis2_deg: parse_finite_float(values[2], "start_axis2_deg")?,
    })
}

fn parse_direction_flag(value: &str) -> Result<bool, Box<dyn Error>> {
    match value.to_ascii_lowercase().as_str() {
        "0" | "false" => Ok(false),
        "1" | "true" => Ok(true),
        _ => Err(format!("ccw_positive must be 0/1 or true/false, got {value:?}").into()),
    }
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
    let steps_per_rev = values[5].parse::<u64>()?;
    let microstep = values[6].parse::<u64>()?;
    validate_motor_settings(steps_per_rev, microstep)
        .map_err(|error| -> Box<dyn Error> { error.into() })?;
    let mut config = MotionConfig::new(
        parse_finite_float(values[0], "radius_mm")?,
        parse_finite_float(values[1], "base_angle_deg")?,
        parse_finite_float(values[2], "height_mm")?,
        parse_finite_float(values[3], "l1_mm")?,
        parse_finite_float(values[4], "l2_mm")?,
        steps_per_rev,
        microstep,
        parse_direction_flag(values[7])?,
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
        theta_base_deg: parse_finite_float(values[0], "base_deg")?,
        theta1_deg: parse_finite_float(values[1], "axis1_deg")?,
        theta2_deg: parse_finite_float(values[2], "axis2_deg")?,
        z_eff_mm: 0.0,
    };
    let steps_per_rev = values[3].parse::<u64>()?;
    let microstep = values[4].parse::<u64>()?;
    validate_motor_settings(steps_per_rev, microstep)
        .map_err(|error| -> Box<dyn Error> { error.into() })?;
    let mut config = MotionConfig::new(
        0.0,
        0.0,
        0.0,
        1.0,
        1.0,
        steps_per_rev,
        microstep,
        parse_direction_flag(values[5])?,
    );
    config.start = start;
    Ok((config, solution))
}
