use super::*;

#[test]
fn a_domain_label() {
    announce_domain("API and runtime checks");
}

#[test]
fn api_parses_args_mode() {
    let command = parse_api_request(
        r#"{"command":"args","radius_mm":100,"base_angle_deg":20,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#,
        None,
    )
    .unwrap();
    match command {
        ApiCommand::Args(config) => assert_eq!(
            config,
            MotionConfig::new(100.0, 20.0, 50.0, 200.0, 200.0, 200, 16, true)
        ),
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
        ApiCommand::Raw(config, solution) => assert_eq!(
            (config.steps_per_rev, config.microstep, config.ccw_positive, solution),
            (
                400,
                8,
                false,
                ArmSolution {
                    theta_base_deg: -10.0,
                    theta1_deg: 25.0,
                    theta2_deg: 30.0,
                    z_eff_mm: 0.0,
                }
            )
        ),
        other => panic!("expected raw command, got {other:?}"),
    }
}

#[test]
fn api_parses_status_command() {
    assert_eq!(
        parse_api_request(r#"{"command":"status"}"#, None),
        Ok(ApiCommand::Status)
    );
}

#[test]
fn api_parses_help_command() {
    assert_eq!(
        parse_api_request(r#"{"command":"help"}"#, None),
        Ok(ApiCommand::Help)
    );
}

#[test]
fn api_parses_test_command() {
    assert_eq!(
        parse_api_request(r#"{"command":"test"}"#, None),
        Ok(ApiCommand::Test)
    );
}

#[test]
fn api_parses_quit_command() {
    assert_eq!(
        parse_api_request(r#"{"command":"quit"}"#, None),
        Ok(ApiCommand::Quit)
    );
}

#[test]
fn api_rejects_incomplete_args_command() {
    assert!(parse_api_request(r#"{"command":"args","radius_mm":100}"#, None).is_err());
}

#[test]
fn api_rejects_incomplete_raw_command() {
    assert!(parse_api_request(r#"{"command":"raw","base_deg":0,"axis1_deg":0,"axis2_deg":0,"steps_per_rev":200,"microstep":16}"#, None).is_err());
}

#[test]
fn api_rejects_unknown_command() {
    assert!(parse_api_request(r#"{"command":"unknown"}"#, None).is_err());
}

#[test]
fn api_rejects_empty_request_body() {
    assert!(parse_api_request("", None).is_err());
}

#[test]
fn api_rejects_unrecognized_command_name() {
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

#[test]
fn runtime_check_report_collects_failure_details() {
    let checks: [(&str, fn() -> Result<(), String>); 2] = [
        ("passing_check", || Ok(())),
        ("failing_check", || Err("expected synthetic failure".to_owned())),
    ];
    let report = report_runtime_checks(&checks);
    assert_eq!(report.passed, 1);
    assert_eq!(report.failed, 1);
    assert_eq!(report.checks, vec!["passing_check", "failing_check"]);
    assert_eq!(report.failures.len(), 1);
    assert_eq!(report.failures[0].function, "failing_check");
    assert_eq!(report.failures[0].message, "expected synthetic failure");
}

#[test]
fn runtime_ik_roundtrip_validator_accepts_expected_target() {
    assert!(validate_ik_roundtrip(150.0, 40.0).is_ok());
}

#[test]
fn runtime_ik_roundtrip_validator_reports_wrong_radius() {
    assert!(
        validate_ik_roundtrip(151.0, 40.0)
            .unwrap_err()
            .contains("radius=151")
    );
}

#[test]
fn runtime_step_conversion_validator_accepts_expected_step_count() {
    assert!(validate_step_conversion(3200).is_ok());
}

#[test]
fn runtime_step_conversion_validator_reports_wrong_step_count() {
    assert!(validate_step_conversion(3199).unwrap_err().contains("got 3199"));
}

#[test]
fn runtime_timing_validator_accepts_rejected_timing() {
    assert!(validate_timing_validation(Err("too fast".to_owned())).is_ok());
}

#[test]
fn runtime_timing_validator_rejects_accepted_invalid_timing() {
    assert_eq!(
        validate_timing_validation(Ok(7)).unwrap_err(),
        "accepted invalid timing and returned 7"
    );
}

#[test]
fn runtime_planner_validator_accepts_expected_totals() {
    assert!(validate_planner_totals([10, 5, 0]).is_ok());
}

#[test]
fn runtime_planner_validator_reports_wrong_totals() {
    assert!(validate_planner_totals([10, 4, 0])
        .unwrap_err()
        .contains("[10, 4, 0]"));
}

#[test]
fn runtime_relative_plan_validator_accepts_shorter_relative_motion() {
    assert!(validate_relative_plan([20.0, 15.0, 20.0], [4, 2, 4], [2, 1, 2]).is_ok());
}

#[test]
fn runtime_relative_plan_validator_rejects_wrong_joint_deltas() {
    assert!(validate_relative_plan([0.0, 0.0, 0.0], [4, 2, 4], [2, 1, 2])
        .unwrap_err()
        .contains("joint deltas"));
}

#[test]
fn runtime_relative_plan_validator_rejects_unchanged_step_counts() {
    assert!(validate_relative_plan([20.0, 15.0, 20.0], [1, 2, 3], [1, 0, 1])
        .unwrap_err()
        .contains("did not differ"));
}

#[test]
fn runtime_relative_plan_validator_rejects_longer_relative_steps() {
    assert!(validate_relative_plan([20.0, 15.0, 20.0], [10, 10, 10], [-20, -20, -20])
        .unwrap_err()
        .contains("not shorter"));
}

#[test]
fn runtime_ik_solution_accepts_positive_link_lengths() {
    assert!(runtime_ik_solution(150.0, 30.0, 40.0, 120.0, 90.0).is_ok());
}

#[test]
fn runtime_ik_solution_rejects_zero_link_length() {
    assert_eq!(
        runtime_ik_solution(150.0, 30.0, 40.0, 0.0, 90.0).unwrap_err(),
        "Link lengths must be positive"
    );
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
fn api_position_rejects_a_partial_start_position() {
    let args = r#"{"command":"args","radius_mm":1,"base_angle_deg":2,"height_mm":3,"l1_mm":4,"l2_mm":5,"steps_per_rev":200,"microstep":8,"ccw_positive":true,"start_base_deg":1}"#;
    let error = parse_api_request(args, None).unwrap_err();
    assert!(error.contains("all of start_base_deg"), "{error}");
}

#[test]
fn api_raw_rejects_a_partial_start_position() {
    let raw = r#"{"command":"raw","base_deg":1,"axis1_deg":2,"axis2_deg":3,"steps_per_rev":200,"microstep":8,"ccw_positive":true,"start_axis1_deg":1,"start_axis2_deg":2}"#;
    let error = parse_api_request(raw, None).unwrap_err();
    assert!(error.contains("all of start_base_deg"), "{error}");
}

#[test]
fn api_start_position_must_be_numeric() {
    assert!(
        parse_api_request(
            r#"{"command":"raw","base_deg":0,"axis1_deg":0,"axis2_deg":0,"steps_per_rev":200,"microstep":8,"ccw_positive":true,"start_base_deg":"x","start_axis1_deg":1,"start_axis2_deg":2}"#,
            None
        )
        .is_err()
    );
}

#[test]
fn api_help_documents_all_start_position_fields() {
    let help = api_help();
    assert!(
        ["start_base_deg", "start_axis1_deg", "start_axis2_deg", "relative"]
            .iter()
            .all(|phrase| help.contains(phrase)),
        "{help}"
    );
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
            r#"{"radius_mm":1,"base_angle_deg":2,"height_mm":3,"l1_mm":4,"l2_mm":5,"steps_per_rev":200,"microstep":8,"ccw_positive":true}"#,
            Some("ARGS")
        ),
        Ok(ApiCommand::Args(_))
    ));
}

#[test]
fn api_rejects_invalid_json() {
    assert!(
        parse_api_request("not json", None)
            .unwrap_err()
            .starts_with("invalid JSON:")
    );
}

#[test]
fn api_rejects_missing_command_field() {
    assert_eq!(
        parse_api_request("{}", None).unwrap_err(),
        "missing JSON field 'command'"
    );
}

#[test]
fn api_rejects_null_command_field() {
    assert!(parse_api_request(r#"{"command":null}"#, None).is_err());
}

#[test]
fn api_rejects_non_string_command_field() {
    assert!(parse_api_request(r#"{"command":123}"#, None).is_err());
}

#[test]
fn api_rejects_missing_position_radius() {
    assert_eq!(
        parse_api_request(&args_request_without("radius_mm"), None).unwrap_err(),
        "missing radius_mm"
    );
}

#[test]
fn api_rejects_missing_position_base_angle() {
    assert_eq!(
        parse_api_request(&args_request_without("base_angle_deg"), None).unwrap_err(),
        "missing base_angle_deg"
    );
}

#[test]
fn api_rejects_missing_position_height() {
    assert_eq!(
        parse_api_request(&args_request_without("height_mm"), None).unwrap_err(),
        "missing height_mm"
    );
}

#[test]
fn api_rejects_missing_position_first_link_length() {
    assert_eq!(
        parse_api_request(&args_request_without("l1_mm"), None).unwrap_err(),
        "missing l1_mm"
    );
}

#[test]
fn api_rejects_missing_position_second_link_length() {
    assert_eq!(
        parse_api_request(&args_request_without("l2_mm"), None).unwrap_err(),
        "missing l2_mm"
    );
}

#[test]
fn api_rejects_missing_position_steps_per_revolution() {
    assert_eq!(
        parse_api_request(&args_request_without("steps_per_rev"), None).unwrap_err(),
        "missing steps_per_rev"
    );
}

#[test]
fn api_rejects_missing_position_microstep() {
    assert_eq!(
        parse_api_request(&args_request_without("microstep"), None).unwrap_err(),
        "missing microstep"
    );
}

#[test]
fn api_rejects_missing_position_direction_flag() {
    assert_eq!(
        parse_api_request(&args_request_without("ccw_positive"), None).unwrap_err(),
        "missing ccw_positive"
    );
}

#[test]
fn api_rejects_missing_raw_base_angle() {
    assert_eq!(
        parse_api_request(&raw_request_without("base_deg"), None).unwrap_err(),
        "missing base_deg"
    );
}

#[test]
fn api_rejects_missing_raw_shoulder_angle() {
    assert_eq!(
        parse_api_request(&raw_request_without("axis1_deg"), None).unwrap_err(),
        "missing axis1_deg"
    );
}

#[test]
fn api_rejects_missing_raw_elbow_angle() {
    assert_eq!(
        parse_api_request(&raw_request_without("axis2_deg"), None).unwrap_err(),
        "missing axis2_deg"
    );
}

#[test]
fn api_rejects_missing_raw_steps_per_revolution() {
    assert_eq!(
        parse_api_request(&raw_request_without("steps_per_rev"), None).unwrap_err(),
        "missing steps_per_rev"
    );
}

#[test]
fn api_rejects_missing_raw_microstep() {
    assert_eq!(
        parse_api_request(&raw_request_without("microstep"), None).unwrap_err(),
        "missing microstep"
    );
}

#[test]
fn api_rejects_missing_raw_direction_flag() {
    assert_eq!(
        parse_api_request(&raw_request_without("ccw_positive"), None).unwrap_err(),
        "missing ccw_positive"
    );
}

#[test]
fn api_status_response_includes_ready_status_and_commands() {
    let status = api_command_response(ApiCommand::Status);
    assert_eq!(
        (
            status["ok"].as_bool(),
            status["status"].as_str(),
            status["commands"]
                .as_array()
                .is_some_and(|commands| commands.contains(&serde_json::json!("raw")))
        ),
        (Some(true), Some("ready"), true)
    );
}

#[test]
fn api_help_response_contains_command_documentation() {
    let help = api_command_response(ApiCommand::Help);
    assert!(
        help["ok"].as_bool() == Some(true)
            && help["help"]
                .as_str()
                .is_some_and(|text| text.contains("JSON commands"))
    );
}

#[test]
fn api_quit_response_reports_client_bye() {
    assert_eq!(
        api_command_response(ApiCommand::Quit),
        serde_json::json!({"ok": true, "status": "bye", "reason": "client"})
    );
}

#[test]
fn api_test_response_reports_successful_runtime_checks() {
    let response = api_command_response(ApiCommand::Test);
    assert_eq!(
        (
            response["ok"].as_bool(),
            response["status"].as_str(),
            response["tests"]["failed"].as_u64(),
            response["tests"]["passed"].as_u64(),
            response["tests"]["failures"]
                .as_array()
                .map(std::vec::Vec::len)
        ),
        (Some(true), Some("tests_completed"), Some(0), Some(5), Some(0))
    );
}

#[test]
fn api_args_response_reports_successful_execution() {
    let args = parse_api_request(
        r#"{"command":"args","radius_mm":100,"base_angle_deg":0,"height_mm":50,"l1_mm":200,"l2_mm":200,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#,
        None,
    )
    .unwrap();
    let args_response = api_command_response(args);
    assert_eq!(
        (args_response["ok"].as_bool(), args_response["mode"].as_str()),
        (Some(true), Some("args"))
    );
}

#[test]
fn api_raw_response_reports_successful_execution() {
    let raw = parse_api_request(
        r#"{"command":"raw","base_deg":0,"axis1_deg":25,"axis2_deg":30,"steps_per_rev":200,"microstep":16,"ccw_positive":true}"#,
        None,
    )
    .unwrap();
    let response = api_command_response(raw);
    assert_eq!(
        (response["ok"].as_bool(), response["mode"].as_str()),
        (Some(true), Some("raw"))
    );
}

#[test]
fn api_args_response_maps_inverse_kinematics_failure() {
    let mut invalid_position = MotionConfig::new(-1.0, 0.0, 0.0, 100.0, 100.0, 200, 16, true);
    invalid_position.total_time_us = 1083;
    let args_response = api_command_response(ApiCommand::Args(invalid_position));
    assert!(
        args_response["ok"].as_bool() == Some(false)
            && args_response["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("Radius X"))
    );
}

#[test]
fn api_raw_response_maps_timing_failure() {
    let mut invalid_timing = MotionConfig::new(0.0, 0.0, 0.0, 1.0, 1.0, 200, 16, true);
    invalid_timing.total_time_us = 0;
    let raw_response = api_command_response(ApiCommand::Raw(
        invalid_timing,
        ArmSolution {
            theta_base_deg: 0.0,
            theta1_deg: 0.0,
            theta2_deg: 0.0,
            z_eff_mm: 0.0,
        },
    ));
    assert!(
        raw_response["ok"].as_bool() == Some(false)
            && raw_response["error"]["message"]
                .as_str()
                .is_some_and(|message| message.contains("total_time"))
    );
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
fn api_position_rejects_zero_steps_per_revolution() {
    let body = args_request_with_motor_settings(0, 1);
    assert!(parse_api_request(&body, None).is_err());
}

#[test]
fn api_position_rejects_unsupported_microstep() {
    let body = args_request_with_motor_settings(200, 3);
    assert!(parse_api_request(&body, None).is_err());
}

#[test]
fn api_raw_rejects_zero_steps_per_revolution() {
    let body = raw_request_with_motor_settings(0, 1);
    assert!(parse_api_request(&body, None).is_err());
}

#[test]
fn api_raw_rejects_unsupported_microstep() {
    let body = raw_request_with_motor_settings(200, 32);
    assert!(parse_api_request(&body, None).is_err());
}

#[test]
fn api_position_accepts_each_a4988_microstep_value() {
    for microstep in [1, 2, 4, 8, 16] {
        let body = args_request_with_motor_settings(200, microstep);
        assert!(parse_api_request(&body, None).is_ok(), "{microstep}");
    }
}

#[test]
fn api_raw_accepts_each_a4988_microstep_value() {
    for microstep in [1, 2, 4, 8, 16] {
        let body = raw_request_with_motor_settings(200, microstep);
        assert!(parse_api_request(&body, None).is_ok(), "{microstep}");
    }
}
