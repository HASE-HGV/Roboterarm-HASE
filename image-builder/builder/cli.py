#!/usr/bin/env python3
"""Build a customized Raspberry Pi OS image from an existing .img file."""

from __future__ import annotations

import argparse
import hashlib
import ipaddress
import json
import math
import os
import re
import shutil
import subprocess
import sys
import tarfile
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any

import yaml


class BuildError(RuntimeError):
    pass


def run(args: list[str], *, check: bool = True, **kwargs: Any) -> subprocess.CompletedProcess:
    result = subprocess.run(args, check=False, **kwargs)
    if check and result.returncode:
        raise BuildError(f"Command failed ({result.returncode}): {' '.join(args)}")
    return result


def require_tool(name: str) -> None:
    if shutil.which(name) is None:
        raise BuildError(f"Required tool is missing from the builder container: {name}")


def load_config(path: Path) -> dict[str, Any]:
    try:
        data = yaml.safe_load(path.read_text())
    except (OSError, yaml.YAMLError) as exc:
        raise BuildError(f"Cannot read configuration {path}: {exc}") from exc
    if not isinstance(data, dict):
        raise BuildError("Configuration root must be a YAML mapping")
    return data


def config_path(config_file: Path, value: str) -> Path:
    """Resolve a config-relative source path, including paths under /project."""
    path = Path(value)
    if path.is_absolute():
        return path
    candidate = (config_file.parent / path).resolve()
    project = Path("/project").resolve()
    try:
        candidate.relative_to(project)
    except ValueError as exc:
        raise BuildError(f"Config source must be inside the mounted project: {value}") from exc
    return candidate


def target_path(rootfs: Path, destination: str) -> Path:
    target = (rootfs / destination.lstrip("/")).resolve()
    try:
        target.relative_to(rootfs.resolve())
    except ValueError as exc:
        raise BuildError(f"Destination must stay inside the target root filesystem: {destination}") from exc
    return target


def detect_architecture(rootfs: Path) -> str:
    status = rootfs / "var/lib/dpkg/status"
    if status.is_file():
        for stanza in status.read_text(errors="replace").split("\n\n"):
            if re.search(r"(?m)^Package: dpkg$", stanza):
                match = re.search(r"(?m)^Architecture: (arm64|armhf)$", stanza)
                if match:
                    return match.group(1)
    raise BuildError("Cannot determine target architecture from the dpkg database")


def image_partitions(image: Path) -> tuple[list[dict[str, str]], list[str]]:
    """Attach individual image partitions by offset; does not depend on loop pN nodes."""
    table = json.loads(run(["sfdisk", "--json", str(image)], capture_output=True, text=True).stdout)["partitiontable"]
    sector_size = int(table.get("sectorsize", 512))
    rows: list[dict[str, str]] = []
    devices: list[str] = []
    try:
        for partition in table.get("partitions", []):
            offset = int(partition["start"]) * sector_size
            size = int(partition["size"]) * sector_size
            node = str(partition.get("node", ""))
            number_match = re.search(r"(\d+)$", node)
            if not number_match:
                raise BuildError(f"Cannot determine partition number for {node or partition}")
            device = run(
                ["losetup", "--find", "--show", "--offset", str(offset), "--sizelimit", str(size), str(image)],
                capture_output=True,
                text=True,
            ).stdout.strip()
            devices.append(device)
            probe = run(["blkid", "-p", "-o", "export", device], check=False, capture_output=True, text=True)
            metadata = {}
            for line in probe.stdout.splitlines():
                if "=" in line:
                    key, value = line.split("=", 1)
                    metadata[key] = value
            rows.append(
                {
                    "PATH": device,
                    "LABEL": metadata.get("LABEL", ""),
                    "FSTYPE": metadata.get("TYPE", ""),
                    "PARTLABEL": str(partition.get("name", "")),
                    "TYPE": str(partition.get("type", "")),
                    "PARTNUM": number_match.group(1),
                    "START_SECTOR": str(partition["start"]),
                    "SIZE_SECTORS": str(partition["size"]),
                    "SECTOR_SIZE": str(sector_size),
                }
            )
    except Exception:
        for device in reversed(devices):
            run(["losetup", "--detach", device], check=False)
        raise
    return rows, devices


def root_partition_row(rows: list[dict[str, str]]) -> dict[str, str]:
    root_device, _ = select_partitions(rows)
    return next(row for row in rows if row["PATH"] == root_device)


def detach_partitions(devices: list[str]) -> None:
    for device in reversed(devices):
        run(["losetup", "--detach", device], check=False)


def check_ext_filesystem(device: str) -> None:
    result = run(["e2fsck", "-f", "-y", device], check=False)
    # e2fsck returns 1 when it fixed filesystem errors; this is safe to continue.
    if result.returncode not in {0, 1}:
        raise BuildError(f"e2fsck failed for {device} with status {result.returncode}")


def resize_partition(image: Path, partition_number: str, end: str) -> None:
    run(["parted", "-s", "-f", str(image), "unit", "s", "resizepart", partition_number, end])


