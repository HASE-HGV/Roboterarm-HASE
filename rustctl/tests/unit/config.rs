use super::*;

#[test]
fn a_domain_label() {
    announce_domain("Configuration parsing");
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
}

#[test]
fn position_command_rejects_a_partial_start_position() {
    let values = [
        "100", "0", "50", "200", "200", "200", "16", "1", "10", "20",
    ];
    let error = config_from_position(&values).unwrap_err().to_string();
    assert!(error.contains("8 or 11"), "{error}");
}

#[test]
fn position_line_rejects_a_partial_start_position() {
    let error = config_from_line("100 0 50 200 200 200 16 1 10 20")
        .unwrap_err()
        .to_string();
    assert!(error.contains("8 or 11"), "{error}");
}

#[test]
fn position_argument_count_errors_name_the_expected_counts() {
    let error = config_from_position(&["1", "2", "3"]).unwrap_err().to_string();
    assert!(
        error.contains("8 or 11") && error.contains("got 3"),
        "{error}"
    );
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
fn raw_command_rejects_a_partial_start_position() {
    let error = raw_command("0 25 30 200 16 1 10 20").unwrap_err().to_string();
    assert!(error.contains("6 or 9"), "{error}");
}

#[test]
fn raw_command_rejects_a_non_numeric_start_angle() {
    assert!(raw_command("0 25 30 200 16 1 x 20 30").is_err());
}

#[test]
fn config_parses_position_values_and_default_timing() {
    let config =
        config_from_position(&["100", "-20.5", "50", "200", "150", "400", "8", "1"]).unwrap();
    let mut expected = MotionConfig::new(100.0, -20.5, 50.0, 200.0, 150.0, 400, 8, true);
    expected.total_time_us = TOTAL_TIME_US;
    expected.pulse_t_us = PULSE_T_US;
    assert_eq!(config, expected);
}

#[test]
fn config_accepts_zero_and_one_direction_flags() {
    let zero = config_from_line("1 2 3 4 5 200 8 0").unwrap();
    assert!(!zero.ccw_positive);

    let one = config_from_line("1 2 3 4 5 200 8 1").unwrap();
    assert!(one.ccw_positive);
}

#[test]
fn config_rejects_non_boolean_direction_flags() {
    for value in ["2", "-1", "3", "truee", "yes"] {
        let error = config_from_line(&format!("1 2 3 4 5 200 8 {value}")).unwrap_err();
        assert!(error.to_string().contains("ccw_positive"), "error={error}");
    }
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
fn raw_command_parses_angles_and_motor_settings() {
    let (config, solution) = raw_command("-10 25 30 400 8 1").unwrap();
    assert_eq!(
        (config, solution),
        (
            MotionConfig::new(0.0, 0.0, 0.0, 1.0, 1.0, 400, 8, true),
            ArmSolution {
                theta_base_deg: -10.0,
                theta1_deg: 25.0,
                theta2_deg: 30.0,
                z_eff_mm: 0.0,
            }
        )
    );
}

#[test]
fn raw_command_rejects_invalid_argument_counts() {
    assert!([
        "",
        "1 2 3 4 5",
        "1 2 3 4 5 6 7",
    ]
    .into_iter()
    .all(|input| raw_command(input).is_err()));
}

#[test]
fn raw_command_rejects_non_numeric_values() {
    assert!([
        "x 2 3 4 5 1",
        "1 x 3 4 5 1",
        "1 2 x 4 5 1",
        "1 2 3 x 5 1",
        "1 2 3 4 x 1",
        "1 2 3 4 5 x",
    ]
    .into_iter()
    .all(|input| raw_command(input).is_err()));
}

#[test]
fn motor_settings_accept_all_a4988_microstep_values() {
    for microstep in [1, 2, 4, 8, 16] {
        assert!(validate_motor_settings(200, microstep).is_ok(), "{microstep}");
    }
}

#[test]
fn motor_settings_reject_zero_steps_per_revolution() {
    assert_eq!(
        validate_motor_settings(0, 1),
        Err("steps_per_rev must be positive".to_owned())
    );
}

#[test]
fn motor_settings_reject_zero_microstep() {
    assert_eq!(
        validate_motor_settings(200, 0),
        Err("microstep must be one of 1, 2, 4, 8, or 16".to_owned())
    );
}

#[test]
fn motor_settings_reject_non_power_of_two_microstep() {
    assert!(validate_motor_settings(200, 3).is_err());
}

#[test]
fn motor_settings_reject_microstep_above_a4988_maximum() {
    assert!(validate_motor_settings(200, 32).is_err());
}

#[test]
fn position_text_rejects_zero_steps_per_revolution() {
    assert!(config_from_line("100 0 50 200 200 0 1 1").is_err());
}

#[test]
fn position_text_rejects_unsupported_microstep() {
    assert!(config_from_line("100 0 50 200 200 200 3 1").is_err());
}

#[test]
fn raw_text_rejects_zero_steps_per_revolution() {
    assert!(raw_command("0 25 30 0 1 1").is_err());
}

#[test]
fn raw_text_rejects_unsupported_microstep() {
    assert!(raw_command("0 25 30 200 32 1").is_err());
}
