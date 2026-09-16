use super::*;

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
    assert_eq!(report.failed, 0);
    assert_eq!(report.passed, 4);
    assert!(report.failures.is_empty());
}

fn approx(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-6
}

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
        assert!(approx(z_eff, height), "height mismatch: {z_eff} vs {height}");
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