def shrink_partition(
    image: Path,
    partition_number: str,
    start_sector: int,
    end_sector: int,
    sector_size: int,
    minimum_partition_bytes: int,
) -> None:
    """Safely shrink a partition after its filesystem has already been reduced."""
    def get_partition_table() -> dict[str, Any]:
        result = run(["sfdisk", "--json", str(image)], capture_output=True, text=True)
        return json.loads(result.stdout)["partitiontable"]

    def get_number(partition: dict[str, Any]) -> str | None:
        match = re.search(r"(\d+)$", str(partition.get("node", "")))
        return match.group(1) if match else None

    table = get_partition_table()
    matches = [
        partition
        for partition in table.get("partitions", [])
        if get_number(partition) == partition_number
    ]
    if len(matches) != 1:
        raise BuildError(f"Cannot uniquely identify partition {partition_number} in {image}")

    partition = matches[0]
    current_start = int(partition["start"])
    current_end = current_start + int(partition["size"]) - 1
    if current_start != start_sector:
        raise BuildError(f"Partition {partition_number} start changed unexpectedly before shrinking")
    if end_sector >= current_end:
        raise BuildError(f"Refusing to shrink partition {partition_number} to a non-smaller size")
    target_partition_bytes = (end_sector - start_sector + 1) * sector_size
    if end_sector < start_sector or target_partition_bytes < minimum_partition_bytes:
        raise BuildError(
            f"Refusing to shrink partition {partition_number}: the requested end does not leave enough room "
            "for its filesystem and safety margin"
        )

    # Parted's --script mode always answers "no" to this safety prompt. Its
    # pretend-input-tty option lets us answer "Yes" only after the checks above,
    # and only on the disposable working image whose filesystem was shrunk.
    run(
        [
            "parted",
            "-f",
            str(image),
            "---pretend-input-tty",
            "unit",
            "s",
            "resizepart",
            partition_number,
            f"{end_sector}s",
        ],
        input="Yes\n",
        text=True,
        env={**os.environ, "LC_ALL": "C"},
    )

    updated_table = get_partition_table()
    updated = [
        candidate
        for candidate in updated_table.get("partitions", [])
        if get_number(candidate) == partition_number
    ]
    if len(updated) != 1:
        raise BuildError(f"Cannot verify resized partition {partition_number} in {image}")
    actual_start = int(updated[0]["start"])
    actual_end = actual_start + int(updated[0]["size"]) - 1
    if actual_start != start_sector or actual_end != end_sector:
        raise BuildError(
            f"Partition {partition_number} resize was not applied as requested "
            f"(expected sectors {start_sector}-{end_sector}, found {actual_start}-{actual_end})"
        )


def ensure_root_is_last(rows: list[dict[str, str]], root: dict[str, str]) -> None:
    if root["FSTYPE"] not in {"ext2", "ext3", "ext4"}:
        raise BuildError(f"Automatic image resizing requires an ext filesystem; found {root['FSTYPE'] or 'unknown'}")
    last = max(rows, key=lambda row: int(row["START_SECTOR"]) + int(row["SIZE_SECTORS"]))
    if root["PARTNUM"] != last["PARTNUM"]:
        raise BuildError("Automatic image resizing requires the Linux root partition to be the final partition")


def grow_working_image(image: Path, max_size_bytes: int) -> None:
    """Grow the temporary image and root filesystem before packages are installed."""
    if image.stat().st_size > max_size_bytes:
        raise BuildError(
            f"Input image is already {image.stat().st_size / 1_000_000_000:.2f} GB, "
            f"larger than image.max_size_gb ({max_size_bytes / 1_000_000_000:.2f} GB)"
        )

    rows, devices = image_partitions(image)
    try:
        root = root_partition_row(rows)
        ensure_root_is_last(rows, root)
        root_number = root["PARTNUM"]
    finally:
        detach_partitions(devices)

    # Extending the sparse backing file first gives parted room to enlarge the
    # root partition without allocating all intervening blocks on disk.
    with image.open("r+b") as stream:
        stream.truncate(max_size_bytes)
    resize_partition(image, root_number, "100%")

    rows, devices = image_partitions(image)
    try:
        root = root_partition_row(rows)
        check_ext_filesystem(root["PATH"])
        run(["resize2fs", "-p", root["PATH"]])
    finally:
        detach_partitions(devices)
    print(f"[image] Expanded temporary root partition to a {max_size_bytes / 1_000_000_000:.2f} GB image limit", flush=True)


def filesystem_geometry(device: str) -> tuple[int, int]:
    details = run(["dumpe2fs", "-h", device], capture_output=True, text=True).stdout
    blocks = re.search(r"(?m)^Block count:\s*(\d+)\s*$", details)
    block_size = re.search(r"(?m)^Block size:\s*(\d+)\s*$", details)
    if not blocks or not block_size:
        raise BuildError(f"Cannot read ext filesystem geometry from {device}")
    return int(blocks.group(1)), int(block_size.group(1))


