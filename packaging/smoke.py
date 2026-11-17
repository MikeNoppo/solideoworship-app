import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tarfile
import tempfile

from package import ROOT, invoke, msi_version, native_platform, prepare, run


def probe(executable, version):
    run(executable, "--update-probe", version)


def windows_install(package, directory, fmt, scope):
    if fmt == "nsis":
        command = f'"{package}" /S /{scope} /D={directory}'
        subprocess.run(command, check=True)
    else:
        run("msiexec.exe", "/i", package, "/qn", "/norestart", f"INSTALLDIR={directory}")


def windows_remove(directory, package, fmt, scope="AllUsers"):
    if fmt == "nsis":
        command = f'"{directory / "uninstall.exe"}" /S /{scope} _?={directory}'
        subprocess.run(command, check=True)
    else:
        run("msiexec.exe", "/x", package, "/qn", "/norestart")


def test_release(dist, version):
    system = native_platform()
    with tempfile.TemporaryDirectory(prefix="sdw package ") as directory:
        directory = Path(directory)
        if system == "windows":
            os.environ["PATH"] = os.pathsep.join(path for path in os.environ["PATH"].split(os.pathsep) if path != os.environ["FFMPEG_BIN_DIR"])
            for fmt, pattern in [("nsis", "*.exe"), ("wix", "*.msi")]:
                package = next(dist.glob(pattern)).resolve()
                for scope in (["CurrentUser", "AllUsers"] if fmt == "nsis" else ["AllUsers"]):
                    installed = directory / f"{fmt} {scope}"
                    windows_install(package, installed, fmt, scope)
                    probe(installed / "SolideoWorship.exe", version)
                    windows_remove(installed, package, fmt, scope)
        elif system == "macos":
            archive = next(dist.glob("*.app.tar.gz"))
            with tarfile.open(archive) as source:
                source.extractall(directory, filter="data")
            bundle = directory / "SolideoWorship.app"
            run("codesign", "--verify", "--deep", "--strict", bundle)
            probe(bundle / "Contents/MacOS/solideoworship", version)
            mount = directory / "mount"
            mount.mkdir()
            run("hdiutil", "attach", "-nobrowse", "-mountpoint", mount, next(dist.glob("*.dmg")))
            try:
                probe(mount / "SolideoWorship.app/Contents/MacOS/solideoworship", version)
            finally:
                run("hdiutil", "detach", mount)
        else:
            deb = next(dist.glob("*.deb"), None)
            if deb:
                run("dpkg-deb", "-x", deb, directory)
                probe(directory / "usr/bin/solideoworship", version)
            image = next(dist.glob("*.AppImage"), None)
            if image:
                image.chmod(0o755)
                probe(image, version)
            arch = next(dist.glob("*.pkg.tar.zst"), None)
            if arch:
                run("bsdtar", "-xf", arch, "-C", directory)
                probe(directory / "usr/bin/solideoworship", version)


def fixture(directory, version, fmt, broken=False):
    system = native_platform()
    extension = ".exe" if system == "windows" else ""
    bins = directory / "fixture"
    bins.mkdir(parents=True)
    source = bins / "main.rs"
    expected = "wrong-version" if broken else version
    source.write_text('fn main() -> std::process::ExitCode {\nlet mut args = std::env::args().skip(1);\nif args.next().as_deref() == Some("--update-probe") && args.next().as_deref() == Some(' + json.dumps(expected) + ') { std::process::ExitCode::SUCCESS } else { std::process::ExitCode::FAILURE }\n}\n')
    run("rustc", "--edition=2024", source, "-o", bins / ("sdw-app" + extension))
    shutil.copy2(bins / ("sdw-app" + extension), bins / ("sdw-updater" + extension))
    config = prepare(json.loads((ROOT / "packaging/packager.json").read_text()), directory, system, version, bins)
    config["formats"] = [fmt]
    if system == "windows":
        marker = directory / "package-format"
        marker.write_text(fmt)
        config["resources"].append({"src": str(marker), "target": "package-format"})
    if fmt == "wix":
        config["version"] = msi_version(version)
    invoke(config, directory / "packager.json")
    output = Path(config["outDir"])
    if fmt == "app":
        bundle = output / "SolideoWorship.app"
        run("python3", ROOT / "packaging/macos/bundle.py", bundle)
        archive = output / "fixture.app.tar.gz"
        with tarfile.open(archive, "w:gz", format=tarfile.USTAR_FORMAT) as tar:
            tar.add(bundle, arcname=bundle.name)
        return archive
    return next(output.glob({"nsis": "*.exe", "wix": "*.msi", "appimage": "*.AppImage"}[fmt]))


def test_upgrades(directory):
    system = native_platform()
    formats = {"windows": ["nsis", "wix"], "macos": ["app"], "linux": ["appimage"]}[system]
    private = directory / "test-key.der"
    private.write_bytes(bytes.fromhex("302e020100300506032b657004220420") + bytes([7]) * 32)
    public = run("openssl", "pkey", "-inform", "DER", "-in", private, "-pubout", "-outform", "DER", capture_output=True).stdout[-32:].hex()
    for fmt in formats:
        packages = [fixture(directory / f"{fmt}-{index}", f"0.1.0-beta.{100 + index}", fmt, broken=index == 2) for index in range(3)]
        installed = directory / f"{fmt} installation"
        installed.mkdir()
        if system == "windows":
            windows_install(packages[0], installed, fmt, "AllUsers")
            executable = installed / "SolideoWorship.exe"
        elif system == "macos":
            with tarfile.open(packages[0]) as archive:
                archive.extractall(installed, filter="data")
            executable = installed / "SolideoWorship.app/Contents/MacOS/solideoworship"
        else:
            executable = installed / "SolideoWorship.AppImage"
            shutil.copy2(packages[0], executable)
            executable.chmod(0o755)
        probe(executable, "0.1.0-beta.100")
        for index, package in enumerate(packages[1:], 1):
            env = os.environ.copy()
            env.update(SDW_UPDATE_PUBLIC_KEY=public, SDW_TEST_EXECUTABLE=str(executable), SDW_TEST_PACKAGE=str(package), SDW_TEST_VERSION=f"0.1.0-beta.{100 + index}")
            if index == 2:
                env["SDW_TEST_REJECT"] = "1"
            run("cargo", "test", "-p", "sdw-app", "--lib", "--locked", "--profile", "ci", "native_package_update", "--", "--ignored", env=env, cwd=ROOT)
            probe(executable, "0.1.0-beta.101")
        if system == "windows":
            windows_remove(installed, packages[1], fmt)


if __name__ == "__main__":
    if native_platform() == "windows" and os.environ.get("GITHUB_ACTIONS") != "true":
        raise SystemExit("Run Windows system installation tests only on disposable GitHub Actions runners")
    parser = argparse.ArgumentParser()
    parser.add_argument("dist", type=Path)
    parser.add_argument("version")
    parser.add_argument("--upgrades", action="store_true")
    args = parser.parse_args()
    test_release(args.dist.resolve(), args.version)
    if args.upgrades:
        with tempfile.TemporaryDirectory(prefix="sdw upgrades ") as directory:
            test_upgrades(Path(directory))
