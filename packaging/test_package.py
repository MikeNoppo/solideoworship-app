import json
from pathlib import Path
from tempfile import TemporaryDirectory
import unittest
from unittest.mock import patch

from package import ROOT, msi_version, prepare
from smoke import windows_msi


class Versions(unittest.TestCase):
    def test_msi_orders_betas_candidates_and_stable_within_three_fields(self):
        versions = ["0.1.0-beta.4", "0.1.0-beta.10", "0.1.0-rc.1", "0.1.0", "0.1.1-beta.1", "0.1.1", "0.2.0-beta.1"]
        numeric = [tuple(map(int, msi_version(version).split("."))) for version in versions]
        self.assertEqual(numeric, sorted(set(numeric)))

    def test_msi_rejects_versions_that_would_collide_or_overflow(self):
        for version in ["0.1.0-beta.0", "0.1.0-beta.500", "0.1.0-rc.499", "256.0.0", "0.256.0", "0.1.66", "0.1.0-alpha.1", "0.1.0+build"]:
            with self.subTest(version=version), self.assertRaises(ValueError):
                msi_version(version)


class WindowsResources(unittest.TestCase):
    def test_packaged_license_sources_have_unique_install_filenames(self):
        with TemporaryDirectory() as temp:
            stage = Path(temp)
            for exe in ["sdw-app.exe", "sdw-updater.exe"]:
                (stage / exe).touch()
            ffmpeg = stage / "ffmpeg" / "bin"
            ffmpeg.mkdir(parents=True)
            (ffmpeg / "avcodec.dll").touch()
            (ffmpeg.parent / "LICENSE.txt").write_text("FFmpeg license")
            config = json.loads((ROOT / "packaging/packager.json").read_text())
            with patch.dict("os.environ", {"FFMPEG_BIN_DIR": str(ffmpeg)}):
                prepare(config, stage, "windows", "0.1.0-beta.4", stage)

            files = {item["target"]: Path(item["src"]) for item in config["resources"]}
            self.assertEqual(set(files), {path.name for path in files.values()})
            self.assertEqual(files["LICENSE.txt"].read_bytes(), (ROOT / "LICENSE").read_bytes())
            self.assertEqual(files["LICENSE-ac-ffmpeg.txt"].read_bytes(), (ROOT / "vendor/ac-ffmpeg/LICENSE").read_bytes())
            self.assertEqual(files["LICENSE-FFmpeg.txt"].read_text(), "FFmpeg license")


class MsiCommand(unittest.TestCase):
    def test_directory_property_quotes_only_its_value(self):
        with patch("smoke.subprocess.run") as process:
            windows_msi("/i", r"C:\Update Cache\package.msi", "/qn", installation=Path("C:\\Users\\operator\\Solideo Worship\\"))
        command = process.call_args.args[0]
        self.assertIn('INSTALLDIR="C:\\Users\\operator\\Solideo Worship\\"', command)
        self.assertNotIn('"INSTALLDIR=', command)


if __name__ == "__main__":
    unittest.main()