def shrink_final_image(image: Path, max_size_bytes: int, reserve_bytes: int) -> None:
    """Compact the final image while leaving runtime headroom in its root filesystem."""
    rows, devices = image_partitions(image)
    try:
        root = root_partition_row(rows)
        ensure_root_is_last(rows, root)
        root_number = root["PARTNUM"]
        start_sector = int(root["START_SECTOR"])
        sector_size = int(root["SECTOR_SIZE"])
        original_fs_size = int(root["SIZE_SECTORS"]) * sector_size
        check_ext_filesystem(root["PATH"])
        run(["resize2fs", "-p", "-M", root["PATH"]])
        minimum_blocks, block_size = filesystem_geometry(root["PATH"])
    finally:
        detach_partitions(devices)

    target_blocks = math.ceil((minimum_blocks * block_size + reserve_bytes) / block_size)
    target_fs_bytes = target_blocks * block_size
    partition_slack_bytes = 16 * 1024 * 1024
    partition_bytes = target_fs_bytes + partition_slack_bytes
    partition_sectors = math.ceil(partition_bytes / sector_size)
    end_sector = start_sector + partition_sectors - 1
    image_size = (end_sector + 1) * sector_size + 1024 * 1024
    if image_size > max_size_bytes:
        needed = image_size / 1_000_000_000
        limit = max_size_bytes / 1_000_000_000
        raise BuildError(
            f"Finished root filesystem needs {needed:.2f} GB including the configured "
            f"{reserve_bytes / 1_000_000_000:.2f} GB reserve, exceeding the {limit:.2f} GB image limit. "
            "Reduce installed packages or Guacamole image storage, or raise image.max_size_gb (up to 16)."
        )

    # Grow back from the absolute minimum to the desired runtime free-space reserve.
    rows, devices = image_partitions(image)
    try:
        root = root_partition_row(rows)
        run(["resize2fs", "-p", root["PATH"], str(target_blocks)])
        final_blocks, block_size = filesystem_geometry(root["PATH"])
        target_fs_bytes = final_blocks * block_size
    finally:
        detach_partitions(devices)

    partition_bytes = target_fs_bytes + partition_slack_bytes
    partition_sectors = math.ceil(partition_bytes / sector_size)
    end_sector = start_sector + partition_sectors - 1
    image_size = (end_sector + 1) * sector_size + 1024 * 1024
    if image_size > max_size_bytes:
        raise BuildError("Final image exceeds the configured maximum size after filesystem resizing")
    shrink_partition(
        image,
        root_number,
        start_sector,
        end_sector,
        sector_size,
        target_fs_bytes + partition_slack_bytes,
    )
    with image.open("r+b") as stream:
        stream.truncate(image_size)
    # Reposition the backup GPT header after truncating GPT-based images.
    run(["parted", "-s", "-f", str(image), "print"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

    freed = max(0, original_fs_size - target_fs_bytes) / 1_000_000_000
    print(
        f"[image] Compacted final image to {image_size / 1_000_000_000:.2f} GB "
        f"with about {reserve_bytes / 1_000_000_000:.2f} GB filesystem reserve "
        f"(reclaimed {freed:.2f} GB from the temporary root partition)",
        flush=True,
    )


def image_size_settings(cfg: dict[str, Any]) -> tuple[int, int]:
    settings = cfg.get("image", {}) or {}
    try:
        max_size_gb = float(settings.get("max_size_gb", 16))
        reserve_gb = float(settings.get("reserve_gb", 2))
    except (AttributeError, TypeError, ValueError) as exc:
        raise BuildError("image.max_size_gb and image.reserve_gb must be numbers") from exc
    if not math.isfinite(max_size_gb) or max_size_gb <= 0 or max_size_gb > 16:
        raise BuildError("image.max_size_gb must be greater than 0 and no more than 16 GB")
    if not math.isfinite(reserve_gb) or reserve_gb < 0 or reserve_gb >= max_size_gb:
        raise BuildError("image.reserve_gb must be non-negative and smaller than image.max_size_gb")
    # Decimal GB matches SD card and file size labels and guarantees the final
    # image stays within the user's 16 GB limit.
    return int(max_size_gb * 1_000_000_000), int(reserve_gb * 1_000_000_000)


def select_partitions(rows: list[dict[str, str]]) -> tuple[str, str | None]:
    roots = [r["PATH"] for r in rows if r["LABEL"].lower() in {"rootfs", "root"} or r["PARTLABEL"].lower() in {"rootfs", "root"}]
    if not roots:
        roots = [r["PATH"] for r in rows if r["FSTYPE"] in {"ext4", "ext3", "ext2"}]
    if len(roots) != 1:
        summary = [f"{r['PATH']} label={r['LABEL']!r} partlabel={r['PARTLABEL']!r} fs={r['FSTYPE']!r}" for r in rows]
        raise BuildError(f"Could not identify exactly one Linux root partition; candidates: {roots}; partitions: {summary}")
    boots = [r["PATH"] for r in rows if r["LABEL"].lower() in {"boot", "bootfs"} or r["PARTLABEL"].lower() in {"boot", "bootfs"}]
    if len(boots) > 1:
        raise BuildError(f"Found multiple boot partitions: {boots}")
    return roots[0], boots[0] if boots else None


def bind_mounts(rootfs: Path) -> list[Path]:
    mounts: list[Path] = []
    for name in ("dev", "proc", "sys", "run"):
        target = rootfs / name
        target.mkdir(parents=True, exist_ok=True)
        run(["mount", "--rbind", f"/{name}", str(target)])
        run(["mount", "--make-rslave", str(target)])
        mounts.append(target)
    return mounts


def chroot(rootfs: Path, command: list[str], *, check: bool = True, **kwargs: Any) -> subprocess.CompletedProcess:
    return run(["chroot", str(rootfs), *command], check=check, **kwargs)


def package_stage(rootfs: Path, cfg: dict[str, Any]) -> None:
    packages = cfg.get("packages", {}) or {}
    if packages.get("update", True):
        print("[packages] Updating installed packages", flush=True)
        chroot(rootfs, ["/usr/bin/env", "DEBIAN_FRONTEND=noninteractive", "apt-get", "update"])
        chroot(rootfs, ["/usr/bin/env", "DEBIAN_FRONTEND=noninteractive", "apt-get", "-y", "full-upgrade"])

    wanted = list(dict.fromkeys(packages.get("keep", []) + packages.get("extra", [])))
    netplan = cfg.get("netplan", {}) or {}
    if netplan.get("enabled"):
        wanted.append("netplan.io")
        renderer = netplan.get("renderer")
        if renderer == "NetworkManager":
            wanted.extend(["network-manager", "iw"])
        elif renderer == "networkd":
            wanted.append("systemd")
        elif renderer:
            raise BuildError("netplan.renderer must be NetworkManager or networkd")
    if (cfg.get("guacamole", {}) or {}).get("enabled"):
        wanted.extend(["openssh-server", "docker.io"])
    if wanted:
        print(f"[packages] Ensuring requested packages: {', '.join(wanted)}", flush=True)
        chroot(rootfs, ["/usr/bin/env", "DEBIAN_FRONTEND=noninteractive", "apt-get", "install", "-y", *wanted])

    if packages.get("prune", False):
        # Mark the existing package set automatic, then mark the declared roots as
        # manual. APT computes and preserves their dependency closure on autoremove.
        installed = chroot(rootfs, ["dpkg-query", "-W", "-f=${binary:Package}\\n"], capture_output=True, text=True).stdout.splitlines()
        for start in range(0, len(installed), 250):
            batch = installed[start : start + 250]
            if batch:
                chroot(rootfs, ["apt-mark", "auto", *batch], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        if wanted:
            chroot(rootfs, ["apt-mark", "manual", *wanted])
        print("[packages] Removing unneeded automatic packages", flush=True)
        chroot(rootfs, ["/usr/bin/env", "DEBIAN_FRONTEND=noninteractive", "apt-get", "-y", "autoremove", "--purge"])
    else:
        print("[packages] Pruning disabled; preserving the existing package set", flush=True)

    print("[packages] Clearing cached APT downloads and package indexes", flush=True)
    chroot(rootfs, ["apt-get", "clean"])
    apt_lists = rootfs / "var/lib/apt/lists"
    if apt_lists.is_dir():
        for item in apt_lists.iterdir():
            if item.name == "partial":
                continue
            if item.is_dir() and not item.is_symlink():
                shutil.rmtree(item)
            else:
                item.unlink()


def copy_declared(config_file: Path, rootfs: Path, entries: list[dict[str, Any]], *, label: str) -> None:
    for entry in entries:
        source = config_path(config_file, str(entry["source"]))
        if not source.exists():
            raise BuildError(f"{label} source does not exist: {source}")
        destination = target_path(rootfs, str(entry["destination"]))
        destination.parent.mkdir(parents=True, exist_ok=True)
        if source.is_dir():
            shutil.copytree(source, destination, dirs_exist_ok=True, symlinks=True)
        else:
            shutil.copy2(source, destination)
        if entry.get("mode"):
            destination.chmod(int(str(entry["mode"]), 8))
        print(f"[{label}] {source} -> /{destination.relative_to(rootfs)}", flush=True)


def apply_hostname(rootfs: Path, cfg: dict[str, Any]) -> None:
    hostname = cfg.get("hostname")
    if hostname is None:
        return
    if not isinstance(hostname, str) or not re.fullmatch(
        r"[A-Za-z0-9](?:[A-Za-z0-9-]{0,61}[A-Za-z0-9])?", hostname
    ):
        raise BuildError("hostname must be a valid DNS hostname label of 1 to 63 characters")

    hostname_path = target_path(rootfs, "/etc/hostname")
    hostname_path.parent.mkdir(parents=True, exist_ok=True)
    hostname_path.write_text(f"{hostname}\n")

    hosts_path = target_path(rootfs, "/etc/hosts")
    lines = hosts_path.read_text(errors="replace").splitlines() if hosts_path.is_file() else []
    updated_lines: list[str] = []
    found_hostname_address = False
    for line in lines:
        host_fields, has_comment, comment = line.partition("#")
        fields = host_fields.split()
        if fields and fields[0] == "127.0.1.1":
            if not found_hostname_address:
                replacement = f"127.0.1.1\t{hostname}"
                if has_comment:
                    replacement += f"  #{comment}"
                updated_lines.append(replacement)
                found_hostname_address = True
            continue
        updated_lines.append(line)
    if not found_hostname_address:
        updated_lines.append(f"127.0.1.1\t{hostname}")
    hosts_path.parent.mkdir(parents=True, exist_ok=True)
    hosts_path.write_text("\n".join(updated_lines).rstrip("\n") + "\n")
    print(f"[hostname] Set target hostname to {hostname}", flush=True)


def apply_netplan(config_file: Path, rootfs: Path, cfg: dict[str, Any]) -> None:
    netplan = cfg.get("netplan", {}) or {}
    if not netplan.get("enabled"):
        return
    renderer = netplan.get("renderer", "networkd")
    if renderer not in {"NetworkManager", "networkd"}:
        raise BuildError("netplan.renderer must be NetworkManager or networkd")
    configs = netplan.get("configs", [])
    if not configs:
        raise BuildError("netplan.enabled requires at least one config file")
    for entry in configs:
        source = config_path(config_file, str(entry["source"]))
        try:
            document = yaml.safe_load(source.read_text())
        except (OSError, yaml.YAMLError) as exc:
            raise BuildError(f"Cannot parse Netplan config {source}: {exc}") from exc
        network = document.get("network") if isinstance(document, dict) else None
        if not isinstance(network, dict):
            raise BuildError(f"Netplan config must contain a network mapping: {source}")
        configured_renderer = network.get("renderer", "networkd")
        if configured_renderer != renderer:
            raise BuildError(f"Netplan renderer in {source} ({configured_renderer}) does not match netplan.renderer ({renderer})")
    copy_declared(config_file, rootfs, configs, label="netplan")
    if renderer == "NetworkManager":
        # Netplan generates NetworkManager profiles from the YAML at boot;
        # make sure the selected backend is actually started on headless images.
        run(["systemctl", f"--root={rootfs}", "enable", "NetworkManager.service"])


def apply_systemd(config_file: Path, rootfs: Path, cfg: dict[str, Any]) -> None:
    systemd = cfg.get("systemd", {}) or {}
    units = systemd.get("units", [])
    copy_declared(config_file, rootfs, [u for u in units if "source" in u], label="systemd")
    for unit in units:
        if "name" not in unit:
            unit_name = Path(str(unit["destination"])).name
        else:
            unit_name = str(unit["name"])
        state = unit.get("state", "enabled" if unit.get("enabled", False) else "disabled")
        if state not in {"enabled", "disabled", "masked"}:
            raise BuildError(f"Invalid systemd state for {unit_name}: {state}")
        if state == "masked":
            target = rootfs / "etc/systemd/system" / unit_name
            target.parent.mkdir(parents=True, exist_ok=True)
            if target.exists() or target.is_symlink():
                target.unlink()
            target.symlink_to("/dev/null")
        else:
            verb = "enable" if state == "enabled" else "disable"
            run(["systemctl", f"--root={rootfs}", verb, unit_name])


def guacamole_settings(cfg: dict[str, Any]) -> dict[str, Any]:
    guac = cfg.get("guacamole", {}) or {}
    settings = {
        "username": str(guac.get("username", "root")),
        "password": str(guac.get("password", "root")),
        "ssh_host": str(guac.get("ssh_host", "host.docker.internal")),
        "ssh_port": int(guac.get("ssh_port", 22)),
        "web_port": int(guac.get("web_port", 2000)),
        "bind_address": str(guac.get("bind_address", "127.0.0.1")),
        "target_bind_address": str(guac.get("target_bind_address", "127.0.0.1")),
        "version": str(guac.get("version", "1.6.0")),
    }
    if not settings["username"] or not settings["password"] or not settings["ssh_host"]:
        raise BuildError("guacamole.username, password, and ssh_host must not be empty")
    for label in ("ssh_port", "web_port"):
        if not 1 <= settings[label] <= 65535:
            raise BuildError(f"guacamole.{label} must be between 1 and 65535")
    for label in ("bind_address", "target_bind_address"):
        try:
            ipaddress.IPv4Address(settings[label])
        except ipaddress.AddressValueError as exc:
            raise BuildError(f"guacamole.{label} must be an IPv4 address (for example 127.0.0.1 or 0.0.0.0)") from exc
    if not re.fullmatch(r"[A-Za-z0-9_.-]+", settings["version"]):
        raise BuildError("guacamole.version contains unsupported characters")
    return settings


def prepare_guacamole_images(cfg: dict[str, Any], architecture: str, work_dir: Path) -> list[Path]:
    guac = cfg.get("guacamole", {}) or {}
    if not guac.get("enabled"):
        return []
    if architecture != "arm64":
        raise BuildError("On-Pi Guacamole containers require an arm64 Raspberry Pi OS image")
    require_tool("skopeo")
    settings = guacamole_settings(cfg)
    images = [
        ("guacd", f"guacamole/guacd:{settings['version']}"),
        ("guacamole", f"guacamole/guacamole:{settings['version']}"),
    ]
    archive_paths = []
    image_dir = work_dir / "guacamole-images"
    image_dir.mkdir(parents=True, exist_ok=True)
    for name, reference in images:
        archive = image_dir / f"{name}-{settings['version']}-arm64.tar"
        valid_archive = False
        if archive.is_file():
            try:
                with tarfile.open(archive, "r:") as image_archive:
                    valid_archive = any(member.name == "manifest.json" for member in image_archive.getmembers())
            except (OSError, tarfile.TarError):
                valid_archive = False
        if valid_archive:
            print(f"[guacamole] Reusing preloaded image archive {archive.name}", flush=True)
        else:
            archive.unlink(missing_ok=True)
            print(f"[guacamole] Preloading linux/arm64 image {reference}", flush=True)
            run([
                "skopeo", "copy", "--override-os", "linux", "--override-arch", "arm64",
                f"docker://docker.io/{reference}",
                f"docker-archive:{archive}:{reference}",
            ])
        archive_paths.append(archive)
    return archive_paths


def configure_guacamole_target(rootfs: Path, cfg: dict[str, Any], archives: list[Path]) -> None:
    guac = cfg.get("guacamole", {}) or {}
    if not guac.get("enabled"):
        return

    if len(archives) != 2:
        raise BuildError("Guacamole image archives were not prepared")
    settings = guacamole_settings(cfg)
    version = settings["version"]

    # The default connection forwards Guacamole credentials to the Pi over
    # SSH. Raspberry Pi OS locks root by default, so provision the configured
    # password and explicitly allow password-based root SSH when root is used.
    if settings["username"] == "root":
        run(
            ["chroot", str(rootfs), "/usr/sbin/chpasswd"],
            input=f"root:{settings['password']}\n",
            text=True,
        )
        sshd_config = target_path(rootfs, "/etc/ssh/sshd_config")
        current_config = sshd_config.read_text(errors="replace") if sshd_config.exists() else ""
        sshd_config.parent.mkdir(parents=True, exist_ok=True)
        sshd_config.write_text(
            "PermitRootLogin yes\nPasswordAuthentication yes\n" + current_config
        )

    image_root = target_path(rootfs, "/opt/guacamole/images")
    image_root.mkdir(parents=True, exist_ok=True)
    for archive in archives:
        destination = image_root / archive.name
        shutil.copy2(archive, destination)
        destination.chmod(0o644)

    mapping = ET.Element("user-mapping")
    authorize = ET.SubElement(
        mapping,
        "authorize",
        {
            "username": settings["username"],
            "password": hashlib.md5(settings["password"].encode("utf-8"), usedforsecurity=False).hexdigest(),
            "encoding": "md5",
        },
    )
    connection = ET.SubElement(authorize, "connection", {"name": "Raspberry Pi SSH"})
    ET.SubElement(connection, "protocol").text = "ssh"
    ET.SubElement(connection, "param", {"name": "hostname"}).text = settings["ssh_host"]
    ET.SubElement(connection, "param", {"name": "port"}).text = str(settings["ssh_port"])
    ET.SubElement(connection, "param", {"name": "username"}).text = "${GUAC_USERNAME}"
    ET.SubElement(connection, "param", {"name": "password"}).text = "${GUAC_PASSWORD}"
    ET.indent(mapping, space="  ")
    mapping_path = target_path(rootfs, "/etc/guacamole/user-mapping.xml")
    mapping_path.parent.mkdir(parents=True, exist_ok=True)
    ET.ElementTree(mapping).write(mapping_path, encoding="utf-8", xml_declaration=True)
    # The official Guacamole web container runs as UID/GID 1001, not root.
    # Keep the bind-mounted file readable by that unprivileged process; 0600
    # leaves it unreadable when this file is root-owned in the Pi image.
    mapping_path.chmod(0o644)

    guacd_image = f"guacamole/guacd:{version}"
    guac_image = f"guacamole/guacamole:{version}"
    start_script = f'''#!/bin/sh
set -eu

load_image() {{
  image="$1"
  archive="$2"
  if ! docker image inspect "$image" >/dev/null 2>&1; then
    if [ ! -f "$archive" ]; then
      echo "Missing preloaded Guacamole image archive: $archive" >&2
      return 1
    fi
    docker load --input "$archive"
  fi
  rm -f "$archive"
}}

ensure_container() {{
  name="$1"
  shift
  if docker container inspect "$name" >/dev/null 2>&1; then
    running=$(docker inspect --format '{{{{.State.Running}}}}' "$name")
    if [ "$running" != true ]; then
      docker start "$name"
    fi
  else
    docker run --detach --name "$name" --restart unless-stopped "$@"
  fi
}}

load_image {json.dumps(guacd_image)} /opt/guacamole/images/{archives[0].name}
load_image {json.dumps(guac_image)} /opt/guacamole/images/{archives[1].name}
docker network inspect guac >/dev/null 2>&1 || docker network create guac
ensure_container guacd --network guac --add-host host.docker.internal:host-gateway {json.dumps(guacd_image)}
ensure_container guacamole --network guac --publish {json.dumps(f"{settings['target_bind_address']}:{settings['web_port']}:8080")} \\
  --env GUACD_HOSTNAME=guacd --env GUACD_PORT=4822 --env WEBAPP_CONTEXT=ROOT \\
  --volume /etc/guacamole/user-mapping.xml:/etc/guacamole/user-mapping.xml:ro {json.dumps(guac_image)}
'''
    start_path = target_path(rootfs, "/usr/local/sbin/guacamole-start")
    start_path.parent.mkdir(parents=True, exist_ok=True)
    start_path.write_text(start_script)
    start_path.chmod(0o755)
    stop_path = target_path(rootfs, "/usr/local/sbin/guacamole-stop")
    stop_path.write_text("#!/bin/sh\ndocker stop guacamole guacd >/dev/null 2>&1 || true\n")
    stop_path.chmod(0o755)
    unit_path = target_path(rootfs, "/etc/systemd/system/guacamole.service")
    unit_path.parent.mkdir(parents=True, exist_ok=True)
    unit_path.write_text(
        "[Unit]\n"
        "Description=Apache Guacamole remote desktop gateway\n"
        "Requires=docker.service\n"
        "After=docker.service network-online.target ssh.service\n"
        "Wants=network-online.target\n\n"
        "[Service]\n"
        "Type=oneshot\n"
        "RemainAfterExit=yes\n"
        "ExecStart=/usr/local/sbin/guacamole-start\n"
        "ExecStop=/usr/local/sbin/guacamole-stop\n\n"
        "[Install]\n"
        "WantedBy=multi-user.target\n"
    )
    for service in ("ssh.service", "docker.service", "guacamole.service"):
        run(["systemctl", f"--root={rootfs}", "enable", service])


def generate_guacamole_bundle(output: Path, cfg: dict[str, Any]) -> Path | None:
    guac = cfg.get("guacamole", {}) or {}
    if not guac.get("enabled"):
        return None

    settings = guacamole_settings(cfg)
    username = settings["username"]
    password = settings["password"]
    ssh_host = settings["ssh_host"]
    ssh_port = settings["ssh_port"]
    web_port = settings["web_port"]
    bind_address = settings["bind_address"]
    version = settings["version"]

    bundle = output.parent / f"{output.stem}-guacamole"
    bundle.mkdir(parents=True, exist_ok=True)
    mapping = ET.Element("user-mapping")
    authorize = ET.SubElement(
        mapping,
        "authorize",
        {
            "username": username,
            "password": hashlib.md5(password.encode("utf-8"), usedforsecurity=False).hexdigest(),
            "encoding": "md5",
        },
    )
    connection = ET.SubElement(authorize, "connection", {"name": "Raspberry Pi SSH"})
    ET.SubElement(connection, "protocol").text = "ssh"
    ET.SubElement(connection, "param", {"name": "hostname"}).text = ssh_host
    ET.SubElement(connection, "param", {"name": "port"}).text = str(ssh_port)
    ET.SubElement(connection, "param", {"name": "username"}).text = "${GUAC_USERNAME}"
    ET.SubElement(connection, "param", {"name": "password"}).text = "${GUAC_PASSWORD}"
    ET.indent(mapping, space="  ")
    mapping_path = bundle / "user-mapping.xml"
    ET.ElementTree(mapping).write(mapping_path, encoding="utf-8", xml_declaration=True)
    # Guacamole runs as an unprivileged user in the official container, so
    # the bind-mounted mapping must be readable inside the container.
    mapping_path.chmod(0o644)

    guac_image = f"guacamole/guacamole:{version}"
    guacd_image = f"guacamole/guacd:{version}"
    published_port = f"{bind_address}:{web_port}:8080"
    compose = (
        "services:\n"
        "  guacd:\n"
        f"    image: {json.dumps(guacd_image)}\n"
        "    restart: unless-stopped\n"
        "    networks: [guac]\n"
        "  guacamole:\n"
        f"    image: {json.dumps(guac_image)}\n"
        "    restart: unless-stopped\n"
        "    depends_on: [guacd]\n"
        "    environment:\n"
        "      GUACD_HOSTNAME: guacd\n"
        "      GUACD_PORT: \"4822\"\n"
        "      WEBAPP_CONTEXT: ROOT\n"
        "    volumes:\n"
        "      - ./user-mapping.xml:/etc/guacamole/user-mapping.xml:ro\n"
        f"    ports: [{json.dumps(published_port)}]\n"
        "    networks: [guac]\n"
        "networks:\n"
        "  guac:\n"
        "    driver: bridge\n"
    )
    (bundle / "compose.yaml").write_text(compose)
    (bundle / "README.md").write_text(
        "# Guacamole-Testbundle\n\n"
        "Das erzeugte Raspberry-Pi-Image startet Guacamole selbst beim Boot. Dieses Bundle ist optional und dient zum lokalen Testen auf einem Rechner mit Docker.\n\n"
        "1. Sicherstellen, dass der konfigurierte SSH-Host erreichbar ist. `host.docker.internal` zeigt im Container auf den Docker-Host.\n"
        "2. In diesem Ordner `docker compose up -d` ausführen.\n"
        f"3. Im Browser `http://{bind_address}:{web_port}/` öffnen und mit dem in der Builder-Konfiguration gesetzten Guacamole-Benutzer anmelden.\n"
        "4. `Raspberry Pi SSH` auswählen. Guacamole reicht die beim Weblogin eingegebenen Daten automatisch als SSH-Zugangsdaten weiter.\n\n"
        "Der Zielhost muss dieselben Zugangsdaten akzeptieren. Das Bundle lädt die offiziellen Container beim ersten Start aus dem Internet.\n"
    )
    bundle.chmod(0o700)
    return bundle


def run_hooks(config_file: Path, rootfs: Path, hooks: list[str], stage: str) -> None:
    for hook in hooks:
        script = config_path(config_file, hook)
        if not script.is_file():
            raise BuildError(f"Hook does not exist: {script}")
        print(f"[hook:{stage}] {script}", flush=True)
        run(["bash", str(script)], env={**os.environ, "ROOTFS": str(rootfs), "BUILD_STAGE": stage})


def install_rustctl(rootfs: Path, cfg: dict[str, Any], architecture: str) -> None:
    settings = cfg.get("rustctl", {}) or {}
    if not settings.get("enabled", True):
        return
    if architecture != "arm64":
        raise BuildError("The bundled rustctl artifact currently supports arm64 images only")
    artifact = Path("/opt/image-builder/artifacts/rustctl-aarch64")
    if not artifact.is_file():
        raise BuildError(f"Bundled rustctl artifact is missing: {artifact}")
    destination = rootfs / str(settings.get("install_path", "/rustctl/rustctl")).lstrip("/")
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy2(artifact, destination)
    destination.chmod(0o755)


def build(args: argparse.Namespace) -> None:
    input_image = Path(args.input).resolve()
    config_file = Path(args.config).resolve()
    output = Path(args.output).resolve()
    if not input_image.is_file():
        raise BuildError(f"Input image not found: {input_image}")
    if input_image == output:
        raise BuildError("Output image must not overwrite the input image")
    cfg = load_config(config_file)
    max_size_bytes, reserve_bytes = image_size_settings(cfg)
    for tool in (
        "losetup", "sfdisk", "blkid", "mount", "umount", "chroot", "systemctl",
        "parted", "e2fsck", "resize2fs", "dumpe2fs",
    ):
        require_tool(tool)
    if input_image.suffix.lower() == ".xz":
        require_tool("xz")

    architecture = cfg.get("architecture", "auto")
    if architecture not in {"auto", "arm64", "armhf"}:
        raise BuildError("architecture must be auto, arm64, or armhf")

    work_dir = Path("/work")
    work_dir.mkdir(parents=True, exist_ok=True)
    output.parent.mkdir(parents=True, exist_ok=True)
    preload_arch = "arm64" if architecture == "auto" else architecture
    guacamole_archives = prepare_guacamole_images(cfg, preload_arch, work_dir)
    temp_image = work_dir / f"{output.stem}.working.img"
    if temp_image.exists():
        temp_image.unlink()
    if input_image.suffix.lower() == ".xz":
        print(f"[image] Decompressing {input_image.name} to working image", flush=True)
        with temp_image.open("wb") as uncompressed:
            run(["xz", "--decompress", "--stdout", "--", str(input_image)], stdout=uncompressed)
    else:
        print(f"[image] Copying input image to {temp_image}", flush=True)
        shutil.copyfile(input_image, temp_image)
    grow_working_image(temp_image, max_size_bytes)

    partition_devices: list[str] = []
    mounts: list[Path] = []
    try:
        partitions, partition_devices = image_partitions(temp_image)
        root_partition, boot_partition = select_partitions(partitions)
        rootfs = work_dir / "rootfs"
        rootfs.mkdir(exist_ok=True)
        run(["mount", root_partition, str(rootfs)])
        mounts.append(rootfs)
        detected_arch = detect_architecture(rootfs)
        if architecture == "auto":
            architecture = detected_arch
        elif architecture != detected_arch:
            raise BuildError(f"Configured architecture {architecture} does not match image architecture {detected_arch}")
        if (cfg.get("guacamole", {}) or {}).get("enabled") and architecture != "arm64":
            raise BuildError("On-Pi Guacamole containers require an arm64 Raspberry Pi OS image")
        if boot_partition:
            candidates = [rootfs / "boot/firmware", rootfs / "boot"]
            boot_mount = next((p for p in candidates if p.exists()), candidates[0])
            boot_mount.mkdir(parents=True, exist_ok=True)
            run(["mount", boot_partition, str(boot_mount)])
            mounts.append(boot_mount)

        mounts.extend(bind_mounts(rootfs))
        if run(["chroot", str(rootfs), "/bin/true"], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode:
            raise BuildError("Cannot execute target binaries in chroot. On x86, enable ARM64 QEMU/binfmt in Docker; on Linux it may need host binfmt registration.")
        package_stage(rootfs, cfg)
        copy_declared(config_file, rootfs, cfg.get("files", []), label="file")
        copy_declared(config_file, rootfs, cfg.get("tools", []), label="tool")
        apply_hostname(rootfs, cfg)
        apply_netplan(config_file, rootfs, cfg)
        install_rustctl(rootfs, cfg, architecture)
        apply_systemd(config_file, rootfs, cfg)
        configure_guacamole_target(rootfs, cfg, guacamole_archives)
        hooks = cfg.get("hooks", {}) or {}
        if hooks.get("mode", "container") != "container":
            raise BuildError("Only hooks.mode=container is implemented; host hooks are planned")
        run_hooks(config_file, rootfs, hooks.get("after_packages", []), "after_packages")
        run_hooks(config_file, rootfs, hooks.get("before_finalize", []), "before_finalize")

        if args.test or (cfg.get("test", {}) or {}).get("shell", False):
            print("[test] Entering target chroot; type exit to continue image finalization", flush=True)
            chroot(rootfs, ["/bin/bash", "-l"], check=False)
        run(["sync"])
    finally:
        for mountpoint in reversed(mounts):
            run(["umount", "--recursive", str(mountpoint)], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        detach_partitions(partition_devices)

    shrink_final_image(temp_image, max_size_bytes, reserve_bytes)
    guac_bundle = generate_guacamole_bundle(output, cfg)
    shutil.move(str(temp_image), str(output))
    print(f"[done] Wrote {output}", flush=True)
    if guac_bundle:
        print(f"[done] Guacamole bundle: {guac_bundle}", flush=True)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    sub = parser.add_subparsers(dest="command", required=True)
    build_parser = sub.add_parser("build", help="customize an existing Raspberry Pi OS image")
    build_parser.add_argument("--input", required=True, help="input Raspberry Pi OS .img file")
    build_parser.add_argument("--config", required=True, help="YAML builder configuration")
    build_parser.add_argument("--output", required=True, help="path for the generated .img file")
    build_parser.add_argument("--test", action="store_true", help="open a target chroot shell before finalizing")
    args = parser.parse_args()
    try:
        if args.command == "build":
            build(args)
        return 0
    except (BuildError, subprocess.CalledProcessError) as exc:
        print(f"image-builder: error: {exc}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
