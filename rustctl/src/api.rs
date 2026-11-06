use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::bresenham::MultiAxisPlanner;
use crate::config::{
    GEAR_RATIO, MotionConfig, POSITION_FORMAT, RAW_FORMAT, StartPosition, validate_motor_settings,
};
use crate::kinematics::{
    ArmSolution, deg_to_steps, forward_r_z_mm, ik_angles_3d_deg, overhead_sleep_us,
};
use crate::motion::{execute_position, execute_solution, step_plan};

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
    start_base_deg: Option<f64>,
    start_axis1_deg: Option<f64>,
    start_axis2_deg: Option<f64>,
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

    pub(crate) checks: Vec<&'static str>,
    pub(crate) failures: Vec<RuntimeTestFailure>,
}

pub(crate) fn hardware_enabled() -> bool {
    cfg!(all(feature = "hardware", target_os = "linux"))
}

pub(crate) fn api_help() -> String {
    format!(
        "JSON commands: {{\"command\":\"args\",\"radius_mm\":100,\"base_angle_deg\":0,\"height_mm\":50,\"l1_mm\":200,\"l2_mm\":200,\"steps_per_rev\":200,\"microstep\":16,\"ccw_positive\":true}} | {{\"command\":\"raw\",\"base_deg\":0,\"axis1_deg\":25,\"axis2_deg\":30,\"steps_per_rev\":200,\"microstep\":16,\"ccw_positive\":true}} | {{\"command\":\"status\"}} | {{\"command\":\"test\"}} | {{\"command\":\"help\"}} | {{\"command\":\"quit\"}} | optional start position on either motion command: {{\"start_base_deg\":0,\"start_axis1_deg\":10,\"start_axis2_deg\":20}} makes the move relative to those joint angles. Text formats: {POSITION_FORMAT} and {RAW_FORMAT}"
    )
}

fn start_position(request: &ApiRequest) -> Result<StartPosition, String> {
    let present = [
        request.start_base_deg,
        request.start_axis1_deg,
        request.start_axis2_deg,
    ];
    if present.iter().all(Option::is_none) {
        return Ok(StartPosition::default());
    }
    if present.iter().any(Option::is_none) {
        return Err(
            "start position needs all of start_base_deg, start_axis1_deg and start_axis2_deg"
                .to_owned(),
        );
    }
    Ok(StartPosition {
        base_deg: request.start_base_deg.expect("checked above"),
        axis1_deg: request.start_axis1_deg.expect("checked above"),
        axis2_deg: request.start_axis2_deg.expect("checked above"),
    })
}

pub(crate) fn parse_api_request(
    body: &str,
    default_command: Option<&str>,
) -> Result<ApiCommand, String> {
    let request: ApiRequest =
        serde_json::from_str(body).map_err(|error| format!("invalid JSON: {error}"))?;
    let default_command = default_command.map(str::to_ascii_lowercase);
    let body_command = request.command.as_deref().map(str::to_ascii_lowercase);
    let command = match (default_command, body_command) {
        (Some(route), Some(requested)) => {
            let canonical = if requested == "position" {
                "args"
            } else {
                requested.as_str()
            };
            if canonical != route {
                return Err(format!(
                    "command '{requested}' does not match route /{route}"
                ));
            }
            route.to_owned()
        }
        (Some(route), None) => route.to_owned(),
        (None, Some(requested)) => requested,
        (None, None) => return Err("missing JSON field 'command'".to_owned()),
    };

    match command.as_str() {
        "args" | "position" => {
            let steps_per_rev = request.steps_per_rev.ok_or("missing steps_per_rev")?;
            let microstep = request.microstep.ok_or("missing microstep")?;
            validate_motor_settings(steps_per_rev, microstep)?;
            let mut config = MotionConfig::new(
                request.radius_mm.ok_or("missing radius_mm")?,
                request.base_angle_deg.ok_or("missing base_angle_deg")?,
                request.height_mm.ok_or("missing height_mm")?,
                request.l1_mm.ok_or("missing l1_mm")?,
                request.l2_mm.ok_or("missing l2_mm")?,
                steps_per_rev,
                microstep,
                request.ccw_positive.ok_or("missing ccw_positive")?,
            );
            config.start = start_position(&request)?;
            Ok(ApiCommand::Args(config))
        }
        "raw" => {
            let steps_per_rev = request.steps_per_rev.ok_or("missing steps_per_rev")?;
            let microstep = request.microstep.ok_or("missing microstep")?;
            validate_motor_settings(steps_per_rev, microstep)?;
            let solution = ArmSolution {
                theta_base_deg: request.base_deg.ok_or("missing base_deg")?,
                theta1_deg: request.axis1_deg.ok_or("missing axis1_deg")?,
                theta2_deg: request.axis2_deg.ok_or("missing axis2_deg")?,
                z_eff_mm: 0.0,
            };
            let mut config = MotionConfig::new(
                0.0,
                0.0,
                0.0,
                1.0,
                1.0,
                steps_per_rev,
                microstep,
                request.ccw_positive.ok_or("missing ccw_positive")?,
            );
            config.start = start_position(&request)?;
            Ok(ApiCommand::Raw(config, solution))
        }
        "status" => Ok(ApiCommand::Status),
        "help" => Ok(ApiCommand::Help),
        "test" | "tests" => Ok(ApiCommand::Test),
        "quit" | "exit" => Ok(ApiCommand::Quit),
        _catch_all => Err(format!("unknown API command '{command}'")),
    }
}

