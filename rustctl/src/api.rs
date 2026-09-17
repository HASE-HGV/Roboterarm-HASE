use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::bresenham::MultiAxisPlanner;
use crate::config::{GEAR_RATIO, MotionConfig, PULSE_T_US, TOTAL_TIME_US};
use crate::kinematics::{
    ArmSolution, deg_to_steps, forward_r_z_mm, ik_angles_3d_deg, overhead_sleep_us,
};
use crate::motion::{execute_position, execute_solution};

#[derive(Debug, Deserialize)]
struct ApiRequest {
    #[serde(default)]
    command: Option<String>,
    radius_mm: Option<f64>,
    base_angle_deg: Option<f64>,
    height_mm: Option<f64>,
    l1_mm: Option<f64>,
    l2_mm: Option<f64>,
    steps_per_rev: Option<u64>,
    microstep: Option<u64>,
    ccw_positive: Option<bool>,
    base_deg: Option<f64>,
    axis1_deg: Option<f64>,
    axis2_deg: Option<f64>,
}

#[derive(Debug, PartialEq)]
pub(crate) enum ApiCommand {
    Args(MotionConfig),
    Raw(MotionConfig, ArmSolution),
    Status,
    Help,
    Test,
    Quit,
}

#[derive(Debug, Serialize)]
pub(crate) struct RuntimeTestFailure {
    pub(crate) function: &'static str,
    pub(crate) message: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct RuntimeTestReport {
    pub(crate) passed: usize,
    pub(crate) failed: usize,
    pub(crate) failures: Vec<RuntimeTestFailure>,
}

pub(crate) fn hardware_enabled() -> bool {
    cfg!(all(feature = "hardware", target_os = "linux"))
}

pub(crate) fn api_help() -> &'static str {
    "JSON commands: {\"command\":\"args\",\"radius_mm\":100,\"base_angle_deg\":0,\"height_mm\":50,\"l1_mm\":200,\"l2_mm\":200,\"steps_per_rev\":200,\"microstep\":16,\"ccw_positive\":true} | {\"command\":\"raw\",\"base_deg\":0,\"axis1_deg\":25,\"axis2_deg\":30,\"steps_per_rev\":200,\"microstep\":16,\"ccw_positive\":true} | {\"command\":\"status\"} | {\"command\":\"test\"} | {\"command\":\"help\"} | {\"command\":\"quit\"}"
}

pub(crate) fn parse_api_request(
    body: &str,
    default_command: Option<&str>,
) -> Result<ApiCommand, String> {
    let request: ApiRequest =
        serde_json::from_str(body).map_err(|error| format!("invalid JSON: {error}"))?;
    let command = request
        .command
        .as_deref()
        .or(default_command)
        .ok_or_else(|| "missing JSON field 'command'".to_owned())?
        .to_ascii_lowercase();

    match command.as_str() {
        "args" | "position" => {
            let config = MotionConfig {
                total_time_us: TOTAL_TIME_US,
                pulse_t_us: PULSE_T_US,
                x_mm: request.radius_mm.ok_or("missing radius_mm")?,
                y_mm: request.base_angle_deg.ok_or("missing base_angle_deg")?,
                z_mm: request.height_mm.ok_or("missing height_mm")?,
                l1_mm: request.l1_mm.ok_or("missing l1_mm")?,
                l2_mm: request.l2_mm.ok_or("missing l2_mm")?,
                steps_per_rev: request.steps_per_rev.ok_or("missing steps_per_rev")?,
                microstep: request.microstep.ok_or("missing microstep")?,
                ccw_positive: request.ccw_positive.ok_or("missing ccw_positive")?,
            };
            Ok(ApiCommand::Args(config))
        }
        "raw" => {
            let solution = ArmSolution {
                theta_base_deg: request.base_deg.ok_or("missing base_deg")?,
                theta1_deg: request.axis1_deg.ok_or("missing axis1_deg")?,
                theta2_deg: request.axis2_deg.ok_or("missing axis2_deg")?,
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
                steps_per_rev: request.steps_per_rev.ok_or("missing steps_per_rev")?,
                microstep: request.microstep.ok_or("missing microstep")?,
                ccw_positive: request.ccw_positive.ok_or("missing ccw_positive")?,
            };
            Ok(ApiCommand::Raw(config, solution))
        }
        "status" => Ok(ApiCommand::Status),
        "help" => Ok(ApiCommand::Help),
        "test" | "tests" => Ok(ApiCommand::Test),
        "quit" | "exit" => Ok(ApiCommand::Quit),
        _catch_all => Err(format!("unknown API command '{command}'")),
    }
}

