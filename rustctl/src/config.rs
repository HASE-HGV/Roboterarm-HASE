use serde::Deserialize;
use std::error::Error;
use std::{fs, sync::OnceLock};

use crate::kinematics::ArmSolution;

pub(crate) const NUM_AXES: usize = 3;

// Variables used for reproducible testing
#[cfg(test)]
pub(crate) const TOTAL_TIME_US: u64 = 1083;
#[cfg(test)]
pub(crate) const PULSE_T_US: u64 = 500;
#[cfg(test)]
pub(crate) const GEAR_RATIO: f64 = 16.0;

#[derive(Debug, Clone, Copy, Default, Deserialize)]
#[serde(default)]
// Hardware and pin settings loaded from src/configs/config.toml.
pub(crate) struct HardwareConfig {
    pub(crate) hardware: HardwareSettings,
    pub(crate) pins: Pins,
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
pub(crate) struct HardwareSettings {
    pub(crate) gear_ratio: f64,
    pub(crate) pulse_t_us: u64,
    pub(crate) overhead_us: u64,
    pub(crate) motion_profile: MotionProfile,
    pub(crate) ramp_steps: u64,
    pub(crate) start_period_multiplier: u64,
}

#[derive(Debug, Clone, Copy, Default, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]

//defining Trapezoidal MotionProfile as Default, Fixed only runs if specified

pub(crate) enum MotionProfile {
    Fixed,
    #[default]
    Trapezoidal,
}

impl HardwareSettings {
    pub(crate) fn target_time_us(self) -> u64 {
        self.pulse_t_us
            .checked_mul(2)
            .and_then(|pulse| pulse.checked_add(self.overhead_us))
            .expect("hardware timing is validated at startup")
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
#[serde(default)]
// Each pair is [STEP, DIR] for one motor, this syntax defines an "array" of 2 u8 integers, as
// bigger is not needed, max of u8 is 255 but max acessible GPIO pin number is 27.
pub(crate) struct Pins {
    pub(crate) m1: [u8; 2],
    pub(crate) m2: [u8; 2],
    pub(crate) m3: [u8; 2],
    pub(crate) m4: [u8; 2],
}
//Defaults Pin config for when config file is missing values or is missing all together
impl Default for Pins {
    fn default() -> Self {
        Self {
            m1: [17, 27],
            m2: [22, 23],
            m3: [24, 25],
            m4: [5, 6],
        }
    }
}

// Default Hardware properties

impl Default for HardwareSettings {
    fn default() -> Self {
        Self {
            gear_ratio: 16.0,
            pulse_t_us: 500,
            overhead_us: 83,
            motion_profile: MotionProfile::Trapezoidal,
            ramp_steps: 32,
            start_period_multiplier: 2,
        }
    }
}

static HARDWARE_CONFIG: OnceLock<HardwareConfig> = OnceLock::new();

pub(crate) fn settings() -> &'static HardwareConfig {
    HARDWARE_CONFIG.get_or_init(HardwareConfig::default)
}

// function to load the settings from the default path

pub(crate) fn load_settings() -> Result<(), Box<dyn Error>> {
    let path = "src/configs/config.toml";
    let config = match fs::read_to_string(path) {
        Ok(text) => toml::from_str::<HardwareConfig>(&text)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => HardwareConfig::default(),
        Err(error) => return Err(error.into()),
    };
    validate_settings(&config)?;
    let _ = HARDWARE_CONFIG.set(config);
    Ok(())
}

// TODO add load_settigns_from()


//checks if the loaded config makes sense/is usable NOT COMPATIBLE WITH load_settigns_from()
fn validate_settings(config: &HardwareConfig) -> Result<(), Box<dyn Error>> {
    if !config.hardware.gear_ratio.is_finite() || config.hardware.gear_ratio <= 0.0 {
        return Err("gear_ratio must be finite and positive".into());
    }
    crate::kinematics::overhead_sleep_us_with_overhead(
        config.hardware.target_time_us(),
        config.hardware.pulse_t_us,
        config.hardware.overhead_us,
    )?;
    if config.hardware.motion_profile == MotionProfile::Trapezoidal {
        if config.hardware.ramp_steps == 0 {
            return Err("ramp_steps must be positive for the trapezoidal profile".into());
        }
        if config.hardware.start_period_multiplier < 2 {
            return Err("start_period_multiplier must be at least 2".into());
        }
        config
            .hardware
            .target_time_us()
            .checked_mul(config.hardware.start_period_multiplier)
            .ok_or("maximum ramp period exceeds u64")?;
    }
    let pins = [
        config.pins.m1,
        config.pins.m2,
        config.pins.m3,
        config.pins.m4,
    ];
    let flat: Vec<u8> = pins.into_iter().flatten().collect();
    if flat.iter().any(|pin| *pin > 27)
        || flat
            .iter()
            .enumerate()
            .any(|(i, pin)| flat[..i].contains(pin))
    {
        return Err("GPIO pins must be unique BCM numbers from 0 through 27".into());
    }
    Ok(())
}

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
    if steps_per_rev > u64::MAX / microstep {
        return Err(format!(
            "steps_per_rev is too large for microstep {microstep}; steps_per_rev * microstep must fit in u64"
        ));
    }
    Ok(())
}
// struct for the Start Position of each axis
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
    #[allow(clippy::too_many_arguments)] // here solely to prevent clippy from being a bad guy
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
            total_time_us: settings().hardware.target_time_us(),
            pulse_t_us: settings().hardware.pulse_t_us,
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
