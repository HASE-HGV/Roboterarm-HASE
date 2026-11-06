use super::*;

#[test]
fn a_domain_label() {
    announce_domain("CLI and shell");
}

#[test]
fn cli_and_shell_propagate_input_and_output_errors() {
    assert!(prompt_position_with_io(FailingBufReader, Vec::new()).is_err());
    assert!(prompt_position_with_io(
        FailingStartReader(Cursor::new(b"100\n0\n50\n200\n200\n200\n16\n1\n".to_vec())),
        Vec::new(),
    )
    .is_err());
    assert!(run_position_loop_with_io(FailingBufReader, Vec::new(), false).is_err());
    assert!(run_position_loop_with_io(FailingBufReader, Vec::new(), true).is_err());
    assert!(run_raw_loop_with_io(FailingBufReader, Vec::new()).is_err());

    exercise_writer_failures(|writer| {
        prompt_position_with_io(
            Cursor::new(b"100\n0\n50\n200\n200\n200\n16\n1\n\n\n\n"),
            writer,
        )
        .map(|_| ())
    });
    exercise_writer_failures(|writer| {
        run_position_loop_with_io(Cursor::new(b"\ninvalid\n"), writer, false)
    });
    exercise_writer_failures(|writer| {
        run_position_loop_with_io(
            Cursor::new(b"100 0 50 200 200 200 16 1\n"),
            writer,
            false,
        )
    });
    exercise_writer_failures(|writer| {
        run_position_loop_with_io(Cursor::new(b"\n{\"command\":\"status\"}\n"), writer, true)
    });
    exercise_writer_failures(|writer| run_raw_loop_with_io(Cursor::new(b"\ninvalid\n"), writer));
    exercise_writer_failures(|writer| {
        run_raw_loop_with_io(Cursor::new(b"0 25 30 200 16 1\n"), writer)
    });
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

