#!/usr/bin/env python3
"""Measure both running clients with macOS footprint, including helpers.

Open the same Gmail inbox in both apps, leave the screen unlocked, and wait
for loading to finish. This script reads process memory; it never reads mail.
"""
import argparse
import datetime
import json
from pathlib import Path
import re
import statistics
import subprocess
import time


def processes():
    groups = {"notsuperhuman": [], "Superhuman Desktop": []}
    output = subprocess.check_output(["ps", "-axo", "pid=,comm="], text=True)
    for row in output.splitlines():
        pid, _, command = row.strip().partition(" ")
        command = command.strip()
        if Path(command).name == "notsuperhuman":
            groups["notsuperhuman"].append(pid)
        elif "/Superhuman.app/Contents/" in command and Path(command).name != "ShipIt":
            groups["Superhuman Desktop"].append(pid)
    if any(not pids for pids in groups.values()):
        raise SystemExit("Open both notsuperhuman and Superhuman Desktop before measuring.")
    return groups


def footprint(pids):
    command = ["/usr/bin/footprint", "--noCategories", "--format", "bytes"]
    for pid in pids:
        command.extend(["-p", pid])
    result = subprocess.run(command, text=True, capture_output=True, check=True)
    summary = re.search(r"Summary Footprint: (\d+) B", result.stdout)
    if summary:
        return int(summary[1])
    values = re.findall(r"Footprint: (\d+) B", result.stdout)
    if len(values) != len(pids):
        raise SystemExit("footprint could not inspect every requested process.")
    return sum(map(int, values))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--samples", type=int, default=3)
    parser.add_argument("--interval", type=float, default=10)
    parser.add_argument("--client", choices=["notsuperhuman", "Superhuman Desktop"],
                        help="Measure one client while its window is in the foreground")
    options = parser.parse_args()
    if options.samples < 1 or options.interval < 0:
        parser.error("samples must be positive and interval must be nonnegative")
    measurements = {name: [] for name in processes() if not options.client or name == options.client}
    counts = {name: [] for name in measurements}
    for sample in range(options.samples):
        for name, pids in processes().items():
            if name not in measurements:
                continue
            measurements[name].append(footprint(pids))
            counts[name].append(len(pids))
        if sample + 1 < options.samples:
            time.sleep(options.interval)
    report = {
        "date": datetime.date.today().isoformat(),
        "macos": subprocess.check_output(["sw_vers", "-productVersion"], text=True).strip(),
        "hardware": subprocess.check_output(["sysctl", "-n", "hw.model"], text=True).strip(),
        "metric": "macOS footprint, bytes (1024 bytes per KiB)",
        "clients": {
            name: {
                "process_counts": counts[name],
                "samples_bytes": values,
                "median_mib": round(statistics.median(values) / 1024**2, 1),
            } for name, values in measurements.items()
        },
    }
    print(json.dumps(report, indent=2))


if __name__ == "__main__":
    main()
