use super::*;

#[test]
fn a_domain_label() {
    announce_domain("Kinematics");
}

#[test]
fn ik_fully_extended_along_x() {
    let s = ik_angles_3d_deg(200.0, 0.0, 0.0, 100.0, 100.0).unwrap();
    assert_eq!(
        s,
        ArmSolution {
            theta_base_deg: 0.0,
            theta1_deg: 0.0,
            theta2_deg: 0.0,
            z_eff_mm: 0.0,
        }
    );
}

#[test]
fn ik_right_angle_reach() {
    let s = ik_angles_3d_deg(100.0, 0.0, 100.0, 100.0, 100.0).unwrap();
    assert!(approx(s.theta1_deg, 90.0) && approx(s.theta2_deg, -90.0));
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
fn forward_kinematics_zero_angles_extends_along_radius() {
    assert_eq!(forward_r_z_mm(0.0, 0.0, 10.0, 5.0), (15.0, 0.0));
}

#[test]
fn forward_kinematics_ninety_degree_shoulder_extends_vertically() {
    let (radius, height) = forward_r_z_mm(90.0, 0.0, 10.0, 5.0);
    assert!(approx(radius, 0.0));
    assert!(approx(height, 15.0));
}

#[test]
fn forward_kinematics_ninety_degree_elbow_rotates_second_link() {
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
fn ik_rejects_non_finite_radius() {
    assert_eq!(
        ik_angles_3d_deg(f64::NAN, 0.0, 0.0, 100.0, 100.0),
        Err("Kinematics inputs must be finite")
    );
}

#[test]
fn ik_rejects_infinite_base_angle() {
    assert_eq!(
        ik_angles_3d_deg(100.0, f64::INFINITY, 0.0, 100.0, 100.0),
        Err("Kinematics inputs must be finite")
    );
}

#[test]
fn ik_rejects_nan_height() {
    assert_eq!(
        ik_angles_3d_deg(100.0, 0.0, f64::NAN, 100.0, 100.0),
        Err("Kinematics inputs must be finite")
    );
}

#[test]
fn ik_rejects_infinite_first_link_length() {
    assert_eq!(
        ik_angles_3d_deg(100.0, 0.0, 0.0, f64::INFINITY, 100.0),
        Err("Kinematics inputs must be finite")
    );
}

#[test]
fn ik_rejects_nan_second_link_length() {
    assert_eq!(
        ik_angles_3d_deg(100.0, 0.0, 0.0, 100.0, f64::NAN),
        Err("Kinematics inputs must be finite")
    );
}

#[test]
fn ik_selects_an_elbow_above_the_shoulder_when_a_branch_allows_it() {
    let solution = ik_angles_3d_deg(100.0, 0.0, 100.0, 100.0, 100.0).unwrap();
    assert!(solution.theta1_deg > 0.0);
}

#[test]
fn ik_selects_the_branch_with_the_highest_elbow_across_reachable_targets() {
    for (radius, height, l1, l2) in [
        (100.0, 100.0, 100.0, 100.0),
        (150.0, 40.0, 120.0, 90.0),
        (50.0, -100.0, 100.0, 100.0),
        (0.0, 100.0, 80.0, 120.0),
        (180.0, -20.0, 120.0, 100.0),
    ] {
        let solution = ik_angles_3d_deg(radius, 0.0, height, l1, l2).unwrap();
        let alpha = height.atan2(radius);
        let other_theta2 = -solution.theta2_deg.to_radians();
        let other_theta1 = alpha
            - (l2 * other_theta2.sin()).atan2(l1 + l2 * other_theta2.cos());
        assert!(
            solution.theta1_deg.to_radians().sin() + 1e-10
                >= other_theta1.sin(),
            "radius={radius}, height={height}, links=({l1}, {l2})"
        );
    }
}

#[test]
fn ik_does_not_claim_an_elbow_above_the_shoulder_when_neither_branch_can() {
    let solution = ik_angles_3d_deg(100.0, 0.0, -100.0, 100.0, 100.0).unwrap();
    assert!(solution.theta1_deg.to_radians().sin() <= 1e-10);
}

#[test]
fn ik_round_trips_targets_across_a_reachable_grid() {
    let targets = [
        (40.0, 0.0),
        (0.0, 100.0),
        (100.0, 0.0),
        (150.0, 50.0),
        (0.0, -100.0),
        (200.0, 0.0),
    ];
    for (radius, height) in targets {
        for base in [-720.0, -180.0, -30.0, 0.0, 45.0, 270.0, 1080.0] {
            assert!(
                ik_round_trip_matches(radius, base, height, 80.0, 120.0),
                "radius={radius}, base={base}, height={height}"
            );
        }
    }
}

#[test]
fn ik_accepts_the_exact_outer_workspace_boundary_for_unequal_links() {
    assert!(ik_angles_3d_deg(300.0, 0.0, 0.0, 200.0, 100.0).is_ok());
}

#[test]
fn ik_rejects_a_target_just_beyond_the_outer_workspace_boundary() {
    assert_eq!(
        ik_angles_3d_deg(300.000001, 0.0, 0.0, 200.0, 100.0),
        Err("Out of workspace")
    );
}

#[test]
fn ik_accepts_the_exact_inner_workspace_boundary_for_unequal_links() {
    assert!(ik_angles_3d_deg(100.0, 0.0, 0.0, 200.0, 100.0).is_ok());
}

#[test]
fn ik_rejects_a_target_just_inside_the_inner_workspace_boundary() {
    assert_eq!(
        ik_angles_3d_deg(99.999999, 0.0, 0.0, 200.0, 100.0),
        Err("Out of workspace")
    );
}

#[test]
fn ik_accepts_the_folded_equal_link_origin() {
    assert!(ik_angles_3d_deg(0.0, 0.0, 0.0, 100.0, 100.0).is_ok());
}

#[test]
fn ik_rejects_a_negative_first_link_length() {
    assert_eq!(
        ik_angles_3d_deg(50.0, 0.0, 0.0, -1.0, 100.0),
        Err("Link lengths must be positive")
    );
}

#[test]
fn ik_rejects_a_negative_second_link_length() {
    assert_eq!(
        ik_angles_3d_deg(50.0, 0.0, 0.0, 100.0, -1.0),
        Err("Link lengths must be positive")
    );
}

#[test]
fn ik_rejects_a_zero_first_link_length() {
    assert_eq!(
        ik_angles_3d_deg(50.0, 0.0, 0.0, 0.0, 100.0),
        Err("Link lengths must be positive")
    );
}

#[test]
fn ik_rejects_a_zero_second_link_length() {
    assert_eq!(
        ik_angles_3d_deg(50.0, 0.0, 0.0, 100.0, 0.0),
        Err("Link lengths must be positive")
    );
}

#[test]
fn ik_rejects_infinite_second_link_length() {
    assert_eq!(
        ik_angles_3d_deg(50.0, 0.0, 0.0, 100.0, f64::INFINITY),
        Err("Kinematics inputs must be finite")
    );
}
