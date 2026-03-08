# Hockey Schedule CLI

A command-line tool to fetch and display upcoming hockey games from an iCal feed. Supports filtering by team and exporting to iCal format for import into other calendar services.

## Features

- Fetch games from any iCal feed URL
- Filter to show only games for a specific team
- Display in human-readable format or export as iCal or CSV
- Configurable number of upcoming games to show

## Installation

Requires Rust. Build with:

```bash
cargo build --release
```

The binary will be at `target/release/hockey-schedule`.

## Usage

```
hockey-schedule [OPTIONS]

Options:
  -u, --url <URL>    iCal feed URL [env: HOCKEY_ICAL_URL] [default: https://example.com/hockey-schedule.ics]
  -n, --num <NUM>    Number of upcoming games to show [default: 10]
  -t, --team <TEAM>  Filter to only games involving this team (case-insensitive)
      --ics          Output as iCal format instead of human-readable text
      --csv          Output as CSV format
  -h, --help         Print help
```

## Environment Variables

You can set the iCal feed URL via an environment variable instead of passing it as a flag each time:

```bash
export HOCKEY_ICAL_URL="https://example.com/schedule.ics"
```

Add this to your `~/.bashrc` or `~/.zshrc` to make it permanent. The `--url` flag takes precedence over the environment variable if both are provided.

## Examples

Show next 10 upcoming games:

```bash
hockey-schedule --url "https://example.com/schedule.ics"
```

Show only Raptors games:

```bash
hockey-schedule --url "https://example.com/schedule.ics" --team Raptors
```

Export Raptors games to an iCal file for import into another calendar:

```bash
hockey-schedule --url "https://example.com/schedule.ics" --team Raptors --ics > raptors-games.ics
```

Export to CSV for spreadsheets:

```bash
hockey-schedule --url "https://example.com/schedule.ics" --team Raptors --csv > raptors-games.csv
```

Show only the next 5 games:

```bash
hockey-schedule --url "https://example.com/schedule.ics" --num 5
```

## Dependencies

- `reqwest` - HTTP client for fetching iCal feeds
- `ical` - iCal format parser
- `chrono` - Date/time handling
- `clap` - CLI argument parsing
- `anyhow` - Error handling
- `regex` - Parsing location/address fields
