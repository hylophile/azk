use std::error::Error;
use std::fs::{self, File};
use std::path::PathBuf;

use clap::{arg, Command};

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Record {
    kind: String,
    time: String,
    // country: String,
    // population: Option<u64>,
}

struct DayInfo {
    start: isize,
    duration: isize,
}

const SECONDS_PER_MINUTE: isize = 60;
const SECONDS_PER_HOUR: isize = 60 * 60;

fn hhmmss_to_s(hhmmss: &str) -> isize {
    let mut iter = hhmmss
        .splitn(3, ':')
        .map(|n| n.parse::<isize>().expect("couldn't parse"));

    let h = iter.next().unwrap();
    let m = iter.next().unwrap();
    let s = iter.next().unwrap();

    (h * SECONDS_PER_HOUR) + (m * SECONDS_PER_MINUTE) + s
}

fn s_to_hhmm(s: isize) -> String {
    let hours = s / SECONDS_PER_HOUR;
    let minutes = (s % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE;

    format!("{:02}:{:02}", hours, minutes)
}

fn _hhmmss_distance(from: &str, to: &str) -> String {
    let from = hhmmss_to_s(from);
    let to = hhmmss_to_s(to);
    let result = (to - from).abs();
    s_to_hhmm(result)
}

fn write_record(file: &File, record: Record) -> Result<(), Box<dyn Error>> {
    let write_headers = file.metadata()?.len() == 0;
    let mut wtr = csv::WriterBuilder::new()
        .has_headers(write_headers)
        .from_writer(file);
    wtr.serialize(record)?;
    wtr.flush()?;
    Ok(())
}

fn read_work_time(file: &File) -> Result<DayInfo, Box<dyn Error>> {
    let mut rdr = csv::Reader::from_reader(file);
    // let mut start: isize = 0;
    // let mut it = rdr.deserialize::<Record>();
    // let records: Result<Vec<Record>, csv::Error> = rdr.deserialize::<Record>().collect();
    let records = rdr.deserialize::<Record>();

    let (starts, stops): (Vec<Record>, Vec<Record>) = records
        .map(|x| match x {
            Ok(record) => record,
            Err(err) => panic!("{err}"),
        })
        .partition(|x| x.kind == "strt");

    let adding: isize = starts.iter().map(|x| hhmmss_to_s(&x.time)).sum();
    let subtracting: isize = stops.iter().map(|x| hhmmss_to_s(&x.time)).sum();
    let start = starts.get(0).map_or(0, |x| hhmmss_to_s(&x.time));

    Ok(DayInfo {
        start,
        duration: adding - subtracting,
    })

    // let len = it.clone().count();

    // if let Ok(records) = records {
    //     for (i, record) in records.iter().enumerate() {
    //         if i == 0 {
    //             start = hhmmss_to_s(&record.time);
    //         }

    //         match record.kind.as_str() {
    //             "strt" => duration -= hhmmss_to_s(&record.time),
    //             "stop" => duration += hhmmss_to_s(&record.time),
    //             _ => return Err("meh")?,
    //         }

    //         if i == records.len() - 1 {
    //             return Ok((duration, record.kind.clone(), start));
    //         }
    //     }
    // }
    // let start: isize = if let Some(Ok(first)) = it.next() {
    //     match first.kind.as_str() {
    //         "strt" => duration -= hhmmss_to_s(&first.time),
    //         "stop" => duration += hhmmss_to_s(&first.time),
    //         _ => return Err("meh")?,
    //     }
    //     hhmmss_to_s(&first.time)
    // } else {
    //     0
    // };

    // while let Some(Ok(record)) = it.next() {
    //     match record.kind.as_str() {
    //         "strt" => duration -= hhmmss_to_s(&record.time),
    //         "stop" => duration += hhmmss_to_s(&record.time),
    //         _ => return Err("meh")?,
    //     }
    //     if it.peek().is_none() {
    //         return Ok((duration, record.kind, start));
    //     }
    // }

    // Ok((0, "stop".to_owned(), 0))
}

fn update_time(file: &File, time: &str) -> Result<(), Box<dyn Error>> {
    let DayInfo { duration, start: _ } = read_work_time(file)?;

    let new_kind = if duration < 0 { "stop" } else { "strt" };

    let record = Record {
        time: time.to_owned(),
        kind: new_kind.to_owned(),
    };

    write_record(file, record)?;
    Ok(())
}

fn cli(file_path: &str) -> Command {
    Command::new("azk")
        .about("A work time tracker")
        .subcommand_required(true)
        .arg_required_else_help(true)
        .subcommand(Command::new("stamp").about(format!(
            "Record a timestamp in {file_path} and toggle between work and break",
        )))
        .subcommand(
            Command::new("get")
                .about("Get the work duration for the current day or [DAY]")
                .arg(arg!(day: [DAY] "The day to get the work duration for, in YYYY-MM-DD")),
        )
}

fn file_path(date: &str) -> Result<PathBuf, Box<dyn Error>> {
    if let Some(proj_dirs) = directories::ProjectDirs::from("com", "hylo", "azk") {
        let mut file_path = proj_dirs.data_dir().to_path_buf();
        fs::create_dir_all(&file_path)?;
        file_path.push(date);
        file_path.set_extension("csv");
        return Ok(file_path);
    }
    Err("path error")?
}

fn main() -> Result<(), Box<dyn Error>> {
    let now = chrono::Local::now();
    let date: String = format!("{}", now.format("%Y-%m-%d"));
    let time: String = format!("{}", now.format("%H:%M:%S"));

    let file_path = file_path(&date)?;

    match cli(&file_path.to_str().unwrap()).get_matches().subcommand() {
        Some(("stamp", _)) => {
            let file = File::options()
                .write(true)
                .read(true)
                .create(true)
                .append(true)
                .open(&file_path)?;
            update_time(&file, &time)?;
            println!(
                "Updated {file_path} with {time}.",
                file_path = file_path.display()
            )
        }
        Some(("get", sub_matches)) => {
            let day = sub_matches.get_one::<String>("day").unwrap_or(&date);
            let DayInfo { start, duration } = read_work_time(&File::open(file_path)?)?;

            if duration < 0 {
                println!("Work ain't over yet.")
            } else {
                let duration_hhmm = s_to_hhmm(duration);
                let from_hhmm = s_to_hhmm(start);
                let to_hhmm = s_to_hhmm(start + duration);
                dbg!(start, duration);
                println!("Worked for {duration_hhmm} on {day}.\nFrom {from_hhmm} to {to_hhmm}")
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