pub(crate) type RuntimeCheck = (&'static str, fn() -> Result<(), String>);

pub(crate) fn report_runtime_checks(tests: &[RuntimeCheck]) -> RuntimeTestReport {
    let mut failures = Vec::new();
    for &(function, test) in tests {
        if let Err(message) = test() {
            failures.push(RuntimeTestFailure { function, message });
        }
    }
    RuntimeTestReport {
        passed: tests.len() - failures.len(),
        failed: failures.len(),
        checks: tests.iter().map(|(function, _)| *function).collect(),
        failures,
    }
}

pub(crate) fn validate_ik_roundtrip(radius: f64, height: f64) -> Result<(), String> {
    if (radius - 150.0).abs() > 1e-6 || (height - 40.0).abs() > 1e-6 {
        Err(format!(
            "forward result was radius={radius}, height={height}"
        ))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_step_conversion(steps: i64) -> Result<(), String> {
    if steps != 3200 {
        Err(format!("expected 3200 steps, got {steps}"))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_timing_validation(result: Result<u64, String>) -> Result<(), String> {
    match result {
        Ok(value) => Err(format!("accepted invalid timing and returned {value}")),
        Err(_) => Ok(()),
    }
}

pub(crate) fn validate_planner_totals(totals: [i64; 3]) -> Result<(), String> {
    if totals != [10, 5, 0] {
        Err(format!("planner totals were {totals:?}"))
    } else {
        Ok(())
    }
}

pub(crate) fn validate_relative_plan(
    deltas: [f64; 3],
    absolute: [i64; 3],
    relative: [i64; 3],
) -> Result<(), String> {
    if deltas != [20.0, 15.0, 20.0] {
        return Err(format!("joint deltas were {deltas:?}"));
    }
    if absolute.iter().zip(relative).any(|(a, r)| a - r == 0) {
        return Err(format!(
            "relative steps {relative:?} did not differ from absolute steps {absolute:?}"
        ));
    }
    if relative
        .iter()
        .zip(absolute)
        .any(|(r, a)| r.abs() >= a.abs())
    {
        return Err(format!(
            "relative steps {relative:?} were not shorter than absolute steps {absolute:?}"
        ));
    }
    Ok(())
}

pub(crate) fn runtime_test_report() -> RuntimeTestReport {
    let tests: [RuntimeCheck; 5] = [
        ("runtime_test_ik_roundtrip", || {
            let solution = runtime_ik_solution(150.0, 30.0, 40.0, 120.0, 90.0)?;
            let (radius, height) =
                forward_r_z_mm(solution.theta1_deg, solution.theta2_deg, 120.0, 90.0);
            validate_ik_roundtrip(radius, height)
        }),
        ("runtime_test_step_conversion", || {
            let steps = deg_to_steps(360.0, 200, 1, GEAR_RATIO);
            validate_step_conversion(steps)
        }),
        ("runtime_test_timing_validation", || {
            validate_timing_validation(overhead_sleep_us(482, 200))
        }),
        ("runtime_test_multi_axis_planner", || {
            let mut totals = [0; 3];
            for pulse in MultiAxisPlanner::new([10, 5, 0]) {
                for axis in 0..3 {
                    if pulse[axis] {
                        totals[axis] += 1;
                    }
                }
            }
            validate_planner_totals(totals)
        }),
        ("runtime_test_start_position_is_relative", || {
            let solution = ArmSolution {
                theta_base_deg: 30.0,
                theta1_deg: 40.0,
                theta2_deg: 50.0,
                z_eff_mm: 0.0,
            };
            let mut config = MotionConfig::new(0.0, 0.0, 0.0, 1.0, 1.0, 200, 1, true);
            let absolute = step_plan(&config, solution);
            config.start = StartPosition {
                base_deg: 10.0,
                axis1_deg: 20.0,
                axis2_deg: 35.0,
            };
            let relative = step_plan(&config, solution);
            let deltas = crate::motion::joint_deltas(config.start, solution);
            validate_relative_plan(deltas, absolute, relative)
        }),
    ];
    report_runtime_checks(&tests)
}

pub(crate) fn runtime_ik_solution(
    radius: f64,
    base_angle: f64,
    height: f64,
    l1: f64,
    l2: f64,
) -> Result<ArmSolution, String> {
    ik_angles_3d_deg(radius, base_angle, height, l1, l2).map_err(str::to_owned)
}

pub(crate) fn api_command_response(command: ApiCommand) -> Value {
    match command {
        ApiCommand::Status => {
            json!({"ok": true, "status": "ready", "hardware_enabled": hardware_enabled(), "busy": crate::control::is_busy(), "commands": ["args", "raw", "status", "help", "test", "quit"]})
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
        ("GET", _routecatch) => Err("unknown API route".to_owned()),
        (_methodcatch, _routecatch) => Err("method not allowed".to_owned()),
    }
}
