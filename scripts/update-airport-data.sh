#!/usr/bin/env bash
# Refreshes the worldwide reference data bundled into qdeck for the `airport` tile:
#   - data/airport/runways.csv: every runway in OurAirports (public domain), trimmed to the
#                               columns qdeck uses
#   - data/airport/routes.csv:  every callsign route in the VRS standing data (CC0, adsb.lol
#                               mirror), trimmed to Callsign,AirportCodes
#
# Both files are compiled into the binary with include_str!, so qdeck never downloads them at
# runtime. Run this every few weeks (routes change with airline schedules), check the diff,
# run `cargo test`, and commit.
set -euo pipefail

RUNWAYS_URL="https://davidmegginson.github.io/ourairports-data/runways.csv"
ROUTES_URL="https://vrs-standing-data.adsb.lol/routes.csv.gz"
# Sanity floors: an update with fewer rows than this is treated as a broken download.
MIN_RUNWAYS=40000
MIN_ROUTES=400000

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
data_dir="$repo_root/data/airport"
mkdir -p "$data_dir"

tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

echo "Downloading OurAirports runways..."
curl -sSfL --max-time 120 -o "$tmp/runways.raw.csv" "$RUNWAYS_URL"
echo "Downloading VRS routes..."
curl -sSfL --max-time 300 -o "$tmp/routes.csv.gz" "$ROUTES_URL"
gunzip -c "$tmp/routes.csv.gz" > "$tmp/routes.raw.csv"

python3 -I - "$tmp" <<'PY'
import csv, sys
tmp = sys.argv[1]

# OurAirports quotes fields and a few contain commas, so parse it as real CSV.
runway_cols = [
    "airport_ident", "closed",
    "le_ident", "le_latitude_deg", "le_longitude_deg", "le_elevation_ft", "le_heading_degT",
    "he_ident", "he_latitude_deg", "he_longitude_deg", "he_elevation_ft", "he_heading_degT",
]
with open(f"{tmp}/runways.raw.csv", newline="", encoding="utf-8") as src:
    rows = [[r[c].replace(",", " ").strip() for c in runway_cols] for r in csv.DictReader(src)]
rows.sort(key=lambda r: (r[0], r[2], r[7]))
with open(f"{tmp}/runways.csv", "w", newline="", encoding="utf-8") as out:
    out.write(",".join(runway_cols) + "\n")
    out.writelines(",".join(r) + "\n" for r in rows)

# VRS routes: keep the callsign and its dash-separated stops (e.g. CFE63BB,EGLC-LEIB).
with open(f"{tmp}/routes.raw.csv", newline="", encoding="utf-8-sig") as src:
    routes = sorted(
        {(r["Callsign"].strip(), r["AirportCodes"].strip()) for r in csv.DictReader(src)}
    )
with open(f"{tmp}/routes.csv", "w", newline="", encoding="utf-8") as out:
    out.write("Callsign,AirportCodes\n")
    out.writelines(f"{c},{a}\n" for c, a in routes if c and a)
PY

runway_rows=$(($(wc -l < "$tmp/runways.csv") - 1))
route_rows=$(($(wc -l < "$tmp/routes.csv") - 1))
echo "  $runway_rows runways, $route_rows routes"
if (( runway_rows < MIN_RUNWAYS || route_rows < MIN_ROUTES )); then
  echo "Row counts look truncated; leaving existing files untouched" >&2
  exit 1
fi

mv "$tmp/runways.csv" "$data_dir/runways.csv"
mv "$tmp/routes.csv" "$data_dir/routes.csv"
echo "Wrote $data_dir/runways.csv and $data_dir/routes.csv"
