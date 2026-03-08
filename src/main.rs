use anyhow::{Context, Result};
use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use clap::Parser;
use ical::parser::ical::component::IcalEvent;
use ical::IcalParser;
use regex::Regex;
use std::io::BufReader;

/// CLI tool to fetch and display upcoming hockey games from an iCal feed
#[derive(Parser, Debug)]
#[command(name = "hockey-schedule")]
#[command(about = "Fetch and display upcoming hockey games from an iCal feed")]
struct Args {
    /// iCal feed URL
    #[arg(short, long, env = "HOCKEY_ICAL_URL", default_value = "https://example.com/hockey-schedule.ics")]
    url: String,

    /// Number of upcoming games to show
    #[arg(short, long, default_value_t = 10)]
    num: usize,

    /// Filter to only games involving this team (case-insensitive)
    #[arg(short, long)]
    team: Option<String>,

    /// Output as iCal format instead of human-readable text
    #[arg(long)]
    ics: bool,

    /// Output as CSV format
    #[arg(long)]
    csv: bool,
}

#[derive(Debug)]
struct Game {
    summary: String,
    start: DateTime<Local>,
    end: Option<DateTime<Local>>,
    location: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let ical_content = fetch_ical_feed(&args.url)?;
    let games = parse_ical_events(&ical_content)?;
    let upcoming_games = filter_upcoming_games(games, args.num, args.team.as_deref());

    if upcoming_games.is_empty() {
        println!("No upcoming games found.");
    } else if args.ics {
        print!("{}", output_ical(&upcoming_games));
    } else if args.csv {
        print!("{}", output_csv(&upcoming_games));
    } else {
        display_games(&upcoming_games);
    }

    Ok(())
}

fn fetch_ical_feed(url: &str) -> Result<String> {
    let client = reqwest::blocking::Client::new();
    let response = client
        .get(url)
        .send()
        .with_context(|| format!("Failed to fetch iCal feed from {}", url))?;

    if !response.status().is_success() {
        anyhow::bail!(
            "HTTP request failed with status: {}",
            response.status()
        );
    }

    response
        .text()
        .with_context(|| "Failed to read response body")
}

fn parse_ical_events(ical_content: &str) -> Result<Vec<Game>> {
    let buf_reader = BufReader::new(ical_content.as_bytes());
    let parser = IcalParser::new(buf_reader);

    let mut games = Vec::new();

    for calendar_result in parser {
        let calendar = calendar_result.map_err(|e| anyhow::anyhow!("Failed to parse iCal: {}", e))?;

        for event in calendar.events {
            if let Some(game) = parse_event(&event) {
                games.push(game);
            }
        }
    }

    Ok(games)
}

fn parse_event(event: &IcalEvent) -> Option<Game> {
    let mut summary = None;
    let mut dtstart = None;
    let mut dtend = None;
    let mut location = None;

    for property in &event.properties {
        match property.name.as_str() {
            "SUMMARY" => summary = property.value.clone(),
            "DTSTART" => dtstart = property.value.clone(),
            "DTEND" => dtend = property.value.clone(),
            "LOCATION" => location = property.value.clone(),
            _ => {}
        }
    }

    let summary = summary?;
    let start_str = dtstart?;
    let start = parse_ical_datetime(&start_str)?;

    let end = dtend.and_then(|s| parse_ical_datetime(&s));

    Some(Game {
        summary,
        start,
        end,
        location,
    })
}

fn parse_ical_datetime(dt_str: &str) -> Option<DateTime<Local>> {
    // Handle UTC format: 20260228T190000Z
    if dt_str.ends_with('Z') {
        let naive = NaiveDateTime::parse_from_str(
            dt_str.trim_end_matches('Z'),
            "%Y%m%dT%H%M%S",
        )
        .ok()?;
        let utc_dt = Utc.from_utc_datetime(&naive);
        return Some(utc_dt.with_timezone(&Local));
    }

    // Handle local/floating format: 20260228T190000
    if let Ok(naive) = NaiveDateTime::parse_from_str(dt_str, "%Y%m%dT%H%M%S") {
        return Local.from_local_datetime(&naive).single();
    }

    // Handle date-only format: 20260228
    if let Ok(naive_date) = chrono::NaiveDate::parse_from_str(dt_str, "%Y%m%d") {
        let naive_dt = naive_date.and_hms_opt(0, 0, 0)?;
        return Local.from_local_datetime(&naive_dt).single();
    }

    None
}

fn filter_upcoming_games(mut games: Vec<Game>, limit: usize, team: Option<&str>) -> Vec<Game> {
    let now = Local::now();

    // Filter to upcoming games only
    games.retain(|game| game.start >= now);

    // Filter by team if specified (case-insensitive)
    if let Some(team_name) = team {
        let team_lower = team_name.to_lowercase();
        games.retain(|game| game.summary.to_lowercase().contains(&team_lower));
    }

    // Sort by start time
    games.sort_by(|a, b| a.start.cmp(&b.start));

    // Take first N games
    games.truncate(limit);

    games
}

