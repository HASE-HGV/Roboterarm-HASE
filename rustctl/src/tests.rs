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

#[test]
fn config_parses_position_values_and_fixed_timing() {
    let config = config_from_position(&[
        "100", "-20.5", "50", "200", "150", "400", "8", "1",
    ])
    .unwrap();
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
    for name in ["status", "STATUS", "help", "HELP", "test", "tests", "quit", "exit"] {
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
    assert!(parse_api_request("not json", None)
        .unwrap_err()
        .starts_with("invalid JSON:"));
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
    let fields = ["base_deg", "axis1_deg", "axis2_deg", "steps_per_rev", "microstep", "ccw_positive"];
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
    assert!(status["commands"].as_array().unwrap().contains(&serde_json::json!("raw")));

    let help = api_command_response(ApiCommand::Help);
    assert_eq!(help["ok"], true);
    assert!(help["help"].as_str().unwrap().contains("JSON commands"));

    let quit = api_command_response(ApiCommand::Quit);
    assert_eq!(quit, serde_json::json!({"ok": true, "status": "bye", "reason": "client"}));
}

#[test]
fn api_test_response_contains_runtime_report() {
    let response = api_command_response(ApiCommand::Test);
    assert_eq!(response["ok"], true);
    assert_eq!(response["status"], "tests_completed");
    assert_eq!(response["tests"]["failed"], 0);
    assert_eq!(response["tests"]["passed"], 4);
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
