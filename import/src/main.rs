use anyhow::{anyhow, Context, Result};
use clap::{Arg, Command};
use ontariopublic::{
    CasesByVacStatus, CasesByVacStatusRoot, CsvCase, CsvCasesRoot, DayReport,
    HospitalizationByVacStatusRoot, Index,
};
use rust_decimal::prelude::*;
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

const FNAME: &str = "cases_by_vac_status.json";
const CSV_FNAME: &str = "cases_by_vac_status.csv";
const HFNAME: &str = "hosp_by_vac_status.json";
const OUTFNAME: &str = "bulk.json";

#[derive(Serialize, Debug)]
struct Entry {
    key: String,
    value: String,
}

fn main() -> Result<()> {
    let matches = Command::new("VaxImport")
        .version("0.1")
        .author("Jakub Labath. <jakub@labath.ca>")
        .about("Serializes Ontario Health data into a format we can use")
        .arg(
            Arg::new("cases file")
                .long("cases")
                .help("What file to use as source of cases by vaccination")
                .takes_value(true)
                .possible_values([FNAME, CSV_FNAME])
                .default_value(FNAME),
        )
        .get_matches();

    let cases_by_vac = cases_by_vac_status(matches.value_of("cases file"))?;
    let f_hosp = File::open(HFNAME).with_context(|| format!("Failed to read from {}", HFNAME))?;
    let br_hosp = BufReader::new(f_hosp);
    let hosp_by_vac: HospitalizationByVacStatusRoot = serde_json::from_reader(br_hosp)
        .with_context(|| format!("Failed to deserialize {}", HFNAME))?;
    //validate the root of hospitalizations by status
    hosp_by_vac.validate()?;
    let mut hosp_map = HashMap::new();
    let mut reports = vec![];
    let mut entries = vec![];
    let mut keys: Vec<String> = Vec::new();
    //variables for charts
    let mut labels = vec![];
    let mut cases_dose0 = vec![];
    let mut cases_dose2 = vec![];
    let mut hosp_dose0 = vec![];
    let mut hosp_dose2 = vec![];
    let mut icu_dose0 = vec![];
    let mut icu_dose2 = vec![];
    //put the hospitalizations in a map
    for r in hosp_by_vac {
        match r {
            Ok(h) => {
                hosp_map.insert(h.date, h);
            }
            Err(err) => {
                println!("Error when iterating hosp_by_vac {:?} {}", err, err);
            }
        }
    }
    //iterate cases to to create dayreports use the map to add data
    for r in cases_by_vac {
        match r {
            Ok(cases) => {
                let check = cases.validate();
                if check.is_err() {
                    println!(
                        "Skipping processing invalid cases {:?} due to {:?}",
                        &cases, check
                    );
                    continue;
                }
                match hosp_map.remove(&cases.date) {
                    Some(hosps) => {
                        let report = DayReport::from(cases, hosps);
                        if let Err(e) = report.validate() {
                            println!("Skipping  invalid report {} due to {:?}", report.key(), e);
                            continue;
                        }
                        reports.push(report);
                    }
                    None => {
                        println!("Error did not find hospitalization for {}", cases.date);
                    }
                }
            }
            Err(err) => {
                println!("Error when iterating cases_by_vac {:?} {}", err, err);
            }
        }
    }
    //sort reports
    reports.sort_unstable_by_key(|k| k.key());
    for r in reports {
        let entry = Entry {
            key: r.key(),
            value: serde_json::to_string(&r)?,
        };
        keys.push(r.key());
        entries.push(entry);
        //charts
        labels.push(r.cases.date.format("%Y-%m-%d").to_string());
        cases_dose0.push(chart_float(r.cases.cases_unvac_rate_per100k));
        cases_dose2.push(chart_float(r.cases.cases_full_vac_rate_per100k));
        hosp_dose0.push(chart_float(r.nonicu_unvac_rate_per100k()));
        hosp_dose2.push(chart_float(r.nonicu_full_vac_rate_per100k()));
        icu_dose0.push(chart_float(r.icu_unvac_rate_per100k()));
        icu_dose2.push(chart_float(r.icu_full_vac_rate_per100k()));
    }
    //now add the index
    let index = Index::from(keys.as_slice());
    entries.push(Entry {
        key: "index".into(),
        value: serde_json::to_string(&index)?,
    });
    //now add the chart entries
    entries.push(Entry {
        key: "labels".into(),
        value: serde_json::to_string(&labels)?,
    });
    entries.push(Entry {
        key: "cases_dose0".into(),
        value: serde_json::to_string(&cases_dose0)?,
    });
    entries.push(Entry {
        key: "cases_dose2".into(),
        value: serde_json::to_string(&cases_dose2)?,
    });
    entries.push(Entry {
        key: "nonicu_dose0".into(),
        value: serde_json::to_string(&hosp_dose0)?,
    });
    entries.push(Entry {
        key: "nonicu_dose2".into(),
        value: serde_json::to_string(&hosp_dose2)?,
    });
    entries.push(Entry {
        key: "icu_dose0".into(),
        value: serde_json::to_string(&icu_dose0)?,
    });
    entries.push(Entry {
        key: "icu_dose2".into(),
        value: serde_json::to_string(&icu_dose2)?,
    });

    let fout = File::create(OUTFNAME)
        .with_context(|| format!("Failed to open {} for writing", OUTFNAME))?;
    let _ = serde_json::to_writer(fout, &entries)?;
    Ok(())
}

fn chart_float(n: Decimal) -> f64 {
    n.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
        .to_f64()
        .unwrap_or(0.0)
}

fn cases_by_vac_status(
    fname: Option<&str>,
) -> Result<Box<dyn Iterator<Item = ontariopublic::Result<CasesByVacStatus>>>> {
    match fname {
        Some(FNAME) => {
            let f = File::open(FNAME).with_context(|| format!("Failed to read from {}", FNAME))?;
            let br = BufReader::new(f);
            let cases_by_vac: CasesByVacStatusRoot = serde_json::from_reader(br)
                .with_context(|| format!("Failed to deserialize {}", FNAME))?;
            //validate the root object of cases
            cases_by_vac.validate()?;
            Ok(Box::new(cases_by_vac.into_iter()))
        }
        Some(CSV_FNAME) => {
            let mut cases = vec![];
            let fname = CSV_FNAME;
            let f = File::open(fname).with_context(|| format!("Failed to read from {}", fname))?;
            let br = BufReader::new(f);
            let mut reader = csv::Reader::from_reader(br);

            for record in reader.deserialize() {
                let record: CsvCase = record.context("Reading data into Case struct failed")?;
                //println!("It is a {:?}.", record);
                cases.push(record);
            }

            Ok(Box::new(CsvCasesRoot(cases).into_iter()))
        }
        _ => Err(anyhow!("Invalid value for cases param {:?}", fname)),
    }
}
