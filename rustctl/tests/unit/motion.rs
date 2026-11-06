use super::*;
use crate::motion::{checked_step_plan, execute_solution};

#[test]
fn a_domain_label() {
    announce_domain("Motion planning and steps");
}

#[test]
fn deg_to_steps_full_revolution() {
    assert_eq!(deg_to_steps(360.0, 200, 1, GEAR_RATIO), 3200);
}

#[test]
fn deg_to_steps_quarter_and_microstep() {
    assert_eq!(deg_to_steps(90.0, 200, 1, GEAR_RATIO), 800);
}

#[test]
fn deg_to_steps_scales_with_microstepping() {
    assert_eq!(deg_to_steps(90.0, 200, 16, GEAR_RATIO), 12800);
}

#[test]
fn deg_to_steps_preserves_negative_sign() {
    assert_eq!(deg_to_steps(-90.0, 200, 1, GEAR_RATIO), -800);
}

#[test]
fn deg_to_steps_zero_angle_produces_zero_steps() {
    assert_eq!(deg_to_steps(0.0, 200, 16, GEAR_RATIO), 0);
}

#[test]
fn deg_to_steps_rounds_to_nearest() {
    assert_eq!(deg_to_steps(1.0, 200, 1, GEAR_RATIO), 9);
}

#[test]
fn direction_positive_steps_are_ccw_when_configured_positive() {
    assert!(direction_is_ccw(5, true));
}

#[test]
fn direction_negative_steps_are_not_ccw_when_configured_positive() {
    assert!(!direction_is_ccw(-5, true));
}

#[test]
fn direction_positive_steps_are_not_ccw_when_configured_negative() {
    assert!(!direction_is_ccw(5, false));
}

#[test]
fn direction_negative_steps_are_ccw_when_configured_negative() {
    assert!(direction_is_ccw(-5, false));
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
    assert_eq!((ticks, totals), (30, [30, 12, 7]));
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
fn deg_to_steps_with_zero_steps_per_revolution_is_zero() {
    assert_eq!(deg_to_steps(90.0, 0, 16, GEAR_RATIO), 0);
}

#[test]
fn deg_to_steps_with_zero_microstep_is_zero() {
    assert_eq!(deg_to_steps(90.0, 200, 0, GEAR_RATIO), 0);
}

#[test]
fn deg_to_steps_with_zero_gear_ratio_is_zero() {
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

#[test]
fn plan_report_omits_start_position_for_homed_moves() {
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
    assert!(
        !homed.contains("Start position")
            && !homed.contains("Relative travel")
            && homed.contains("Axis 1: 90.000")
            && homed.contains("Axis 1: 800"),
        "{homed}"
    );
}

#[test]
fn plan_report_shows_current_pose_and_relative_travel() {
    let solution = ArmSolution {
        theta_base_deg: 0.0,
        theta1_deg: 90.0,
        theta2_deg: 0.0,
        z_eff_mm: 0.0,
    };
    let relative = plan_report(
        &config_from_line(&position_line([0.0, 0.0, 0.0], Some([5.0, 45.0, 0.0]))).unwrap(),
        &solution,
    );
    assert!(
        relative.contains("Start position: base 5.000°, axis 1 45.000°, axis 2 0.000°")
            && relative
                .contains("Relative travel: base -5.000°, axis 1 45.000°, axis 2 0.000°")
            && relative.contains("Axis 1: 400"),
        "{relative}"
    );
}

#[test]
fn checked_deg_to_steps_matches_deg_to_steps_for_normal_angles() {
    assert_eq!(checked_deg_to_steps(360.0, 200, 1, GEAR_RATIO), Ok(3200));
    assert_eq!(checked_deg_to_steps(90.0, 200, 1, GEAR_RATIO), Ok(800));
    assert_eq!(
        checked_deg_to_steps(-45.0, 200, 16, GEAR_RATIO),
        Ok(deg_to_steps(-45.0, 200, 16, GEAR_RATIO))
    );
}

#[test]
fn checked_deg_to_steps_refuses_huge_angle() {
    assert!(checked_deg_to_steps(1e300, 200, 16, GEAR_RATIO).is_err());
}

#[test]
fn checked_step_plan_matches_step_plan_for_finite_values() {
    let solution = ArmSolution {
        theta_base_deg: 40.0,
        theta1_deg: 90.0,
        theta2_deg: -45.0,
        z_eff_mm: 0.0,
    };
    let config = config_from_line(&position_line([0.0, 0.0, 0.0], Some([5.0, 10.0, 0.0]))).unwrap();
    assert_eq!(
        checked_step_plan(&config, solution),
        Ok(step_plan(&config, solution))
    );
}

#[test]
fn execute_solution_refuses_huge_joint_angle() {
    let (config, solution) = raw_command("1e300 0 0 200 16 1").unwrap();
    let err = execute_solution(&config, solution).unwrap_err();
    assert!(
        err.to_string()
            .contains("cannot be converted to motor steps"),
        "{err}"
    );
}