pub(crate) fn runtime_test_report() -> RuntimeTestReport {
    let tests: [(&str, fn() -> Result<(), String>); 4] = [
        ("runtime_test_ik_roundtrip", || {
            let solution =
                ik_angles_3d_deg(150.0, 30.0, 40.0, 120.0, 90.0).map_err(str::to_owned)?;
            let (radius, height) =
                forward_r_z_mm(solution.theta1_deg, solution.theta2_deg, 120.0, 90.0);
            if (radius - 150.0).abs() > 1e-6 || (height - 40.0).abs() > 1e-6 {
                return Err(format!(
                    "forward result was radius={radius}, height={height}"
                ));
            }
            Ok(())
        }),
        ("runtime_test_step_conversion", || {
            let steps = deg_to_steps(360.0, 200, 1, GEAR_RATIO);
            if steps != 3200 {
                Err(format!("expected 3200 steps, got {steps}"))
            } else {
                Ok(())
            }
        }),
        (
            "runtime_test_timing_validation",
            || match overhead_sleep_us(482, 200) {
                Ok(value) => Err(format!("accepted invalid timing and returned {value}")),
                Err(_) => Ok(()),
            },
        ),
        ("runtime_test_multi_axis_planner", || {
            let mut totals = [0; 3];
            for pulse in MultiAxisPlanner::new([10, 5, 0]) {
                for axis in 0..3 {
                    if pulse[axis] {
                        totals[axis] += 1;
                    }
                }
            }
            if totals != [10, 5, 0] {
                Err(format!("planner totals were {totals:?}"))
            } else {
                Ok(())
            }
        }),
    ];
    let mut failures = Vec::new();
    for (function, test) in tests {
        if let Err(message) = test() {
            failures.push(RuntimeTestFailure { function, message });
        }
    }
    RuntimeTestReport {
        passed: tests.len() - failures.len(),
        failed: failures.len(),
        failures,
    }
}

pub(crate) fn api_command_response(command: ApiCommand) -> Value {
    match command {
        ApiCommand::Status => {
            json!({"ok": true, "status": "ready", "hardware_enabled": hardware_enabled(), "commands": ["args", "raw", "status", "help", "test", "quit"]})
        }
        ApiCommand::Help => json!({"ok": true, "help": api_help()}),
        ApiCommand::Test => {
            let tests = runtime_test_report();
            json!({"ok": tests.failed == 0, "status": "tests_completed", "tests": tests})
        }
        ApiCommand::Quit => json!({"ok": true, "status": "bye", "reason": "client"}),
        ApiCommand::Args(config) => match execute_position(config) {
            Ok(()) => {
                json!({"ok": true, "status": "done", "mode": "args", "hardware_enabled": hardware_enabled()})
            }
            Err(error) => {
                json!({"ok": false, "error": {"mode": "args", "message": error.to_string()}})
            }
        },
        ApiCommand::Raw(config, solution) => match execute_solution(&config, solution) {
            Ok(()) => {
                json!({"ok": true, "status": "done", "mode": "raw", "hardware_enabled": hardware_enabled()})
            }
            Err(error) => {
                json!({"ok": false, "error": {"mode": "raw", "message": error.to_string()}})
            }
        },
    }
}

pub(crate) fn api_command_for_request(
    method: &str,
    target: &str,
    body: &str,
) -> Result<ApiCommand, String> {
    let path = target.split('?').next().unwrap_or(target);
    match (method, path) {
        ("GET", "/status") if body.is_empty() => Ok(ApiCommand::Status),
        ("GET", "/help") if body.is_empty() => Ok(ApiCommand::Help),
        ("GET", "/test") if body.is_empty() => Ok(ApiCommand::Test),
        ("POST", "/args") => parse_api_request(body, Some("args")),
        ("POST", "/raw") => parse_api_request(body, Some("raw")),
        ("POST", "/api") => parse_api_request(body, None),
        ("GET", _) => Err("unknown API route".to_owned()),
        (_, _) => Err("method not allowed".to_owned()),
    }
}
