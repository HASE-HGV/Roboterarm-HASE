const HARDWARE_OVERHEAD_US: u64 = 83;

macro_rules! debug_invariant {
    ($cond:expr $(, $dump:expr)*) => {
        #[cfg(debug_assertions)]
        {
            if !($cond) {
                $(dbg!(&$dump);)*
                panic!("debug invariant violated: {}", stringify!($cond));
            }
        }
    };
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ArmSolution {
    pub(crate) theta_base_deg: f64,
    pub(crate) theta1_deg: f64,
    pub(crate) theta2_deg: f64,
    pub(crate) z_eff_mm: f64,
}

pub(crate) fn forward_r_z_mm(
    theta1_deg: f64,
    theta2_deg: f64,
    l1_mm: f64,
    l2_mm: f64,
) -> (f64, f64) {
    let t1 = theta1_deg.to_radians();
    let t12 = (theta1_deg + theta2_deg).to_radians();
    let r = l1_mm * t1.cos() + l2_mm * t12.cos();
    let z = l1_mm * t1.sin() + l2_mm * t12.sin();
    (r, z)
}

pub(crate) fn ik_angles_3d_deg(
    x_mm: f64,
    y_mm: f64,
    z_mm: f64,
    l1_mm: f64,
    l2_mm: f64,
) -> Result<ArmSolution, &'static str> {
    if !x_mm.is_finite()
        || !y_mm.is_finite()
        || !z_mm.is_finite()
        || !l1_mm.is_finite()
        || !l2_mm.is_finite()
    {
        return Err("Kinematics inputs must be finite");
    }
    if l1_mm <= 0.0 || l2_mm <= 0.0 {
        return Err("Link lengths must be positive");
    }
    if x_mm < 0.0 {
        return Err("Radius X must be non-negative");
    }

    let theta_base = y_mm.to_radians();
    let r = x_mm;
    let r_space = (r * r + z_mm * z_mm).sqrt();
    if r_space > l1_mm + l2_mm || r_space < (l1_mm - l2_mm).abs() {
        return Err("Out of workspace");
    }

    let alpha = z_mm.atan2(r);
    let cos_theta2 = ((r_space * r_space - l1_mm * l1_mm - l2_mm * l2_mm) / (2.0 * l1_mm * l2_mm))
        .clamp(-1.0, 1.0);
    let theta2 = cos_theta2.acos();
    let theta1 = alpha - (l2_mm * theta2.sin()).atan2(l1_mm + l2_mm * theta2.cos());

    let (_r_eff, z_eff) = forward_r_z_mm(theta1.to_degrees(), theta2.to_degrees(), l1_mm, l2_mm);

    debug_invariant!(
        (_r_eff - r).abs() < 1e-6 && (z_eff - z_mm).abs() < 1e-6,
        r,
        z_mm,
        _r_eff,
        z_eff
    );

    Ok(ArmSolution {
        theta_base_deg: theta_base.to_degrees(),
        theta1_deg: theta1.to_degrees(),
        theta2_deg: theta2.to_degrees(),
        z_eff_mm: z_eff,
    })
}

pub(crate) fn deg_to_steps(
    angle_deg: f64,
    steps_per_rev: u64,
    microstep: u64,
    gear_ratio: f64,
) -> i64 {
    let steps_per_deg = (steps_per_rev * microstep) as f64 / 360.0;
    (angle_deg * steps_per_deg * gear_ratio).round() as i64
}

#[cfg(any(all(feature = "hardware", target_os = "linux"), test))]
pub(crate) fn direction_is_ccw(steps: i64, ccw_positive: bool) -> bool {
    (steps > 0) == ccw_positive
}

pub(crate) fn minimum_period_us(pulse_t_us: u64) -> u64 {
    2 * pulse_t_us + HARDWARE_OVERHEAD_US
}

pub(crate) fn overhead_sleep_us(total_time_us: u64, pulse_t_us: u64) -> Result<u64, String> {
    let min = minimum_period_us(pulse_t_us);
    if total_time_us < min {
        return Err(format!(
            "total_time ({total_time_us}) must be >= {min} µs to accommodate \
             pulse widths and hardware overhead"
        ));
    }
    Ok(total_time_us - min)
}
