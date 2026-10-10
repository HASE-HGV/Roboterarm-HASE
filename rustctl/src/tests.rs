use std::io::{Cursor, Read, Write};
use std::net::{Ipv4Addr, Shutdown, SocketAddr, TcpListener, TcpStream};
use std::sync::Mutex;
use std::thread;
use std::time::Duration;

use crate::cli::{get_mode, print_help, prompt_position_with_io};
use crate::control::{BusyGuard, is_busy};
use crate::http_api::{
    PAGE_HTML, RequestError, handle_connection, handle_request, origin_is_trusted,
    read_http_request,
};
use crate::net::{Address, rank_addresses};
use crate::shell::{run_position_loop_with_io, run_raw_loop_with_io};

static BUSY_TEST_LOCK: Mutex<()> = Mutex::new(());

fn busy_test_lock() -> std::sync::MutexGuard<'static, ()> {
    BUSY_TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

#[test]
fn api_parses_args_mode() {
    let command = parse_api_request(
        r#"{"command":"args","radius_mm":100,"base_angle_deg":20,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#,
        None,
    )
    .unwrap();
    match command {
        ApiCommand::Args(config) => {
            assert_eq!(config.x_mm, 100.0);
            assert_eq!(config.y_mm, 20.0);
            assert_eq!(config.z_mm, 50.0);
            assert_eq!(config.steps_per_rev, 200);
            assert_eq!(config.microstep, 16);
            assert!(config.ccw_positive);
        }
        other => panic!("expected args command, got {other:?}"),
    }
}

#[test]
fn api_parses_position_alias_case_insensitively() {
    assert!(matches!(
        parse_api_request(
            r#"{"command":"POSITION","radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":false}"#,
            None,
        ),
        Ok(ApiCommand::Args(_))
    ));
}

#[test]
fn api_parses_raw_mode() {
    let command = parse_api_request(
        r#"{"command":"raw","base_deg":-10,"axis1_deg":25,"axis2_deg":30,"steps_per_rev":400,"microstep":8,"ccw_positive":false}"#,
        None,
    )
    .unwrap();
    match command {
        ApiCommand::Raw(config, solution) => {
            assert_eq!(solution.theta_base_deg, -10.0);
            assert_eq!(solution.theta1_deg, 25.0);
            assert_eq!(solution.theta2_deg, 30.0);
            assert_eq!(config.steps_per_rev, 400);
            assert_eq!(config.microstep, 8);
            assert!(!config.ccw_positive);
        }
        other => panic!("expected raw command, got {other:?}"),
    }
}

