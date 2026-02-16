import math as m


def calc_angles(x_mm, z_mm, l1_mm, l2_mm):
    distance_r = m.sqrt(x_mm**2 + z_mm**2)

    if distance_r > (l1_mm + l2_mm):
        raise ValueError("Position außerhalb des Arbeitsbereichs")

    alpha = m.atan2(z_mm, x_mm)

    cos_theta2 = (distance_r**2 - l1_mm**2 - l2_mm**2) / (2 * l1_mm * l2_mm)

    cos_theta2 = max(-1, min(1, cos_theta2))

    theta2 = m.acos(cos_theta2)

    theta1 = alpha - m.atan2(
        l2_mm * m.sin(theta2),
        l1_mm + l2_mm * m.cos(theta2),
    )

    return m.degrees(theta1), m.degrees(theta2)
