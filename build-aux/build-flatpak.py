#!/usr/bin/env python3
"""Build and verify a local Linux DS Flatpak bundle."""
import argparse
import configparser
import gzip
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tomllib
import xml.etree.ElementTree as ET

APP_ID = "org.linuxds.Schnapsen"
ARCH = "x86_64"
BRANCH = "master"
PNG_MAGIC = b"\x89PNG\r\n\x1a\n"


def run(*args):
    command = [str(arg) for arg in args]
    print("+", " ".join(command), flush=True)
    subprocess.run(command, check=True)


def snapshot(root, destination):
    paths = subprocess.check_output([
        "git", "-C", root, "ls-files", "-z", "--cached", "--others", "--exclude-standard"
    ]).split(b"\0")
    destination.mkdir()
    for entry in filter(None, paths):
        source = root / entry.decode(errors="surrogateescape")
        target = destination / source.relative_to(root)
        if source.is_symlink():
            target.parent.mkdir(parents=True, exist_ok=True)
            target.symlink_to(source.readlink())
        elif source.is_file():
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(source, target)


def verify_catalog(files, version):
    relative_catalog = Path(f"share/app-info/xmls/{APP_ID}.xml.gz")
    with gzip.open(files / relative_catalog, "rb") as stream:
        catalog = ET.parse(stream).getroot()
    components = [item for item in catalog.findall("component") if item.findtext("id") == APP_ID]
    if len(components) != 1 or components[0].findtext("name") != "Schnapsen":
        raise ValueError("AppStream catalog must contain exactly one Schnapsen component")

    icon_paths = []
    for icon in components[0].findall("icon[@type='cached']"):
        width, height = int(icon.attrib["width"]), int(icon.attrib["height"])
        scale = int(icon.get("scale", "1"))
        size = f"{width}x{height}" + (f"@{scale}" if scale > 1 else "")
        relative_icon = Path(f"share/app-info/icons/flatpak/{size}/{APP_ID}.png")
        if icon.text != relative_icon.name:
            raise ValueError(f"Unexpected cached icon name: {icon.text}")
        with (files / relative_icon).open("rb") as stream:
            header = stream.read(24)
        if (
            header[:8] != PNG_MAGIC or header[12:16] != b"IHDR"
            or struct.unpack(">II", header[16:24]) != (width * scale, height * scale)
        ):
            raise ValueError(f"Invalid PNG header or size: {relative_icon}")
        icon_paths.append(str(relative_icon))
    if not icon_paths:
        raise ValueError("AppStream catalog has no cached PNG icons")

    metainfo = ET.parse(files / f"share/metainfo/{APP_ID}.metainfo.xml").getroot()
    release = metainfo.find("releases/release")
    if (
        metainfo.findtext("id") != APP_ID or metainfo.findtext("name") != "Schnapsen"
        or release is None or release.get("version") != version
    ):
        raise ValueError("Installed metainfo does not match the App ID, name or Cargo version")

    desktop = configparser.ConfigParser(interpolation=None)
    desktop.read(files / f"share/applications/{APP_ID}.desktop", encoding="utf-8")
    if desktop["Desktop Entry"]["Categories"] != "Game;CardGame;":
        raise ValueError("Unexpected desktop categories")
    return {"catalog": str(relative_catalog), "cached_png_icons": icon_paths}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("output", type=Path, help="new or empty local output directory")
    parser.add_argument("--state-dir", type=Path, help="reuse a flatpak-builder cache")
    args = parser.parse_args()
    root = Path(__file__).resolve().parent.parent
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    if any(output.iterdir()):
        parser.error("output directory must be empty")

    source = output / "source"
    snapshot(root, source)
    manifest_path = source / f"{APP_ID}.json"
    manifest = json.loads(manifest_path.read_text())
    version = tomllib.loads((source / "Cargo.toml").read_text())["package"]["version"]

    builder = [
        "flatpak", "run", f"--env=FLATPAK_USER_DIR={Path.home() / '.local/share/flatpak'}",
        "--command=flatpak-builder", "org.gnome.Builder",
        f"--arch={ARCH}", f"--default-branch={BRANCH}", "--disable-rofiles-fuse", "--jobs=8",
        f"--state-dir={args.state_dir.resolve() if args.state_dir else output / 'state'}",
    ]
    app, repo = output / "app", output / "repo"
    # Builder composes AppStream and renders cached icons during normal cleanup.
    run(*builder, app, manifest_path)
    run(
        "flatpak", "run", "--command=appstreamcli", f"--filesystem={app}:ro",
        f"{manifest['sdk']}//{manifest['runtime-version']}", "validate", "--no-net",
        app / f"files/share/metainfo/{APP_ID}.metainfo.xml",
    )
    verify_catalog(app / "files", version)
    run(*builder, "--export-only", f"--repo={repo}", app, manifest_path)
    bundle = output / f"{APP_ID}-{ARCH}-{BRANCH}.flatpak"
    run("sh", source / "build-aux/build-flatpak-bundle.sh", repo, bundle)

    verification = output / "verification-repo"
    run("ostree", f"--repo={verification}", "init", "--mode=bare-user-only")
    run("flatpak", "build-import-bundle", verification, bundle)
    run("ostree", f"--repo={verification}", "fsck")
    ref = f"app/{APP_ID}/{ARCH}/{BRANCH}"
    commit = subprocess.check_output(["ostree", f"--repo={verification}", "rev-parse", ref], text=True).strip()
    exported = subprocess.check_output(["ostree", f"--repo={repo}", "rev-parse", ref], text=True).strip()
    if commit != exported:
        raise ValueError("Imported bundle commit differs from exported commit")
    content = output / "verified-content"
    run("ostree", f"--repo={verification}", "checkout", "--user-mode", ref, content)
    evidence = verify_catalog(content / "files", version)

    metadata = configparser.ConfigParser()
    metadata.read(content / "metadata", encoding="utf-8")
    runtime = f"{manifest['runtime']}/{ARCH}/{manifest['runtime-version']}"
    if metadata["Application"]["name"] != APP_ID or metadata["Application"]["runtime"] != runtime:
        raise ValueError("Unexpected bundle App ID or runtime")
    for line in (source / "po/LINGUAS").read_text().splitlines():
        for language in line.partition("#")[0].split():
            if not (content / f"files/share/locale/{language}/LC_MESSAGES/schnapsen.mo").is_file():
                raise ValueError(f"Missing bundled translation: {language}")

    report = {"bundle": str(bundle), "ref": ref, "commit": commit, "runtime": runtime,
              "version": version, **evidence}
    (output / "verification.json").write_text(json.dumps(report, indent=2) + "\n")
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