#[test]
fn api_parses_control_commands() {
    assert_eq!(
        parse_api_request(r#"{"command":"status"}"#, None),
        Ok(ApiCommand::Status)
    );
    assert_eq!(
        parse_api_request(r#"{"command":"help"}"#, None),
        Ok(ApiCommand::Help)
    );
    assert_eq!(
        parse_api_request(r#"{"command":"test"}"#, None),
        Ok(ApiCommand::Test)
    );
    assert_eq!(
        parse_api_request(r#"{"command":"quit"}"#, None),
        Ok(ApiCommand::Quit)
    );
}

#[test]
fn api_rejects_missing_or_extra_arguments() {
    assert!(parse_api_request(r#"{"command":"args","radius_mm":100}"#, None).is_err());
    assert!(parse_api_request(r#"{"command":"raw","base_deg":0,"axis1_deg":0,"axis2_deg":0,"steps_per_rev":200,"microstep":16}"#, None).is_err());
    assert!(parse_api_request(r#"{"command":"unknown"}"#, None).is_err());
}

#[test]
fn api_rejects_unknown_and_empty_commands() {
    assert!(parse_api_request("", None).is_err());
    assert!(parse_api_request(r#"{"command":"dance"}"#, None).is_err());
}

#[test]
fn api_routes_http_status_and_help() {
    assert_eq!(
        api_command_for_request("GET", "/status", ""),
        Ok(ApiCommand::Status)
    );
    assert_eq!(
        api_command_for_request("GET", "/help?format=text", ""),
        Ok(ApiCommand::Help)
    );
    assert_eq!(
        api_command_for_request("GET", "/test", ""),
        Ok(ApiCommand::Test)
    );
}

#[test]
fn api_routes_http_motion_modes() {
    assert!(matches!(
        api_command_for_request(
            "POST",
            "/args",
            r#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#
        ),
        Ok(ApiCommand::Args(_))
    ));
    assert!(matches!(
        api_command_for_request(
            "POST",
            "/raw",
            r#"{"base_deg":0,"axis1_deg":0,"axis2_deg":0,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#
        ),
        Ok(ApiCommand::Raw(_, _))
    ));
    assert!(matches!(
        api_command_for_request("POST", "/api", r#"{"command":"status"}"#),
        Ok(ApiCommand::Status)
    ));
}

#[test]
fn api_rejects_invalid_http_routes_and_methods() {
    assert!(api_command_for_request("GET", "/args", "{}").is_err());
    assert!(api_command_for_request("POST", "/missing", "").is_err());
    assert!(api_command_for_request("GET", "/status", "unexpected").is_err());
}

#[test]
fn api_reports_compiled_hardware_capability() {
    assert_eq!(
        hardware_enabled(),
        cfg!(all(feature = "hardware", target_os = "linux"))
    );
}

#[test]
fn runtime_tests_report_named_failures() {
    let report = runtime_test_report();
    assert_eq!(report.failed, 0, "failures: {:?}", report.failures);
    assert_eq!(report.passed, 5);
    assert!(report.failures.is_empty());
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6
}

use crate::config::StartPosition;
use crate::motion::{joint_deltas, plan_report, step_plan};

#[test]
fn deg_to_steps_full_revolution() {
    assert_eq!(deg_to_steps(360.0, 200, 1, GEAR_RATIO), 3200);
}

#[test]
fn deg_to_steps_quarter_and_microstep() {
    assert_eq!(deg_to_steps(90.0, 200, 1, GEAR_RATIO), 800);
    assert_eq!(deg_to_steps(90.0, 200, 16, GEAR_RATIO), 12800);
}

#[test]
fn deg_to_steps_sign_and_zero() {
    assert_eq!(deg_to_steps(-90.0, 200, 1, GEAR_RATIO), -800);
    assert_eq!(deg_to_steps(0.0, 200, 16, GEAR_RATIO), 0);
}

#[test]
fn deg_to_steps_rounds_to_nearest() {
    assert_eq!(deg_to_steps(1.0, 200, 1, GEAR_RATIO), 9);
}

#[test]
fn direction_logic_all_quadrants() {
    assert_eq!(direction_is_ccw(5, true), true);
    assert_eq!(direction_is_ccw(-5, true), false);
    assert_eq!(direction_is_ccw(5, false), false);
    assert_eq!(direction_is_ccw(-5, false), true);
}

#[test]
fn overhead_normal() {
    assert_eq!(overhead_sleep_us(1000, 200).unwrap(), 517);
}

#[test]
fn overhead_exact_minimum_is_zero() {
    assert_eq!(overhead_sleep_us(483, 200).unwrap(), 0);
}

#[test]
fn overhead_below_minimum_errors() {
    assert!(overhead_sleep_us(482, 200).is_err());
}

#[test]
fn ik_fully_extended_along_x() {
    let s = ik_angles_3d_deg(200.0, 0.0, 0.0, 100.0, 100.0).unwrap();
    assert!(approx(s.theta_base_deg, 0.0));
    assert!(approx(s.theta1_deg, 0.0));
    assert!(approx(s.theta2_deg, 0.0));
    assert!(approx(s.z_eff_mm, 0.0));
}

#[test]
fn ik_right_angle_reach() {
    let s = ik_angles_3d_deg(100.0, 0.0, 100.0, 100.0, 100.0).unwrap();
    assert!(approx(s.theta1_deg, 0.0));
    assert!(approx(s.theta2_deg, 90.0));
    assert!(approx(s.z_eff_mm, 100.0));
}

#[test]
fn ik_base_rotation_90() {
    let s = ik_angles_3d_deg(100.0, 90.0, 100.0, 100.0, 100.0).unwrap();
    assert!(approx(s.theta_base_deg, 90.0));
}

#[test]
fn ik_negative_radius_rejected() {
    assert_eq!(
        ik_angles_3d_deg(-1.0, 0.0, 0.0, 100.0, 100.0),
        Err("Radius X must be non-negative")
    );
}

#[test]
fn ik_out_of_workspace() {
    assert_eq!(
        ik_angles_3d_deg(300.0, 0.0, 0.0, 100.0, 100.0),
        Err("Out of workspace")
    );
}

#[test]
fn ik_zero_link_rejected() {
    assert!(ik_angles_3d_deg(50.0, 0.0, 0.0, 0.0, 100.0).is_err());
    assert!(ik_angles_3d_deg(50.0, 0.0, 0.0, 100.0, 0.0).is_err());
}

#[test]
fn ik_forward_roundtrip() {
    let (l1, l2) = (120.0, 90.0);
    for &(radius, base_angle, height) in
        &[(150.0, 30.0, 40.0), (80.0, -60.0, 20.0), (0.0, 90.0, 50.0)]
    {
        let s = ik_angles_3d_deg(radius, base_angle, height, l1, l2).unwrap();
        let (r_eff, z_eff) = forward_r_z_mm(s.theta1_deg, s.theta2_deg, l1, l2);
        assert!(
            approx(r_eff, radius),
            "radius mismatch: {r_eff} vs {radius}"
        );
        assert!(
            approx(z_eff, height),
            "height mismatch: {z_eff} vs {height}"
        );
        assert!(approx(s.theta_base_deg, base_angle));
    }
}

#[test]
fn planner_emits_exact_counts() {
    let steps = [30i64, 12, 7];
    let mut totals = [0i64; 3];
    let mut ticks = 0;
    for p in MultiAxisPlanner::new(steps) {
        for i in 0..3 {
            if p[i] {
                totals[i] += 1;
            }
        }
        ticks += 1;
    }
    assert_eq!(ticks, 30);
    assert_eq!(totals, [30, 12, 7]);
}

#[test]
fn planner_dominant_axis_pulses_every_tick() {
    let plan: Vec<[bool; 3]> = MultiAxisPlanner::new([10i64, 0, 0]).collect();
    assert_eq!(plan.len(), 10);
    assert!(plan.iter().all(|t| t[0] && !t[1] && !t[2]));
}

#[test]
fn planner_uses_step_magnitude() {
    let mut totals = [0i64; 3];
    for p in MultiAxisPlanner::new([-8i64, 4, -2]) {
        for i in 0..3 {
            if p[i] {
                totals[i] += 1;
            }
        }
    }
    assert_eq!(totals, [8, 4, 2]);
}

#[test]
fn planner_no_motion_yields_nothing() {
    assert_eq!(MultiAxisPlanner::new([0i64, 0, 0]).count(), 0);
}

#[test]
fn planner_distributes_evenly() {
    let mut max_gap = 0;
    let mut gap = 0;
    for p in MultiAxisPlanner::new([10i64, 5, 0]) {
        if p[1] {
            max_gap = max_gap.max(gap);
            gap = 0;
        } else {
            gap += 1;
        }
    }
    assert!(
        max_gap <= 1,
        "pulses should be evenly spaced, gap={max_gap}"
    );
}

#[test]
fn start_position_defaults_to_homed() {
    let config = config_from_position(&["100", "0", "50", "200", "200", "200", "16", "1"]).unwrap();
    assert_eq!(config.start, StartPosition::default());
    assert!(config.start.is_homed());
}

#[test]
fn position_command_accepts_an_explicit_start_position() {
    let values = [
        "100", "0", "50", "200", "200", "200", "16", "1", "10", "20", "30",
    ];
    let config = config_from_position(&values).unwrap();
    assert_eq!(
        config.start,
        StartPosition {
            base_deg: 10.0,
            axis1_deg: 20.0,
            axis2_deg: 30.0
        }
    );
    assert!(!config.start.is_homed());
}

#[test]
fn position_command_rejects_a_partial_start_position() {
    let values = [
        "100", "0", "50", "200", "200", "200", "16", "1", "10", "20",
    ];
    let error = config_from_position(&values).unwrap_err().to_string();
    assert!(error.contains("8 or 11"), "{error}");
    assert!(!config_from_line("100 0 50 200 200 200 16 1 10 20").is_ok());
}

#[test]
fn position_argument_count_errors_name_the_expected_counts() {
    let error = config_from_position(&["1", "2", "3"]).unwrap_err().to_string();
    assert!(error.contains("8 or 11"), "{error}");
    assert!(error.contains("got 3"), "{error}");
}

#[test]
fn start_position_angles_must_be_numbers() {
    for values in [
        vec!["100", "0", "50", "200", "200", "200", "16", "1", "x", "20", "30"],
        vec!["100", "0", "50", "200", "200", "200", "16", "1", "10", "x", "30"],
        vec!["100", "0", "50", "200", "200", "200", "16", "1", "10", "20", "x"],
    ] {
        assert!(config_from_position(&values).is_err(), "values={values:?}");
    }
}

#[test]
fn raw_command_accepts_an_explicit_start_position() {
    let (config, solution) = raw_command("0 25 30 200 16 1 10 20 30").unwrap();
    assert_eq!(
        config.start,
        StartPosition {
            base_deg: 10.0,
            axis1_deg: 20.0,
            axis2_deg: 30.0
        }
    );
    assert_eq!(solution.theta1_deg, 25.0);
}

#[test]
fn raw_command_without_start_position_is_homed() {
    let (config, _) = raw_command("0 25 30 200 16 1").unwrap();
    assert!(config.start.is_homed());
}

#[test]
fn raw_command_start_position_must_be_complete_and_numeric() {
    let error = raw_command("0 25 30 200 16 1 10 20").unwrap_err().to_string();
    assert!(error.contains("6 or 9"), "{error}");
    assert!(raw_command("0 25 30 200 16 1 x 20 30").is_err());
}



fn position_line(target: [f64; 3], start: Option<[f64; 3]>) -> String {
    let mut fields = vec![
        target[0].to_string(),
        target[1].to_string(),
        target[2].to_string(),
        "200".to_owned(),
        "200".to_owned(),
        "200".to_owned(),
        "1".to_owned(),
        "1".to_owned(),
    ];
    if let Some(start) = start {
        fields.extend(start.map(|value| value.to_string()));
    }
    fields.join(" ")
}

#[test]
fn relative_move_pulses_only_the_joint_travel() {
    let solution = ArmSolution {
        theta_base_deg: 0.0,
        theta1_deg: 90.0,
        theta2_deg: 90.0,
        z_eff_mm: 0.0,
    };
    let config = config_from_line(&position_line([0.0, 0.0, 0.0], Some([0.0, 45.0, 45.0]))).unwrap();
    let expected = deg_to_steps(45.0, 200, 1, GEAR_RATIO);
    assert_eq!(step_plan(&config, solution), [expected, expected, 0]);
}

#[test]
fn homed_move_pulses_the_full_target_angle() {
    let solution = ArmSolution {
        theta_base_deg: 0.0,
        theta1_deg: 90.0,
        theta2_deg: 90.0,
        z_eff_mm: 0.0,
    };
    let config = config_from_line(&position_line([0.0, 0.0, 0.0], None)).unwrap();
    assert!(config.start.is_homed());
    assert_eq!(step_plan(&config, solution), [800, 800, 0]);
}

#[test]
fn start_position_equal_to_target_commands_no_steps() {
    let solution = ArmSolution {
        theta_base_deg: 40.0,
        theta1_deg: 90.0,
        theta2_deg: 90.0,
        z_eff_mm: 0.0,
    };
    let config = config_from_line(&position_line([0.0, 0.0, 0.0], Some([40.0, 90.0, 90.0]))).unwrap();
    assert_eq!(step_plan(&config, solution), [0, 0, 0]);
}

#[test]
fn negative_travel_from_start_pulses_in_the_opposite_direction() {
    let solution = ArmSolution {
        theta_base_deg: 0.0,
        theta1_deg: 0.0,
        theta2_deg: 0.0,
        z_eff_mm: 0.0,
    };
    let config = config_from_line(&position_line([0.0, 0.0, 0.0], Some([90.0, 90.0, 90.0]))).unwrap();
    assert_eq!(step_plan(&config, solution), [-800, -800, -800]);
}

#[test]
fn joint_deltas_follow_the_axis1_axis2_base_order() {
    let solution = ArmSolution {
        theta_base_deg: 30.0,
        theta1_deg: 40.0,
        theta2_deg: 50.0,
        z_eff_mm: 0.0,
    };
    let start = StartPosition {
        base_deg: 10.0,
        axis1_deg: 20.0,
        axis2_deg: 35.0,
    };
    assert_eq!(joint_deltas(start, solution), [20.0, 15.0, 20.0]);
    assert_eq!(joint_deltas(StartPosition::default(), solution), [40.0, 50.0, 30.0]);
}

#[test]
fn step_plan_scales_with_microstepping() {
    let solution = ArmSolution {
        theta_base_deg: 0.0,
        theta1_deg: 90.0,
        theta2_deg: 0.0,
        z_eff_mm: 0.0,
    };
    let mut coarse = config_from_line(&position_line([0.0, 0.0, 0.0], Some([0.0, 45.0, 0.0]))).unwrap();
    let mut fine = coarse;
    coarse.microstep = 1;
    fine.microstep = 16;
    assert_eq!(step_plan(&fine, solution)[0], step_plan(&coarse, solution)[0] * 16);
    assert_eq!(step_plan(&coarse, solution), [400, 0, 0]);
    assert_eq!(step_plan(&fine, solution), [6400, 0, 0]);
}

#[test]
fn api_accepts_a_start_position_on_args() {
    let command = parse_api_request(
        r#"{"command":"args","radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true,"start_base_deg":5,"start_axis1_deg":10,"start_axis2_deg":15}"#,
        None,
    )
    .unwrap();
    match command {
        ApiCommand::Args(config) => assert_eq!(
            config.start,
            StartPosition {
                base_deg: 5.0,
                axis1_deg: 10.0,
                axis2_deg: 15.0
            }
        ),
        other => panic!("expected args command, got {other:?}"),
    }
}

#[test]
fn api_accepts_a_start_position_on_raw() {
    let command = parse_api_request(
        r#"{"command":"raw","base_deg":0,"axis1_deg":25,"axis2_deg":30,"steps_per_rev":200,"microstep":16,"ccw_positive":true,"start_base_deg":5,"start_axis1_deg":10,"start_axis2_deg":15}"#,
        None,
    )
    .unwrap();
    match command {
        ApiCommand::Raw(config, _) => assert_eq!(
            config.start,
            StartPosition {
                base_deg: 5.0,
                axis1_deg: 10.0,
                axis2_deg: 15.0
            }
        ),
        other => panic!("expected raw command, got {other:?}"),
    }
}

#[test]
fn api_without_start_position_stays_homed() {
    let command = parse_api_request(
        r#"{"command":"args","radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#,
        None,
    )
    .unwrap();
    match command {
        ApiCommand::Args(config) => assert!(config.start.is_homed()),
        other => panic!("expected args command, got {other:?}"),
    }
}

#[test]
fn api_rejects_a_partial_start_position_on_both_modes() {
    let args = r#"{"command":"args","radius_mm":1,"base_angle_deg":2,"height_mm":3,"l1_mm":4,"l2_mm":5,"steps_per_rev":6,"microstep":7,"ccw_positive":true,"start_base_deg":1}"#;
    let raw = r#"{"command":"raw","base_deg":1,"axis1_deg":2,"axis2_deg":3,"steps_per_rev":4,"microstep":5,"ccw_positive":true,"start_axis1_deg":1,"start_axis2_deg":2}"#;
    for body in [args, raw] {
        let error = parse_api_request(body, None).unwrap_err();
        assert!(error.contains("all of start_base_deg"), "{error}");
    }
}

#[test]
fn api_start_position_must_be_numeric() {
    assert!(
        parse_api_request(
            r#"{"command":"raw","base_deg":0,"axis1_deg":0,"axis2_deg":0,"steps_per_rev":4,"microstep":5,"ccw_positive":true,"start_base_deg":"x","start_axis1_deg":1,"start_axis2_deg":2}"#,
            None
        )
        .is_err()
    );
}

#[test]
fn api_help_documents_the_start_position() {
    let help = api_help();
    assert!(help.contains("start_base_deg"), "{help}");
    assert!(help.contains("start_axis1_deg"), "{help}");
    assert!(help.contains("start_axis2_deg"), "{help}");
    assert!(help.contains("relative"), "{help}");
}

#[test]
fn config_parses_position_values_and_fixed_timing() {
    let config =
        config_from_position(&["100", "-20.5", "50", "200", "150", "400", "8", "1"]).unwrap();
    assert_eq!(config.total_time_us, TOTAL_TIME_US);
    assert_eq!(config.pulse_t_us, PULSE_T_US);
    assert_eq!(config.x_mm, 100.0);
    assert_eq!(config.y_mm, -20.5);
    assert_eq!(config.z_mm, 50.0);
    assert_eq!(config.l1_mm, 200.0);
    assert_eq!(config.l2_mm, 150.0);
    assert_eq!(config.steps_per_rev, 400);
    assert_eq!(config.microstep, 8);
    assert!(config.ccw_positive);
}

#[test]
fn config_accepts_zero_and_nonzero_direction_flags() {
    let zero = config_from_line("1 2 3 4 5 6 7 0").unwrap();
    let nonzero = config_from_line("1 2 3 4 5 6 7 2").unwrap();
    assert!(!zero.ccw_positive);
    assert!(nonzero.ccw_positive);
}

#[test]
fn config_rejects_wrong_position_argument_counts() {
    for values in [
        vec![],
        vec!["1"],
        vec!["1", "2", "3", "4", "5", "6", "7"],
        vec!["1", "2", "3", "4", "5", "6", "7", "8", "9"],
    ] {
        assert!(config_from_position(&values).is_err(), "values={values:?}");
    }
}

#[test]
fn config_rejects_invalid_position_numbers() {
    let cases = [
        ["x", "2", "3", "4", "5", "6", "7", "0"],
        ["1", "x", "3", "4", "5", "6", "7", "0"],
        ["1", "2", "x", "4", "5", "6", "7", "0"],
        ["1", "2", "3", "x", "5", "6", "7", "0"],
        ["1", "2", "3", "4", "x", "6", "7", "0"],
        ["1", "2", "3", "4", "5", "x", "7", "0"],
        ["1", "2", "3", "4", "5", "6", "x", "0"],
        ["1", "2", "3", "4", "5", "6", "7", "x"],
    ];
    for values in cases {
        assert!(config_from_position(&values).is_err(), "values={values:?}");
    }
}

#[test]
fn raw_command_parses_values_and_fixed_geometry() {
    let (config, solution) = raw_command("-10 25 30 400 8 1").unwrap();
    assert_eq!(solution.theta_base_deg, -10.0);
    assert_eq!(solution.theta1_deg, 25.0);
    assert_eq!(solution.theta2_deg, 30.0);
    assert_eq!(solution.z_eff_mm, 0.0);
    assert_eq!(config.l1_mm, 1.0);
    assert_eq!(config.l2_mm, 1.0);
    assert_eq!(config.steps_per_rev, 400);
    assert_eq!(config.microstep, 8);
}

#[test]
fn raw_command_rejects_wrong_counts_and_invalid_numbers() {
    assert!(raw_command("").is_err());
    assert!(raw_command("1 2 3 4 5").is_err());
    assert!(raw_command("1 2 3 4 5 6 7").is_err());
    assert!(raw_command("x 2 3 4 5 1").is_err());
    assert!(raw_command("1 x 3 4 5 1").is_err());
    assert!(raw_command("1 2 x 4 5 1").is_err());
    assert!(raw_command("1 2 3 x 5 1").is_err());
    assert!(raw_command("1 2 3 4 x 1").is_err());
    assert!(raw_command("1 2 3 4 5 x").is_err());
}

#[test]
fn api_command_names_are_case_insensitive_and_aliases_work() {
    for name in [
        "status", "STATUS", "help", "HELP", "test", "tests", "quit", "exit",
    ] {
        assert!(parse_api_request(&format!(r#"{{"command":"{name}"}}"#), None).is_ok());
    }
    assert!(matches!(
        parse_api_request(
            r#"{"radius_mm":1,"base_angle_deg":2,"height_mm":3,"l1_mm":4,"l2_mm":5,"steps_per_rev":6,"microstep":7,"ccw_positive":true}"#,
            Some("ARGS")
        ),
        Ok(ApiCommand::Args(_))
    ));
}

#[test]
fn api_rejects_invalid_json_and_missing_command() {
    assert!(
        parse_api_request("not json", None)
            .unwrap_err()
            .starts_with("invalid JSON:")
    );
    assert_eq!(
        parse_api_request("{}", None).unwrap_err(),
        "missing JSON field 'command'"
    );
    assert!(parse_api_request(r#"{"command":null}"#, None).is_err());
    assert!(parse_api_request(r#"{"command":123}"#, None).is_err());
}

#[test]
fn api_rejects_missing_each_position_field() {
    let fields = [
        "radius_mm",
        "base_angle_deg",
        "height_mm",
        "l1_mm",
        "l2_mm",
        "steps_per_rev",
        "microstep",
        "ccw_positive",
    ];
    for missing in fields {
        let mut request = serde_json::json!({
            "command": "args",
            "radius_mm": 1,
            "base_angle_deg": 2,
            "height_mm": 3,
            "l1_mm": 4,
            "l2_mm": 5,
            "steps_per_rev": 6,
            "microstep": 7,
            "ccw_positive": true,
        });
        request.as_object_mut().unwrap().remove(missing);
        let body = request.to_string();
        assert!(parse_api_request(&body, None).is_err(), "missing={missing}");
    }
}

#[test]
fn api_rejects_missing_each_raw_field() {
    let fields = [
        "base_deg",
        "axis1_deg",
        "axis2_deg",
        "steps_per_rev",
        "microstep",
        "ccw_positive",
    ];
    for missing in fields {
        let mut request = serde_json::json!({
            "command": "raw",
            "base_deg": 1,
            "axis1_deg": 2,
            "axis2_deg": 3,
            "steps_per_rev": 4,
            "microstep": 5,
            "ccw_positive": true,
        });
        request.as_object_mut().unwrap().remove(missing);
        let body = request.to_string();
        assert!(parse_api_request(&body, None).is_err(), "missing={missing}");
    }
}

#[test]
fn api_response_shapes_are_stable_for_control_commands() {
    let status = api_command_response(ApiCommand::Status);
    assert_eq!(status["ok"], true);
    assert_eq!(status["status"], "ready");
    assert!(
        status["commands"]
            .as_array()
            .unwrap()
            .contains(&serde_json::json!("raw"))
    );

    let help = api_command_response(ApiCommand::Help);
    assert_eq!(help["ok"], true);
    assert!(help["help"].as_str().unwrap().contains("JSON commands"));

    let quit = api_command_response(ApiCommand::Quit);
    assert_eq!(
        quit,
        serde_json::json!({"ok": true, "status": "bye", "reason": "client"})
    );
}

#[test]
fn api_test_response_contains_runtime_report() {
    let response = api_command_response(ApiCommand::Test);
    assert_eq!(response["ok"], true);
    assert_eq!(response["status"], "tests_completed");
    assert_eq!(response["tests"]["failed"], 0);
    assert_eq!(response["tests"]["passed"], 5);
    assert_eq!(response["tests"]["failures"].as_array().unwrap().len(), 0);
}

#[test]
fn api_executes_valid_simulation_commands() {
    let args = parse_api_request(
        r#"{"command":"args","radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#,
        None,
    )
    .unwrap();
    let raw = parse_api_request(
        r#"{"command":"raw","base_deg":0,"axis1_deg":25,"axis2_deg":30,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#,
        None,
    )
    .unwrap();
    let args_response = api_command_response(args);
    let raw_response = api_command_response(raw);
    assert_eq!(args_response["ok"], true);
    assert_eq!(args_response["mode"], "args");
    assert_eq!(raw_response["ok"], true);
    assert_eq!(raw_response["mode"], "raw");
}

#[test]
fn api_route_query_strings_do_not_change_routes() {
    assert_eq!(
        api_command_for_request("GET", "/status?format=json", ""),
        Ok(ApiCommand::Status)
    );
    assert!(api_command_for_request("GET", "/status", " ").is_err());
    assert!(api_command_for_request("POST", "/args?verbose=true", "{}").is_err());
}

#[test]
fn api_route_method_errors_are_distinct() {
    assert_eq!(
        api_command_for_request("PUT", "/status", "").unwrap_err(),
        "method not allowed"
    );
    assert_eq!(
        api_command_for_request("GET", "/missing", "").unwrap_err(),
        "unknown API route"
    );
}

#[test]
fn forward_kinematics_known_angles() {
    assert_eq!(forward_r_z_mm(0.0, 0.0, 10.0, 5.0), (15.0, 0.0));
    let (radius, height) = forward_r_z_mm(90.0, 0.0, 10.0, 5.0);
    assert!(approx(radius, 0.0));
    assert!(approx(height, 15.0));
    let (radius, height) = forward_r_z_mm(0.0, 90.0, 10.0, 5.0);
    assert!(approx(radius, 10.0));
    assert!(approx(height, 5.0));
}

#[test]
fn ik_accepts_outer_workspace_boundary() {
    let solution = ik_angles_3d_deg(300.0, 12.0, 0.0, 200.0, 100.0).unwrap();
    assert!(approx(solution.theta_base_deg, 12.0));
    assert!(approx(solution.theta1_deg, 0.0));
    assert!(approx(solution.theta2_deg, 0.0));
}

#[test]
fn ik_accepts_negative_height_and_preserves_it() {
    let solution = ik_angles_3d_deg(100.0, -30.0, -50.0, 100.0, 100.0).unwrap();
    let (radius, height) = forward_r_z_mm(solution.theta1_deg, solution.theta2_deg, 100.0, 100.0);
    assert!(approx(radius, 100.0));
    assert!(approx(height, -50.0));
    assert!(approx(solution.theta_base_deg, -30.0));
}

#[test]
fn ik_rejects_inner_workspace_boundary() {
    assert_eq!(
        ik_angles_3d_deg(50.0, 0.0, 0.0, 100.0, 25.0),
        Err("Out of workspace")
    );
}

#[test]
fn ik_rejects_non_finite_inputs() {
    assert!(ik_angles_3d_deg(f64::NAN, 0.0, 0.0, 100.0, 100.0).is_err());
    assert!(ik_angles_3d_deg(f64::INFINITY, 0.0, 0.0, 100.0, 100.0).is_err());
    assert!(ik_angles_3d_deg(100.0, f64::NAN, 0.0, 100.0, 100.0).is_err());
    assert!(ik_angles_3d_deg(100.0, 0.0, f64::INFINITY, 100.0, 100.0).is_err());
    assert!(ik_angles_3d_deg(100.0, 0.0, 0.0, f64::NAN, 100.0).is_err());
}

#[test]
fn deg_to_steps_zero_parameters_are_predictable() {
    assert_eq!(deg_to_steps(90.0, 0, 16, GEAR_RATIO), 0);
    assert_eq!(deg_to_steps(90.0, 200, 0, GEAR_RATIO), 0);
    assert_eq!(deg_to_steps(90.0, 200, 16, 0.0), 0);
}

#[test]
fn deg_to_steps_is_antisymmetric_for_many_angles() {
    for angle in -720..=720 {
        let angle = angle as f64 / 3.0;
        assert_eq!(
            deg_to_steps(-angle, 200, 16, GEAR_RATIO),
            -deg_to_steps(angle, 200, 16, GEAR_RATIO),
            "angle={angle}"
        );
    }
}

#[test]
fn direction_logic_zero_is_not_positive() {
    assert!(!direction_is_ccw(0, true));
    assert!(direction_is_ccw(0, false));
}

#[test]
fn timing_validation_handles_multiple_pulse_widths() {
    for pulse in [0, 1, 10, 200, 1_000] {
        let minimum = minimum_period_us(pulse);
        assert_eq!(overhead_sleep_us(minimum, pulse), Ok(0));
        assert!(overhead_sleep_us(minimum.saturating_sub(1), pulse).is_err());
        assert_eq!(overhead_sleep_us(minimum + 17, pulse), Ok(17));
    }
}

#[test]
fn planner_counts_are_exact_for_many_vectors() {
    for a in 0..=20 {
        for b in 0..=20 {
            for c in 0..=20 {
                let expected = [a, b, c];
                let mut actual = [0; 3];
                for pulse in MultiAxisPlanner::new([a, -b, c]) {
                    for axis in 0..3 {
                        actual[axis] += i64::from(pulse[axis]);
                    }
                }
                assert_eq!(actual, expected, "steps={expected:?}");
            }
        }
    }
}

fn http_exchange(request: &[u8]) -> Vec<u8> {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        handle_connection(stream).unwrap();
    });

    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(request).unwrap();
    client.shutdown(Shutdown::Write).unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    server.join().unwrap();
    response
}

#[test]
fn cli_prompt_parses_injected_input() {
    let input = b"100\n20\n50\n200\n150\n400\n8\n1\n";
    let mut output = Vec::new();
    let config = prompt_position_with_io(Cursor::new(input), &mut output).unwrap();
    assert_eq!(config.x_mm, 100.0);
    assert_eq!(config.y_mm, 20.0);
    assert_eq!(config.l2_mm, 150.0);
    assert!(config.ccw_positive);
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("Target radius X")
    );
}

#[test]
fn cli_prompt_collects_a_start_position_when_given() {
    let input = b"100\n20\n50\n200\n150\n400\n8\n1\n10\n20\n30\n";
    let mut output = Vec::new();
    let config = prompt_position_with_io(Cursor::new(input), &mut output).unwrap();
    assert_eq!(
        config.start,
        StartPosition {
            base_deg: 10.0,
            axis1_deg: 20.0,
            axis2_deg: 30.0
        }
    );
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Start base angle"), "{output}");
    assert!(output.contains("Start axis 2 angle"), "{output}");
}

#[test]
fn cli_prompt_treats_blank_start_answers_as_homed() {
    let input = b"100\n20\n50\n200\n150\n400\n8\n1\n\n\n\n";
    let mut output = Vec::new();
    let config = prompt_position_with_io(Cursor::new(input), &mut output).unwrap();
    assert!(config.start.is_homed());
    assert_eq!(config.x_mm, 100.0);
}

#[test]
fn cli_prompt_rejects_a_partial_start_position() {
    let input = b"100\n20\n50\n200\n150\n400\n8\n1\n10\n20\n";
    let error = prompt_position_with_io(Cursor::new(input), Vec::new()).unwrap_err();
    assert!(error.to_string().contains("expected 3 angles"), "{error}");
}

#[test]
fn cli_prompt_rejects_a_non_numeric_start_position() {
    let input = b"100\n20\n50\n200\n150\n400\n8\n1\nx\n20\n30\n";
    assert!(prompt_position_with_io(Cursor::new(input), Vec::new()).is_err());
}

#[test]
fn cli_help_text_documents_the_start_position_and_the_site_mode() {
    let text = crate::pretty::help("rustctl");
    assert!(text.contains("--site"), "{text}");
    assert!(text.contains("RUSTCTL_SITE_ADDR"), "{text}");
    assert!(text.contains("start_base_deg"), "{text}");
    assert!(text.contains("start_axis1_deg"), "{text}");
    assert!(text.contains("start_axis2_deg"), "{text}");
    assert!(text.contains("relative"), "{text}");
    assert!(!text.contains("--api"), "{text}");
}

#[test]
fn shell_modes_document_the_optional_start_position() {
    let mut position_output = Vec::new();
    run_position_loop_with_io(Cursor::new(b""), &mut position_output, false).unwrap();
    let position_output = String::from_utf8(position_output).unwrap();
    assert!(position_output.contains("start_base_deg"), "{position_output}");
    assert!(position_output.contains("optional"), "{position_output}");

    let mut raw_output = Vec::new();
    run_raw_loop_with_io(Cursor::new(b""), &mut raw_output).unwrap();
    let raw_output = String::from_utf8(raw_output).unwrap();
    assert!(raw_output.contains("start_base_deg"), "{raw_output}");
    assert!(raw_output.contains("optional"), "{raw_output}");
}


#[test]
fn plan_report_shows_the_start_position_only_for_relative_moves() {
    let solution = ArmSolution {
        theta_base_deg: 0.0,
        theta1_deg: 90.0,
        theta2_deg: 0.0,
        z_eff_mm: 0.0,
    };
    let homed = plan_report(
        &config_from_line(&position_line([0.0, 0.0, 0.0], None)).unwrap(),
        &solution,
    );
    assert!(!homed.contains("Start position"), "{homed}");
    assert!(!homed.contains("Relative travel"), "{homed}");
    assert!(homed.contains("Axis 1: 90.000"), "{homed}");
    assert!(homed.contains("Axis 1: 800"), "{homed}");

    let relative = plan_report(
        &config_from_line(&position_line([0.0, 0.0, 0.0], Some([5.0, 45.0, 0.0]))).unwrap(),
        &solution,
    );
    assert!(
        relative.contains("Start position: base 5.000°, axis 1 45.000°, axis 2 0.000°"),
        "{relative}"
    );
    assert!(
        relative.contains("Relative travel: base -5.000°, axis 1 45.000°, axis 2 0.000°"),
        "{relative}"
    );
    assert!(relative.contains("Axis 1: 400"), "{relative}");
}

#[test]
fn shell_position_loop_rejects_an_incomplete_start_position() {
    let mut output = Vec::new();
    run_position_loop_with_io(Cursor::new(b"100 0 50 200 200 200 16 1 10 20\n"), &mut output, false)
        .unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Command failed"), "{output}");
    assert!(output.contains("8 or 11"), "{output}");
}

#[test]
fn shell_raw_loop_rejects_an_incomplete_start_position() {
    let mut output = Vec::new();
    run_raw_loop_with_io(Cursor::new(b"0 25 30 200 16 1 10 20\n"), &mut output).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("Command failed"), "{output}");
    assert!(output.contains("6 or 9"), "{output}");
}

#[test]
fn cli_prompt_reports_incomplete_input() {
    let error = prompt_position_with_io(Cursor::new(b"100\n"), Vec::new()).unwrap_err();
    assert!(
        error.to_string().contains("invalid digit") || error.to_string().contains("cannot parse")
    );
}

#[test]
fn cli_help_and_dispatch_cover_help_branches() {
    print_help("robot-arm");
    assert!(get_mode(&[]).is_ok());
    assert!(get_mode(&["robot-arm".to_owned(), "--help".to_owned()]).is_ok());
    let error = get_mode(&["robot-arm".to_owned(), "--unknown".to_owned()]).unwrap_err();
    assert!(error.to_string().contains("Unknown mode '--unknown'"));
}

#[test]
fn shell_api_loop_handles_status_empty_line_and_invalid_json() {
    let input = Cursor::new(b"\n{\"command\":\"status\"}\nnot json\n");
    let mut output = Vec::new();
    run_position_loop_with_io(input, &mut output, true).unwrap();
    let output = String::from_utf8(output).unwrap();
    assert!(output.contains("API mode ready"));
    assert!(output.contains("\"status\":\"ready\""));
    assert!(output.contains("invalid JSON"));
}

#[test]
fn shell_loops_exit_cleanly_at_eof() {
    let mut position_output = Vec::new();
    run_position_loop_with_io(Cursor::new(b""), &mut position_output, false).unwrap();
    assert!(
        String::from_utf8(position_output)
            .unwrap()
            .contains("Shell mode")
    );

    let mut raw_output = Vec::new();
    run_raw_loop_with_io(Cursor::new(b""), &mut raw_output).unwrap();
    assert!(String::from_utf8(raw_output).unwrap().contains("Raw mode"));
}

#[test]
fn shell_position_loop_reports_invalid_command() {
    let mut output = Vec::new();
    run_position_loop_with_io(Cursor::new(b"bad command\n"), &mut output, false).unwrap();
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("Command failed")
    );
}

#[test]
fn shell_position_loop_executes_valid_simulation_command() {
    let mut output = Vec::new();
    run_position_loop_with_io(
        Cursor::new(b"100 0 50 200 200 200 16 1\n"),
        &mut output,
        false,
    )
    .unwrap();
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("Command completed")
    );
}

#[test]
fn shell_raw_loop_reports_invalid_command() {
    let mut output = Vec::new();
    run_raw_loop_with_io(Cursor::new(b"bad command\n"), &mut output).unwrap();
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("Command failed")
    );
}

#[test]
fn shell_raw_loop_executes_valid_simulation_command() {
    let mut output = Vec::new();
    run_raw_loop_with_io(Cursor::new(b"0 25 30 200 16 1\n"), &mut output).unwrap();
    assert!(
        String::from_utf8(output)
            .unwrap()
            .contains("Command completed")
    );
}

fn http_post(path: &str, body: &str) -> String {
    let request = format!(
        "POST {path} HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    String::from_utf8(http_exchange(request.as_bytes())).unwrap()
}

#[test]
fn http_status_request_returns_json_success() {
    let response = http_exchange(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let response = String::from_utf8(response).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("Content-Type: application/json"));
    assert!(response.contains("\"status\":\"ready\""));
}

#[test]
fn http_post_args_returns_success() {
    let _serial = busy_test_lock();
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let request = format!(
        "POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );
    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"));
    assert!(response.contains("\"mode\":\"args\""));
}

#[test]
fn http_post_args_with_a_start_position_succeeds() {
    let _serial = busy_test_lock();
    let response = http_post(
        "/args",
        r#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true,"start_base_deg":5,"start_axis1_deg":10,"start_axis2_deg":15}"#,
    );
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("\"mode\":\"args\""), "{response}");
}

#[test]
fn http_post_raw_with_a_start_position_succeeds() {
    let _serial = busy_test_lock();
    let response = http_post(
        "/raw",
        r#"{"base_deg":0,"axis1_deg":25,"axis2_deg":30,"steps_per_rev":200,"microstep":16,"ccw_positive":true,"start_base_deg":5,"start_axis1_deg":10,"start_axis2_deg":15}"#,
    );
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("\"mode\":\"raw\""), "{response}");
}

#[test]
fn http_post_args_with_a_partial_start_position_is_rejected() {
    let response = http_post(
        "/args",
        r#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true,"start_base_deg":5}"#,
    );
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
    assert!(response.contains("all of start_base_deg"), "{response}");
}

#[test]
fn http_get_help_documents_the_start_position() {
    let response =
        String::from_utf8(http_exchange(b"GET /help HTTP/1.1\r\nHost: localhost\r\n\r\n")).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    for field in ["start_base_deg", "start_axis1_deg", "start_axis2_deg"] {
        assert!(response.contains(field), "help must mention {field}: {response}");
    }
}

#[test]
fn http_routes_return_expected_error_statuses() {
    let not_found = http_exchange(b"GET /missing HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(
        String::from_utf8(not_found)
            .unwrap()
            .starts_with("HTTP/1.1 404 Not Found\r\n")
    );

    let method = http_exchange(b"PUT /status HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert!(
        String::from_utf8(method)
            .unwrap()
            .starts_with("HTTP/1.1 405 Method Not Allowed\r\n")
    );

    let bad_json = http_exchange(
        b"POST /api HTTP/1.1\r\nHost: localhost\r\nContent-Length: 8\r\n\r\nnot-json",
    );
    assert!(
        String::from_utf8(bad_json)
            .unwrap()
            .starts_with("HTTP/1.1 400 Bad Request\r\n")
    );
}

#[test]
fn http_malformed_requests_return_bad_request() {
    let response = http_exchange(b"not an HTTP request\r\n\r\n");
    let response = String::from_utf8(response).unwrap();
    assert!(response.starts_with("HTTP/1.1 400 Bad Request\r\n"));
    assert!(response.contains("unsupported HTTP version"));
}

#[test]
fn http_rejects_unsupported_version_and_invalid_content_length() {
    let version = http_exchange(b"GET /status HTTP/2.0\r\nHost: localhost\r\n\r\n");
    assert!(
        String::from_utf8(version)
            .unwrap()
            .contains("unsupported HTTP version")
    );

    let length =
        http_exchange(b"POST /api HTTP/1.1\r\nHost: localhost\r\nContent-Length: nope\r\n\r\n");
    assert!(
        String::from_utf8(length)
            .unwrap()
            .contains("invalid Content-Length")
    );
}

#[test]
fn cli_help_text_describes_site_mode_not_api() {
    let text = crate::pretty::help("rustctl");
    assert!(text.contains("--site"));
    assert!(text.contains("RUSTCTL_SITE_ADDR"));
    assert!(!text.contains("--api"));
}

#[test]
fn cli_rejects_removed_api_flag_with_helpful_message() {
    let error = get_mode(&["rustctl".to_owned(), "--api".to_owned()]).unwrap_err();
    assert!(error.to_string().contains("renamed to '--site'"), "{error}");
}

#[test]
fn control_second_acquire_fails_until_released() {
    let _serial = busy_test_lock();
    assert!(!is_busy());
    let first = BusyGuard::acquire().expect("first acquire should succeed");
    assert!(is_busy());
    assert!(
        BusyGuard::acquire().is_none(),
        "a second acquire must fail while a motion is in progress"
    );
    drop(first);
    assert!(!is_busy());
    let second = BusyGuard::acquire().expect("acquire should succeed again after release");
    drop(second);
    assert!(!is_busy());
}

#[test]
fn control_guard_releases_the_flag_even_after_a_panic() {
    let _serial = busy_test_lock();
    assert!(!is_busy());
    let result = std::panic::catch_unwind(|| {
        let _guard = BusyGuard::acquire().unwrap();
        assert!(is_busy());
        panic!("simulated failure while a move is in progress");
    });
    assert!(result.is_err());
    assert!(
        !is_busy(),
        "the guard must release the flag during unwind, not just on a normal return"
    );
}

#[test]
fn origin_absent_is_always_trusted() {
    assert!(origin_is_trusted(None, None));
    assert!(origin_is_trusted(None, Some("localhost:5000")));
}

#[test]
fn origin_matching_host_is_trusted_regardless_of_scheme_or_case() {
    assert!(origin_is_trusted(
        Some("http://localhost:5000"),
        Some("localhost:5000")
    ));
    assert!(origin_is_trusted(
        Some("https://LOCALHOST:5000"),
        Some("localhost:5000")
    ));
    assert!(origin_is_trusted(
        Some("http://192.168.1.50:5000"),
        Some("192.168.1.50:5000")
    ));
}

#[test]
fn origin_mismatched_host_is_rejected() {
    assert!(!origin_is_trusted(
        Some("http://evil.example"),
        Some("localhost:5000")
    ));
    assert!(!origin_is_trusted(
        Some("http://localhost:5000"),
        Some("localhost:5001")
    ));
}

#[test]
fn origin_without_scheme_or_without_host_header_is_rejected() {
    assert!(!origin_is_trusted(
        Some("localhost:5000"),
        Some("localhost:5000")
    ));
    assert!(!origin_is_trusted(Some("http://localhost:5000"), None));
    assert!(!origin_is_trusted(Some("null"), Some("localhost:5000")));
}

#[test]
fn req_oversized_headers_are_rejected() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n")
        .unwrap();
    client.write_all(&vec![b'x'; 17 * 1024]).unwrap();
    let result = server.join().unwrap();
    match result {
        Err(RequestError::Malformed(message)) => {
            assert!(message.contains("too large"), "{message}")
        }
        other => panic!("expected a Malformed(too large) error, got {other:?}"),
    }
}

#[test]
fn req_content_length_over_one_mib_is_rejected_before_reading_the_body() {
    let response = String::from_utf8(http_exchange(
        b"POST /api HTTP/1.1\r\nHost: localhost\r\nContent-Length: 5000000\r\n\r\n",
    ))
    .unwrap();
    assert!(
        response.starts_with("HTTP/1.1 400 Bad Request\r\n"),
        "{response}"
    );
    assert!(response.contains("too large"));
}

#[test]
fn req_body_delivered_across_multiple_reads_is_reassembled() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let head = format!(
        "POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n",
        body.len()
    );
    let split_at = body.len() / 2;
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let mut client = TcpStream::connect(address).unwrap();
    client.write_all(head.as_bytes()).unwrap();
    client.write_all(&body[..split_at]).unwrap();
    thread::sleep(Duration::from_millis(80));
    client.write_all(&body[split_at..]).unwrap();
    let request = server.join().unwrap().expect("request should parse");
    assert_eq!(request.method, "POST");
    assert_eq!(request.body, std::str::from_utf8(body).unwrap());
}

#[test]
fn req_body_closing_early_is_reported_as_malformed() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(
            b"POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\n{\"partial",
        )
        .unwrap();
    client.shutdown(Shutdown::Both).unwrap();
    let result = server.join().unwrap();
    match result {
        Err(RequestError::Malformed(message)) => {
            assert!(message.contains("before request body"), "{message}")
        }
        other => panic!("expected a Malformed(before request body) error, got {other:?}"),
    }
}

#[test]
fn req_body_read_can_time_out_independently_of_the_header_read() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .write_all(
            b"POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: 100\r\n\r\n{\"partial",
        )
        .unwrap();
    let result = server.join().unwrap();
    assert_eq!(result.unwrap_err(), RequestError::Timeout);
    drop(client);
}

#[test]
fn req_read_http_request_times_out_without_data() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        read_http_request(&mut stream)
    });
    let _client = TcpStream::connect(address).unwrap();
    let result = server.join().unwrap();
    assert_eq!(result.unwrap_err(), RequestError::Timeout);
}

#[test]
fn req_handle_request_maps_timeout_to_408() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    let server = thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(Duration::from_millis(150)))
            .unwrap();
        stream
            .set_write_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        handle_request(&mut stream).unwrap();
    });
    let mut client = TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    let mut response = Vec::new();
    client.read_to_end(&mut response).unwrap();
    server.join().unwrap();
    let response = String::from_utf8(response).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 408 Request Timeout\r\n"),
        "{response}"
    );
    assert!(response.contains("request timed out"));
}

fn spawn_accept_loop() -> SocketAddr {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let address = listener.local_addr().unwrap();
    thread::spawn(move || {
        for stream in listener.incoming() {
            if let Ok(stream) = stream {
                thread::spawn(move || {
                    let _ = handle_connection(stream);
                });
            }
        }
    });
    address
}

fn read_full_response(stream: &mut TcpStream) -> String {
    let mut response = Vec::new();
    stream.read_to_end(&mut response).unwrap();
    String::from_utf8(response).unwrap()
}

#[test]
fn con_second_connection_served_while_first_is_idle() {
    let address = spawn_accept_loop();
    let _idle = TcpStream::connect(address).unwrap();

    let mut b = TcpStream::connect(address).unwrap();
    b.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    b.write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let response = read_full_response(&mut b);
    assert!(
        response.starts_with("HTTP/1.1 200 OK\r\n"),
        "a second client must be served promptly while the first is idle: {response}"
    );
}

#[test]
fn con_accept_loop_survives_abrupt_client_disconnect() {
    let address = spawn_accept_loop();
    {
        let mut a = TcpStream::connect(address).unwrap();
        a.write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n")
            .unwrap();
        a.shutdown(Shutdown::Both).ok();
    }
    thread::sleep(Duration::from_millis(150));
    let mut b = TcpStream::connect(address).unwrap();
    b.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
    b.write_all(b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n")
        .unwrap();
    let response = read_full_response(&mut b);
    assert!(
        response.starts_with("HTTP/1.1 200 OK\r\n"),
        "the accept loop must keep serving requests after another client disconnects abruptly: {response}"
    );
}

#[test]
fn con_cross_origin_post_is_rejected() {
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let request = format!(
        "POST /args HTTP/1.1\r\nHost: localhost:5000\r\nOrigin: http://evil.example\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );
    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 403 Forbidden\r\n"),
        "{response}"
    );
    assert!(response.contains("cross-origin request refused"));
}

#[test]
fn con_same_origin_post_is_allowed() {
    let _serial = busy_test_lock();
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let request = format!(
        "POST /args HTTP/1.1\r\nHost: localhost:5000\r\nOrigin: http://localhost:5000\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );
    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("\"mode\":\"args\""));
}

#[test]
fn con_get_ignores_origin_header() {
    let request =
        b"GET /status HTTP/1.1\r\nHost: localhost:5000\r\nOrigin: http://evil.example\r\n\r\n";
    let response = String::from_utf8(http_exchange(request)).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
}

#[test]
fn con_busy_arm_rejects_concurrent_motion_request_with_409() {
    let _serial = busy_test_lock();
    let body = br#"{"radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#;
    let request = format!(
        "POST /args HTTP/1.1\r\nHost: localhost\r\nContent-Length: {}\r\n\r\n{}",
        body.len(),
        std::str::from_utf8(body).unwrap()
    );

    let guard = BusyGuard::acquire().unwrap();
    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(
        response.starts_with("HTTP/1.1 409 Conflict\r\n"),
        "{response}"
    );
    assert!(response.contains("arm is busy"));
    assert!(response.contains("\"busy\":true"));
    drop(guard);

    let response = String::from_utf8(http_exchange(request.as_bytes())).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
}

#[test]
fn con_status_reports_busy_field() {
    let _serial = busy_test_lock();
    let idle = String::from_utf8(http_exchange(
        b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ))
    .unwrap();
    assert!(idle.contains("\"busy\":false"), "{idle}");

    let guard = BusyGuard::acquire().unwrap();
    let busy = String::from_utf8(http_exchange(
        b"GET /status HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ))
    .unwrap();
    assert!(busy.contains("\"busy\":true"), "{busy}");
    drop(guard);
}

#[test]
fn page_html_constant_is_self_contained_with_no_external_resources() {
    let lower = PAGE_HTML.to_ascii_lowercase();
    assert!(
        !lower.contains("stylesheet"),
        "page must not link an external stylesheet"
    );
    assert!(
        !lower.contains(".css"),
        "page must not reference a CSS file"
    );
    assert!(
        !PAGE_HTML.contains("http://") && !PAGE_HTML.contains("https://"),
        "page must be self-contained: no external URLs"
    );
    assert!(!lower.contains("innerhtml"), "page must not use innerHTML");
    assert!(!lower.contains("eval("), "page must not use eval()");
    assert!(!lower.contains("document.write"));
    for element in PAGE_HTML.lines() {
        assert!(
            !element.trim_start().starts_with("style="),
            "page must not use inline style attributes: {element}"
        );
    }
}


fn page_style_block() -> &'static str {
    let start = PAGE_HTML.find("<style>").expect("page must ship its own CSS");
    let body = &PAGE_HTML[start + "<style>".len()..];
    let end = body.find("</style>").expect("the style block must be closed");
    &body[..end]
}

#[test]
fn page_styles_are_scoped_to_the_win95_theme() {
    let css = page_style_block();
    let selectors: Vec<&str> = css
        .lines()
        .map(str::trim)
        .filter(|line| line.ends_with('{') && line.contains('{') && !line.starts_with('@'))
        .collect();
    assert!(
        selectors.len() > 10,
        "expected a full set of theme rules, got {selectors:?}"
    );
    for selector in &selectors {
        assert!(
            selector.starts_with("html[data-theme=\"win95\"]"),
            "every rule must be scoped to the win95 theme so the plain theme stays CSS-less: {selector}"
        );
    }
    for selector in &selectors {
        assert!(
            !selector.contains("\"plain\""),
            "the plain theme must not be styled at all: {selector}"
        );
    }
}

#[test]
fn page_offers_a_theme_toggle_for_both_themes() {
    assert!(
        PAGE_HTML.contains(r#"data-theme="win95""#),
        "the page must declare the win95 theme as its default"
    );
    assert!(
        PAGE_HTML.contains(r#"id="theme""#),
        "page must have a theme toggle control"
    );
    for token in ["THEME_KEY", "applyTheme", "localStorage", "rustctl-theme"] {
        assert!(PAGE_HTML.contains(token), "theme persistence is missing {token}");
    }
    assert!(
        PAGE_HTML.contains("\"CSS-less\""),
        "the toggle must offer the CSS-less theme by name"
    );
}

#[test]
fn page_form_fields_match_the_json_api_field_names() {
    for field in [
        "radius_mm",
        "base_angle_deg",
        "height_mm",
        "l1_mm",
        "l2_mm",
        "steps_per_rev",
        "microstep",
        "ccw_positive",
        "base_deg",
        "axis1_deg",
        "axis2_deg",
        "start_base_deg",
        "start_axis1_deg",
        "start_axis2_deg",
    ] {
        assert!(
            PAGE_HTML.contains(&format!("name=\"{field}\"")),
            "page is missing a field for {field}"
        );
    }
}

#[test]
fn page_offers_a_start_position_in_both_motion_forms() {
    for (id, path) in [("args", "/args"), ("raw", "/raw")] {
        let start = PAGE_HTML
            .find(&format!("id=\"{id}\""))
            .unwrap_or_else(|| panic!("page must contain the {id} form"));
        let end = PAGE_HTML[start..]
            .find("</form>")
            .map(|offset| start + offset)
            .expect("form must be closed");
        let form = &PAGE_HTML[start..end];
        assert!(form.contains(&format!("data-path=\"{path}\"")), "{id}");
        for field in ["start_base_deg", "start_axis1_deg", "start_axis2_deg"] {
            assert!(
                form.contains(&format!("name=\"{field}\"")),
                "the {id} form is missing {field}"
            );
        }
    }
}

#[test]
fn page_omits_an_all_zero_start_position_from_the_request() {
    assert!(
        PAGE_HTML.contains(r#"el.name.startsWith("start_")"#),
        "the page must drop all-zero start angles so the controller stays homed"
    );
}

#[test]
fn page_root_serves_html_with_expected_headers() {
    let response =
        String::from_utf8(http_exchange(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n")).unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
    assert!(response.contains("Content-Type: text/html"));
    assert!(response.contains("Connection: close"));
    assert!(response.contains("<h1>Roboterarm HASE</h1>"));
}

#[test]
fn page_index_html_alias_serves_identical_content() {
    let root = http_exchange(b"GET / HTTP/1.1\r\nHost: localhost\r\n\r\n");
    let alias = http_exchange(b"GET /index.html HTTP/1.1\r\nHost: localhost\r\n\r\n");
    assert_eq!(root, alias);
}

#[test]
fn page_query_string_is_ignored() {
    let response = String::from_utf8(http_exchange(
        b"GET /?debug=1 HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ))
    .unwrap();
    assert!(response.starts_with("HTTP/1.1 200 OK\r\n"), "{response}");
}

#[test]
fn page_other_unknown_paths_still_404() {
    let response = String::from_utf8(http_exchange(
        b"GET /favicon.ico HTTP/1.1\r\nHost: localhost\r\n\r\n",
    ))
    .unwrap();
    assert!(
        response.starts_with("HTTP/1.1 404 Not Found\r\n"),
        "{response}"
    );
}

#[test]
fn page_post_to_root_is_not_treated_as_the_page_route() {
    let response = String::from_utf8(http_exchange(
        b"POST / HTTP/1.1\r\nHost: localhost\r\nContent-Length: 0\r\n\r\n",
    ))
    .unwrap();
    assert!(!response.starts_with("HTTP/1.1 200 OK\r\n"));
}

fn addr(interface: &str, ip: [u8; 4]) -> Address {
    Address {
        interface: interface.to_owned(),
        ip: Ipv4Addr::from(ip),
    }
}

#[test]
fn net_filters_loopback_and_virtual_interfaces() {
    let ranked = rank_addresses(vec![
        addr("lo", [127, 0, 0, 1]),
        addr("docker0", [172, 17, 0, 1]),
        addr("veth1234", [172, 18, 0, 1]),
        addr("br-abcdef", [172, 19, 0, 1]),
        addr("virbr0", [192, 168, 122, 1]),
        addr("eth0", [192, 168, 1, 50]),
    ]);
    assert_eq!(ranked, vec![addr("eth0", [192, 168, 1, 50])]);
}

#[test]
fn net_ranks_wired_before_wireless_before_usb_before_other() {
    let ranked = rank_addresses(vec![
        addr("wlan0", [192, 168, 1, 20]),
        addr("usb0", [192, 168, 2, 20]),
        addr("eth0", [192, 168, 1, 50]),
        addr("tun0", [10, 0, 0, 5]),
    ]);
    let order: Vec<&str> = ranked.iter().map(|a| a.interface.as_str()).collect();
    assert_eq!(order, vec!["eth0", "wlan0", "usb0", "tun0"]);
}

#[test]
fn net_dedups_duplicate_ip_addresses() {
    let ranked = rank_addresses(vec![
        addr("eth0", [192, 168, 1, 50]),
        addr("eth0:1", [192, 168, 1, 50]),
    ]);
    assert_eq!(ranked.len(), 1);
}

#[test]
fn net_empty_input_yields_empty_output() {
    assert!(rank_addresses(vec![]).is_empty());
}
