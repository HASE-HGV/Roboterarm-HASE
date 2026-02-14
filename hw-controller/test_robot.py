import unittest
from unittest.mock import patch, MagicMock
import robot as robot # deine Datei


class TestMoveMotor(unittest.TestCase):

    @patch("robot.time.sleep")
    @patch("robot.GPIO.output")
    def test_move_motor_forward(self, mock_output, mock_sleep):
        robot.move_motor(
            step_pin=17, dir_pin=27, steps=3, delay=0.01, delta=10  # positiv -> LOW
        )

        # Richtungs-Pin muss LOW gesetzt werden
        mock_output.assert_any_call(27, robot.GPIO.LOW)

        # 3 Steps = 6 output calls (HIGH + LOW je Step)
        self.assertEqual(mock_output.call_count, 1 + 6)

    @patch("robot.time.sleep")
    @patch("robot.GPIO.output")
    def test_move_motor_backward(self, mock_output, mock_sleep):
        robot.move_motor(
            step_pin=17, dir_pin=27, steps=2, delay=0.01, delta=-10  # negativ -> HIGH
        )

        mock_output.assert_any_call(27, robot.GPIO.HIGH)
