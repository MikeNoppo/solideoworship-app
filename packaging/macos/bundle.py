import argparse
import os
import re
from pathlib import Path
import shutil
import subprocess


def run(*args):
    return subprocess.run([str(arg) for arg in args], check=True, capture_output=True, text=True).stdout


def dependencies(binary):
    return [line.strip().split(" (compatibility version", 1)[0] for line in run("otool", "-L", binary).splitlines()[1:]]


def finalize(bundle):
    frameworks = bundle / "Contents/Frameworks"
    frameworks.mkdir(exist_ok=True)
    main_dir = bundle / "Contents/MacOS"
    pending = [(path, path) for path in main_dir.iterdir() if path.is_file()]
    copied = {}
    signed = []
    while pending:
        original, binary = pending.pop()
        signed.append(binary)
        for dependency in dependencies(original):
            if dependency.startswith(("/usr/lib/", "/System/Library/")):
                continue
            resolved = dependency
            if dependency.startswith("@rpath/"):
                rpaths = re.findall(r"cmd LC_RPATH\n\s*cmdsize \d+\n\s*path (.*?) \(offset", run("otool", "-l", original))
                candidates = [Path(path.replace("@loader_path", str(original.parent)).replace("@executable_path", str(main_dir))) / dependency.removeprefix("@rpath/") for path in rpaths]
                resolved = next((str(path) for path in candidates if path.is_file()), None)
                if resolved is None:
                    raise ValueError(f"Unresolved runtime search path in {original}: {dependency}")
            source = Path(resolved.replace("@loader_path", str(original.parent)).replace("@executable_path", str(main_dir))).resolve(strict=True)
            if source == original.resolve():
                continue
            target = frameworks / source.name
            if source.name in copied and copied[source.name] != source:
                raise ValueError(f"Conflicting dynamic libraries: {source.name}")
            if source.name not in copied:
                copied[source.name] = source
                shutil.copy2(source, target)
                target.chmod(0o755)
                run("install_name_tool", "-id", f"@rpath/{source.name}", target)
                pending.append((source, target))
            replacement = f"@loader_path/{source.name}" if binary.parent == frameworks else f"@executable_path/../Frameworks/{source.name}"
            run("install_name_tool", "-change", dependency, replacement, binary)
    for binary in signed:
        run("codesign", "--force", "--sign", "-", binary)
    run("codesign", "--force", "--sign", "-", bundle)
    run("codesign", "--verify", "--deep", "--strict", bundle)
    for binary in signed:
        if any(dep.startswith(("/opt/homebrew/", "/usr/local/")) for dep in dependencies(binary)):
            raise ValueError(f"Unbundled dynamic library in {binary}")
    license_dir = bundle / "Contents/Resources/third-party"
    license_dir.mkdir(exist_ok=True)
    for source in copied.values():
        parts = source.parts
        if "Cellar" in parts:
            cellar = Path(*parts[:parts.index("Cellar") + 3])
            name = parts[parts.index("Cellar") + 1]
            for pattern in ["LICENSE*", "COPYING*", "NOTICE*"]:
                for license_path in cellar.glob(pattern):
                    if license_path.is_file():
                        shutil.copy2(license_path, license_dir / f"{name}-{license_path.name}")
    run("codesign", "--force", "--sign", "-", bundle)
    run("codesign", "--verify", "--deep", "--strict", bundle)


if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("bundle", type=Path)
    parser.add_argument("--before-package", action="store_true")
    args = parser.parse_args()
    if not args.before_package or os.environ.get("CARGO_PACKAGER_FORMAT") == "dmg":
        finalize(args.bundle)
