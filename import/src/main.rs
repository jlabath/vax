use anyhow::{Context, Result};
use ontariopublic::{CasesByVacStatusRoot, DayReport, HospitalizationByVacStatusRoot, Index};
use serde::Serialize;
use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;

const FNAME: &str = "cases_by_vac_status.json";
const HFNAME: &str = "hosp_by_vac_status.json";
const OUTFNAME: &str = "bulk.json";

#[derive(Serialize, Debug)]
struct Entry {
    key: String,
    value: String,
}

fn main() -> Result<()> {
    let f = File::open(FNAME).with_context(|| format!("Failed to read from {}", FNAME))?;
    let br = BufReader::new(f);
    let cases_by_vac: CasesByVacStatusRoot = serde_json::from_reader(br)?;
    let f_hosp = File::open(HFNAME).with_context(|| format!("Failed to read from {}", HFNAME))?;
    let br_hosp = BufReader::new(f_hosp);
    let hosp_by_vac: HospitalizationByVacStatusRoot = serde_json::from_reader(br_hosp)?;
    let mut hosp_map = HashMap::new();
    let mut reports = vec![];
    let mut entries = vec![];
    let mut keys: Vec<String> = Vec::new();
    //put the hospitalizations in a map
    for r in hosp_by_vac {
        match r {
            Ok(h) => {
                hosp_map.insert(h.date, h);
            }
            Err(err) => {
                println!("Error {:?} {}", err, err);
            }
        }
    }
    //iterate cases to to create dayreports use the map to add data
    for r in cases_by_vac {
        match r {
            Ok(cases) => match hosp_map.remove(&cases.date) {
                Some(hosps) => {
                    let report = DayReport::from(cases, hosps);
                    report.validate()?;
                    reports.push(report);
                }
                None => {
                    println!("Error did not find hosptalization for {}", cases.date);
                }
            },
            Err(err) => {
                println!("Error {:?} {}", err, err);
            }
        }
    }
    for r in reports {
        let entry = Entry {
            key: r.key(),
            value: serde_json::to_string(&r)?,
        };
        keys.push(r.key());
        entries.push(entry);
    }
    //now add the index
    let index = Index::from(keys.as_slice());
    entries.push(Entry {
        key: "index".into(),
        value: serde_json::to_string(&index)?,
    });
    let fout = File::create(OUTFNAME)
        .with_context(|| format!("Failed to open {} for writing", OUTFNAME))?;
    let _ = serde_json::to_writer(fout, &entries)?;
    Ok(())
}
