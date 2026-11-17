import argparse
import copy
import json
import os
from pathlib import Path
import platform
import re
import shlex
import shutil
import subprocess
import sys
import tarfile
import tomllib

ROOT = Path(__file__).resolve().parent.parent
FORMATS = {"windows": ["nsis", "wix"], "macos": ["app", "dmg"], "linux": ["deb", "appimage", "pacman"]}


def run(*args, **kwargs):
    try:
        return subprocess.run([str(arg) for arg in args], check=True, **kwargs)
    except subprocess.CalledProcessError as error:
        if error.stderr:
            sys.stderr.write(error.stderr.decode() if isinstance(error.stderr, bytes) else error.stderr)
        raise


def msi_version(version):
    match = re.fullmatch(r"(\d+)\.(\d+)\.(\d+)(?:-(beta|rc)\.(\d+))?", version)
    if not match:
        raise ValueError("MSI releases must use x.y.z, x.y.z-beta.N or x.y.z-rc.N")
    major, minor, patch = map(int, match.group(1, 2, 3))
    channel, serial = match.group(4, 5)
    rank = 999
    if channel:
        serial = int(serial)
        if not 1 <= serial <= (499 if channel == "beta" else 498):
            raise ValueError("MSI prerelease counter exceeds its reserved range")
        rank = serial + (500 if channel == "rc" else 0)
    build = patch * 1000 + rank
    if major > 255 or minor > 255 or build > 65535:
        raise ValueError("Version exceeds Windows Installer numeric limits")
    return f"{major}.{minor}.{build}"


def native_platform():
    return {"win32": "windows", "darwin": "macos"}.get(sys.platform, "linux")


def prepare(config, stage, system, version, binary_dir):
    main = "SolideoWorship" if system == "windows" else "solideoworship"
    helper = "SolideoWorshipUpdater" if system == "windows" else "solideoworship-updater"
    extension = ".exe" if system == "windows" else ""
    bins = stage / "bin"
    bins.mkdir(parents=True, exist_ok=True)
    for source, target in [("sdw-app", main), ("sdw-updater", helper)]:
        shutil.copy2(binary_dir / (source + extension), bins / (target + extension))
    config.update(version=version, binariesDir=str(bins), outDir=str(stage / "out"))
    config["binaries"] = [{"path": main, "main": True}, {"path": helper, "main": False}]
    config["icons"] = [str(ROOT / "assets/logo/solideoworship.ico")]
    config["icons"] += [str(ROOT / f"assets/logo/png/app-icon-{size}.png") for size in [16, 24, 32, 48, 64, 128, 256]]
    resources = {
        "LICENSE.txt": ROOT / "LICENSE",
        "PRIVACY.txt": ROOT / "PRIVACY.md",
        "LICENSE-ac-ffmpeg.txt": ROOT / "vendor/ac-ffmpeg/LICENSE",
        "LICENSE-Inter.txt": ROOT / "assets/fonts/Inter-LICENSE.txt",
        "LICENSE-JetBrainsMono.txt": ROOT / "assets/fonts/JetBrainsMono-LICENSE.txt",
        "LICENSE-Lucide.txt": ROOT / "assets/fonts/Lucide-LICENSE.txt",
    }
    if system == "windows":
        ffmpeg = Path(os.environ["FFMPEG_BIN_DIR"])
        dlls = list(ffmpeg.glob("*.dll"))
        if not dlls:
            raise ValueError("FFMPEG_BIN_DIR contains no DLLs")
        resources.update({dll.name: dll for dll in dlls})
        resources["LICENSE-FFmpeg.txt"] = ffmpeg.parent / "LICENSE.txt"
        renamed = stage / "resources"
        renamed.mkdir(exist_ok=True)
        for name, source in resources.items():
            if source.name != name:
                target = renamed / name
                shutil.copy2(source, target)
                resources[name] = target
        eula = stage / "privacy.txt"
        eula.write_text("\n\n".join((ROOT / f"packaging/windows/privacy-{lang}.txt").read_text(encoding="utf-8") for lang in ["en", "id"]), encoding="utf-8")
        config["licenseFile"] = str(eula)
        nsis_template = stage / "installer.nsi"
        nsis_template.write_text((ROOT / "packaging/windows/installer.nsi").read_text().replace("__INSTALL_VERSION__", msi_version(version)))
        config["nsis"].update(template=str(nsis_template), installerIcon=str(ROOT / "assets/logo/solideoworship.ico"))
        config["wix"] = {"template": str(ROOT / "packaging/windows/installer.wxs")}
    if system == "linux":
        config["deb"] = {"packageName": "solideoworship", "desktopTemplate": str(ROOT / "packaging/linux/solideoworship.desktop")}
    if system == "linux" and shutil.which("dpkg-query"):
        libraries = run("ldd", bins / main, capture_output=True).stdout.decode()
        for path in re.findall(r"=> (/[^ ]+)", libraries):
            owned = subprocess.run(["dpkg-query", "-S", path], capture_output=True, text=True)
            if owned.returncode == 0:
                package = owned.stdout.split(":", 1)[0]
                license_path = Path("/usr/share/doc") / package / "copyright"
                if license_path.is_file():
                    resources[f"third-party/{package}-copyright"] = license_path
    config["resources"] = [{"src": str(source), "target": name} for name, source in resources.items()]
    return config