fn display_games(games: &[Game]) {
    println!("Upcoming Hockey Games");
    println!("{}", "=".repeat(60));
    println!();

    for (i, game) in games.iter().enumerate() {
        println!("{}. {}", i + 1, game.summary);
        println!("   When: {}", game.start.format("%a %b %d, %Y %I:%M %p"));

        if let Some(ref end) = game.end {
            println!("   Until: {}", end.format("%I:%M %p"));
        }

        if let Some(ref location) = game.location {
            println!("   Where: {}", location);
        }

        println!();
    }
}

fn output_ical(games: &[Game]) -> String {
    let mut output = String::new();

    output.push_str("BEGIN:VCALENDAR\r\n");
    output.push_str("VERSION:2.0\r\n");
    output.push_str("PRODID:-//Hockey Schedule//EN\r\n");

    for (i, game) in games.iter().enumerate() {
        output.push_str("BEGIN:VEVENT\r\n");

        // Generate a UID based on start time and index
        let uid = format!(
            "{}-{}@hockey-schedule",
            game.start.format("%Y%m%dT%H%M%S"),
            i
        );
        output.push_str(&format!("UID:{}\r\n", uid));

        // DTSTART in UTC
        let start_utc = game.start.with_timezone(&Utc);
        output.push_str(&format!("DTSTART:{}\r\n", start_utc.format("%Y%m%dT%H%M%SZ")));

        // DTEND in UTC
        if let Some(ref end) = game.end {
            let end_utc = end.with_timezone(&Utc);
            output.push_str(&format!("DTEND:{}\r\n", end_utc.format("%Y%m%dT%H%M%SZ")));
        }

        // SUMMARY
        output.push_str(&format!("SUMMARY:{}\r\n", escape_ical_text(&game.summary)));

        // LOCATION
        if let Some(ref location) = game.location {
            output.push_str(&format!("LOCATION:{}\r\n", escape_ical_text(location)));
        }

        // DTSTAMP (required)
        let now_utc = Utc::now();
        output.push_str(&format!("DTSTAMP:{}\r\n", now_utc.format("%Y%m%dT%H%M%SZ")));

        output.push_str("END:VEVENT\r\n");
    }

    output.push_str("END:VCALENDAR\r\n");
    output
}

fn escape_ical_text(text: &str) -> String {
    text.replace('\\', "\\\\")
        .replace(';', "\\;")
        .replace(',', "\\,")
        .replace('\n', "\\n")
}

fn output_csv(games: &[Game]) -> String {
    let mut output = String::new();

    // Header row matching BenchApp import template
    output.push_str("Type,Game Type,Title (Optional),Away,Home,Date,Time,Duration,Location (Optional),Address (Optional),Notes (Optional)\n");

    for game in games {
        // Parse "Home vs Away" from summary
        let (home, away) = parse_teams(&game.summary);

        // Date as DD/MM/YYYY
        let date = game.start.format("%d/%m/%Y").to_string();

        // Time as "9:00 PM"
        let time = game.start.format("%-I:%M %p").to_string();

        // Calculate duration from start/end
        let duration = if let Some(end) = game.end {
            let dur = end.signed_duration_since(game.start);
            let hours = dur.num_hours();
            let minutes = dur.num_minutes() % 60;
            format!("{}:{:02}", hours, minutes)
        } else {
            "1:00".to_string() // Default 1 hour
        };

        let (venue, address) = game
            .location
            .as_ref()
            .map(|l| split_location(l))
            .unwrap_or_default();

        output.push_str(&format!(
            "GAME,REGULAR,,{},{},{},{},{},{},{},\n",
            escape_csv_field(&away),
            escape_csv_field(&home),
            date,
            time,
            duration,
            escape_csv_field(&venue),
            escape_csv_field(&address)
        ));
    }

    output
}

fn parse_teams(summary: &str) -> (String, String) {
    // Parse "Home vs Away" format
    if let Some(pos) = summary.to_lowercase().find(" vs ") {
        let home = summary[..pos].trim().to_string();
        let away = summary[pos + 4..].trim().to_string();
        (home, away)
    } else {
        (summary.to_string(), String::new())
    }
}

fn escape_csv_field(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

fn split_location(location: &str) -> (String, String) {
    // Regex to find where a street address begins
    // Looks for: number + optional direction + word + street type
    let re = Regex::new(
        r"(?i)\d+\s+([NSEW]\.?\s+)?\w+\s+(St|Street|Ave|Avenue|Blvd|Boulevard|Dr|Drive|Rd|Road|Way|Ln|Lane|Ct|Court|Pl|Place|Cir|Circle)\b"
    ).unwrap();

    if let Some(m) = re.find(location) {
        let venue = location[..m.start()].trim().to_string();
        let address = location[m.start()..].trim().to_string();
        (venue, address)
    } else {
        // No address pattern found, put everything in venue
        (location.to_string(), String::new())
    }
}
