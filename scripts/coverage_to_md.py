
import json
import sys
from pathlib import Path

data = json.loads(Path(sys.argv[1]).read_text())

files = data.get("data", [{}])[0].get("files", [])

print("# Rust Code Coverage\n")
print("| Datei | Regions | Functions | Lines |")
print("|---|---:|---:|---:|")

totals = {
    "regions": [0, 0],
    "functions": [0, 0],
    "lines": [0, 0],
}

for file in files:
    name = file["filename"].split("/")[-1]
    summary = file["summary"]

    def get_counts(key):
        item = summary[key]
        return item["covered"], item["count"]

    regions = get_counts("regions")
    functions = get_counts("functions")
    lines = get_counts("lines")

    for key, values in [
        ("regions", regions),
        ("functions", functions),
        ("lines", lines),
    ]:
        totals[key][0] += values[0]
        totals[key][1] += values[1]

    def percentage(values):
        covered, total = values
        return f"{covered / total * 100:.1f}%" if total else "N/A"

    print(
        f"| `{name}` "
        f"| {percentage(regions)} "
        f"| {percentage(functions)} "
        f"| {percentage(lines)} |"
    )

print("\n## Overall Coverage\n")

for key, values in totals.items():
    print(f"- **{key.capitalize()}:** {percentage(values)}")