def package(formats, skip_build=False):
    system = native_platform()
    arch = {"AMD64": "x86_64", "arm64": "aarch64"}.get(platform.machine(), platform.machine())
    if arch not in (["x86_64", "aarch64"] if system == "macos" else ["x86_64"]):
        raise ValueError(f"Unsupported release architecture: {system}-{arch}")
    version = tomllib.loads((ROOT / "Cargo.toml").read_text())["workspace"]["package"]["version"]
    formats = formats or (["deb", "appimage"] if system == "linux" else FORMATS[system])
    if set(formats) - set(FORMATS[system]):
        raise ValueError(f"Unsupported formats for {system}: {formats}")
    if system == "macos" and "-" not in version:
        raise ValueError("Stable macOS distribution requires Developer ID and notarization")
    if not skip_build:
        run("cargo", "build", "--release", "--locked", "-p", "sdw-app", "--bins", cwd=ROOT)
    metadata = json.loads(run("cargo", "metadata", "--no-deps", "--format-version", "1", cwd=ROOT, capture_output=True).stdout)
    target = Path(metadata["target_directory"])
    stage = target / "packaging" / f"{system}-{arch}"
    stage.mkdir(parents=True, exist_ok=True)
    config = prepare(json.loads((ROOT / "packaging/packager.json").read_text()), stage, system, version, target / "release")
    destination = target / "dist" / f"{system}-{arch}"
    destination.mkdir(parents=True, exist_ok=True)
    prefix = f"SolideoWorship-{version}-{system}-{'x64' if system == 'windows' else arch}"
    if system == "macos":
        config["formats"] = ["app", "dmg"] if "dmg" in formats else ["app"]
        bundle = Path(config["outDir"]) / "SolideoWorship.app"
        config["beforeEachPackageCommand"] = shlex.join([sys.executable, str(ROOT / "packaging/macos/bundle.py"), str(bundle), "--before-package"])
        invoke(config, stage / "packager.json")
        if "dmg" not in formats:
            run(sys.executable, ROOT / "packaging/macos/bundle.py", bundle)
        with tarfile.open(destination / f"{prefix}.app.tar.gz", "w:gz", format=tarfile.USTAR_FORMAT) as archive:
            archive.add(bundle, arcname=bundle.name)
        if "dmg" in formats:
            export_one(Path(config["outDir"]), "*.dmg", destination / f"{prefix}.dmg")
        return
    for fmt in formats:
        current = copy.deepcopy(config)
        output = stage / fmt
        if output.exists():
            shutil.rmtree(output)
        output.mkdir()
        current.update(outDir=str(output), formats=[fmt])
        if system == "windows":
            marker = stage / "package-format"
            marker.write_text(fmt)
            current["resources"].append({"src": str(marker), "target": "package-format"})
        if fmt == "wix":
            current["version"] = msi_version(version)
        if fmt == "deb":
            current["version"] = version.replace("-", "~", 1)
            control = stage / "debian/control"
            control.parent.mkdir(exist_ok=True)
            control.write_text("Source: solideoworship\nSection: misc\nPriority: optional\nMaintainer: MikeNoppo <noreply@solideoworship.com>\nStandards-Version: 4.6.2\n\nPackage: solideoworship\nArchitecture: any\nDescription: Worship presenter\n")
            dependencies = run("dpkg-shlibdeps", "-O", "-e", stage / "bin/solideoworship", cwd=stage, capture_output=True).stdout.decode().strip().removeprefix("shlibs:Depends=")
            current["deb"]["depends"] = [part.strip() for part in dependencies.split(",")]
        if fmt == "pacman":
            current["version"] = version.replace("-", "").replace("+", "_")
            current["pacman"] = {"depends": ["alsa-lib", "ffmpeg", "libxcb", "gcc-libs", "glibc"]}
        invoke(current, stage / f"{fmt}.json")
        if fmt == "pacman":
            pkgbuild = output / "PKGBUILD"
            text = pkgbuild.read_text().replace("pkgname=solideo-worship", "pkgname=solideoworship")
            text = text.replace("options=(!lto)", "options=(!lto !debug !strip)")
            text = text.replace('cp -r "${srcdir}"/* "${pkgdir}"/', 'cp -r "${srcdir}/usr" "${pkgdir}/"')
            pkgbuild.write_text(text + "\nlicense=('GPL-3.0-only')\n")
            run("makepkg", "--force", cwd=output)
            export_one(output, "*.pkg.tar.zst", destination / f"{prefix}.pkg.tar.zst")
        else:
            pattern, suffix = {"nsis": ("*.exe", "-nsis.exe"), "wix": ("*.msi", ".msi"), "deb": ("*.deb", ".deb"), "appimage": ("*.AppImage", ".AppImage")}[fmt]
            export_one(output, pattern, destination / f"{prefix}{suffix}")


def invoke(config, path):
    path.write_text(json.dumps(config, indent=2))
    env = os.environ.copy()
    if "appimage" in config["formats"]:
        env.setdefault("NO_STRIP", "1")
    verbosity = ["--verbose"] if "wix" in config["formats"] else []
    run("cargo", "packager", *verbosity, "--config", path, cwd=ROOT, env=env)


def export_one(directory, pattern, destination):
    files = list(directory.glob(pattern))
    if len(files) != 1:
        raise ValueError(f"Expected one {pattern} in {directory}, found {files}")
    shutil.copy2(files[0], destination)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--format", action="append", choices=sum(FORMATS.values(), []))
    parser.add_argument("--skip-build", action="store_true")
    args = parser.parse_args()
    package(args.format, args.skip_build)
