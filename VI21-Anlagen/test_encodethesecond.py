import contextlib
import io
import os
import tempfile
import unittest
from pathlib import Path
from unittest.mock import patch

import encodethesecond


class EncodeVideosTests(unittest.TestCase):
    def setUp(self):
        self.temp_dir = tempfile.TemporaryDirectory()
        self.previous_dir = Path.cwd()
        os.chdir(self.temp_dir.name)

    def tearDown(self):
        os.chdir(self.previous_dir)
        self.temp_dir.cleanup()

    def make_file(self, name):
        path = Path(name)
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text("video")
        return path

    def test_nested_inputs_create_separate_output_directories(self):
        self.make_file("clips/a.mp4")
        self.make_file("interviews/a.mp4")

        def fake_ffmpeg(command, check):
            Path(command[-1]).write_text("encoded")

        with patch("builtins.input", return_value="clips/a.mp4, interviews/a.mp4"), patch(
            "encodethesecond.subprocess.run", side_effect=fake_ffmpeg
        ) as run, contextlib.redirect_stdout(io.StringIO()):
            encodethesecond.encode_videos()

        self.assertEqual(run.call_count, 2)
        self.assertTrue(Path("encoded/clips/a.mov").is_file())
        self.assertTrue(Path("encoded/interviews/a.mov").is_file())

    def test_same_output_collision_skips_all_conflicting_inputs(self):
        self.make_file("a.mp4")
        self.make_file("a.avi")

        with patch("builtins.input", return_value="a.mp4, a.avi"), patch(
            "encodethesecond.subprocess.run"
        ) as run, contextlib.redirect_stdout(io.StringIO()) as output:
            encodethesecond.encode_videos()

        run.assert_not_called()
        self.assertIn("Multiple inputs map to 'encoded/a.mov'", output.getvalue())
        self.assertFalse(Path("encoded/a.mov").exists())


if __name__ == "__main__":
    unittest.main()
