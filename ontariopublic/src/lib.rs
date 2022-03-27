use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use thiserror::Error;

const HUNDRED_K: Decimal = Decimal::from_parts(100000, 0, 0, false, 0);

#[derive(Error, Debug)]
pub enum DataError {
    #[error("unable to parse date `{0}`")]
    Date(#[from] chrono::ParseError),
    #[error("the value `{0}` is invalid")]
    Invalid(String),
    #[error("unable to parse int from `{0}`")]
    Int(#[from] std::num::ParseIntError),
    #[error("unable to parse Decimal from `{0}`")]
    Decimal(#[from] rust_decimal::Error),
    #[error("{0}")]
    Problem(String),
}

//just some alias for smaller function signatures
pub type Result<T> = std::result::Result<T, DataError>;

#[derive(Deserialize, Debug)]
pub struct CasesByVacStatusRoot {
    pub fields: Vec<HeaderField>,
    pub records: Vec<Vec<serde_json::Value>>,
}

impl CasesByVacStatusRoot {
    pub fn validate(&self) -> Result<()> {
        let expected = self.expected_header();
        if self.fields.len() != expected.len() {
            let msg = format!("the expected headers and received headers have different length expected: {} actual: {}", expected.len(), self.fields.len());
            return Err(DataError::Problem(msg));
        }
        //ok if the same length zip together and compare
        let zipped = self.fields.iter().zip(expected.iter());
        for (a, b) in zipped {
            if *a != *b {
                let msg = format!(
                    "The two header fields do not match left: {:?} right: {:?}",
                    a, b
                );
                return Err(DataError::Problem(msg));
            }
        }
        Ok(())
    }

    fn expected_header(&self) -> Vec<HeaderField> {
        vec![
            HeaderField::new("_id", "int", Default::default()),
            HeaderField::new(
                "Date",
                "timestamp",
                HeaderFieldInfo::new("", "timestamp", ""),
            ),
            HeaderField::new("covid19_cases_unvac", "text", Default::default()),
            HeaderField::new("covid19_cases_partial_vac", "text", Default::default()),
            HeaderField::new("covid19_cases_full_vac", "text", Default::default()),
            HeaderField::new("covid19_cases_vac_unknown", "text", Default::default()),
            HeaderField::new("cases_unvac_rate_per100K", "text", Default::default()),
            HeaderField::new("cases_partial_vac_rate_per100K", "text", Default::default()),
            HeaderField::new("cases_full_vac_rate_per100K", "text", Default::default()),
            HeaderField::new("cases_unvac_rate_7ma", "text", Default::default()),
            HeaderField::new("cases_partial_vac_rate_7ma", "text", Default::default()),
            HeaderField::new("cases_full_vac_rate_7ma", "text", Default::default()),
        ]
    }
}

impl IntoIterator for CasesByVacStatusRoot {
    type Item = Result<CasesByVacStatus>;
    type IntoIter = CasesByVacStatusRootIterator;

    fn into_iter(self) -> Self::IntoIter {
        CasesByVacStatusRootIterator {
            root: self,
            index: 0,
        }
    }
}

pub struct CasesByVacStatusRootIterator {
    root: CasesByVacStatusRoot,
    index: usize,
}

impl Iterator for CasesByVacStatusRootIterator {
    type Item = Result<CasesByVacStatus>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.root.records.get(self.index) {
            Some(rec) => {
                self.index += 1;
                Some(transform_record(rec))
            }
            None => None,
        }
    }
}

#[derive(Deserialize, Debug, Default, PartialEq)]
pub struct HeaderField {
    id: String,
    #[serde(rename = "type")]
    field_type: String,
    #[serde(default)]
    info: HeaderFieldInfo,
}

impl HeaderField {
    fn new(id: &str, field_type: &str, inf: HeaderFieldInfo) -> Self {
        HeaderField {
            id: id.to_string(),
            field_type: field_type.to_string(),
            info: inf,
        }
    }
}

#[derive(Deserialize, Debug, Default, PartialEq)]
pub struct HeaderFieldInfo {
    notes: String,
    type_override: String,
    label: String,
}

impl HeaderFieldInfo {
    fn new(notes: &str, type_override: &str, label: &str) -> Self {
        HeaderFieldInfo {
            notes: notes.to_string(),
            type_override: type_override.to_string(),
            label: label.to_string(),
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct CasesByVacStatus {
    pub id: i64,
    pub date: NaiveDate,
    pub covid19_cases_unvac: Option<i64>,
    pub covid19_cases_partial_vac: i64,
    pub covid19_cases_full_vac: i64,
    pub covid19_cases_vac_unknown: i64,
    pub cases_unvac_rate_per100k: Decimal,
    pub cases_partial_vac_rate_per100k: Decimal,
    pub cases_full_vac_rate_per100k: Decimal,
    pub cases_unvac_rate_7ma: Decimal,
    pub cases_partial_vac_rate_7ma: Decimal,
    pub cases_full_vac_rate_7ma: Decimal,
}

impl Default for CasesByVacStatus {
    fn default() -> Self {
        CasesByVacStatus {
            id: Default::default(),
            date: NaiveDate::from_ymd(2019, 12, 8),
            covid19_cases_unvac: Default::default(),
            covid19_cases_partial_vac: Default::default(),
            covid19_cases_full_vac: Default::default(),
            covid19_cases_vac_unknown: Default::default(),
            cases_unvac_rate_per100k: Default::default(),
            cases_partial_vac_rate_per100k: Default::default(),
            cases_full_vac_rate_per100k: Default::default(),
            cases_unvac_rate_7ma: Default::default(),
            cases_partial_vac_rate_7ma: Default::default(),
            cases_full_vac_rate_7ma: Default::default(),
        }
    }
}

impl CasesByVacStatus {
    //checks the struct for sanity
    pub fn validate(&self) -> Result<()> {
        if self.date < NaiveDate::from_ymd(2020, 7, 1) {
            return Err(DataError::Invalid(self.date.format("%Y-%m-%d").to_string()));
        };
        if self.covid19_cases_unvac.unwrap_or(0) < 0 {
            return Err(DataError::Invalid(
                self.covid19_cases_unvac
                    .map_or("".to_string(), |x| x.to_string()),
            ));
        };
        if self.covid19_cases_partial_vac < 0 {
            return Err(DataError::Invalid(
                self.covid19_cases_partial_vac.to_string(),
            ));
        };
        if self.covid19_cases_full_vac < 0 {
            return Err(DataError::Invalid(self.covid19_cases_full_vac.to_string()));
        };
        if self.covid19_cases_vac_unknown < 0 {
            return Err(DataError::Invalid(
                self.covid19_cases_vac_unknown.to_string(),
            ));
        };
        let zero = Decimal::new(0, 0);
        if self.cases_unvac_rate_per100k < zero && self.cases_unvac_rate_per100k > HUNDRED_K {
            return Err(DataError::Invalid(
                self.cases_unvac_rate_per100k.to_string(),
            ));
        };
        if self.cases_partial_vac_rate_per100k < zero
            && self.cases_partial_vac_rate_per100k > HUNDRED_K
        {
            return Err(DataError::Invalid(
                self.cases_partial_vac_rate_per100k.to_string(),
            ));
        };
        if self.cases_full_vac_rate_per100k < zero && self.cases_full_vac_rate_per100k > HUNDRED_K {
            return Err(DataError::Invalid(
                self.cases_full_vac_rate_per100k.to_string(),
            ));
        };
        if self.cases_unvac_rate_7ma < zero && self.cases_unvac_rate_7ma > HUNDRED_K {
            return Err(DataError::Invalid(self.cases_unvac_rate_7ma.to_string()));
        };
        if self.cases_partial_vac_rate_7ma < zero && self.cases_partial_vac_rate_7ma > HUNDRED_K {
            return Err(DataError::Invalid(
                self.cases_partial_vac_rate_7ma.to_string(),
            ));
        };
        if self.cases_full_vac_rate_7ma < zero && self.cases_full_vac_rate_7ma > HUNDRED_K {
            return Err(DataError::Invalid(self.cases_full_vac_rate_7ma.to_string()));
        };
        Ok(())
    }

    pub fn calc_unvac_population(&self) -> Option<Decimal> {
        match self.covid19_cases_unvac {
            Some(cases) => Some(compute_total_population_from_cases_and_rate(
                cases,
                self.cases_unvac_rate_per100k,
            )),
            _ => None,
        }
    }

    pub fn calc_full_vac_population(&self) -> Decimal {
        compute_total_population_from_cases_and_rate(
            self.covid19_cases_full_vac,
            self.cases_full_vac_rate_per100k,
        )
    }

    pub fn calc_partial_vac_population(&self) -> Decimal {
        compute_total_population_from_cases_and_rate(
            self.covid19_cases_partial_vac,
            self.cases_partial_vac_rate_per100k,
        )
    }
}

//transfrorms a record from ontario source json into our usable type
fn transform_record(record: &[serde_json::Value]) -> Result<CasesByVacStatus> {
    let mut v: CasesByVacStatus = Default::default();
    //get id
    if let Some(idv) = record.get(0) {
        if let Some(id) = idv.as_i64() {
            v.id = id;
        }
    };
    //get date
    if let Some(datev) = record.get(1) {
        if let Some(date) = datev.as_str() {
            let dt = NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S")?;
            v.date = dt.date();
        }
    };
    //get cases unvac
    if let Some(rv) = record.get(2) {
        if let Some(snum) = rv.as_str() {
            v.covid19_cases_unvac = snum.parse::<i64>().ok();
        }
    };
    //get cases partial vac
    if let Some(rv) = record.get(3) {
        if let Some(snum) = rv.as_str() {
            v.covid19_cases_partial_vac = snum.parse::<i64>()?;
        }
    };
    //get cases full vac
    if let Some(rv) = record.get(4) {
        if let Some(snum) = rv.as_str() {
            v.covid19_cases_full_vac = snum.parse::<i64>()?;
        }
    };
    //get cases vac unknown
    if let Some(rv) = record.get(5) {
        if let Some(snum) = rv.as_str() {
            v.covid19_cases_vac_unknown = snum.parse::<i64>()?;
        }
    };
    //get unvac rate
    if let Some(rv) = record.get(6) {
        if let Some(snum) = rv.as_str() {
            v.cases_unvac_rate_per100k = Decimal::from_str(snum)?;
        }
    };
    //get partial rate
    if let Some(rv) = record.get(7) {
        if let Some(snum) = rv.as_str() {
            v.cases_partial_vac_rate_per100k = Decimal::from_str(snum)?;
        }
    };
    //get full rate
    if let Some(rv) = record.get(8) {
        if let Some(snum) = rv.as_str() {
            v.cases_full_vac_rate_per100k = Decimal::from_str(snum)?;
        }
    };
    //get unvac rate 7ma
    if let Some(rv) = record.get(9) {
        if let Some(snum) = rv.as_str() {
            v.cases_unvac_rate_7ma = Decimal::from_str(snum)?;
        }
    };
    //get partial rate 7ma
    if let Some(rv) = record.get(10) {
        if let Some(snum) = rv.as_str() {
            v.cases_partial_vac_rate_7ma = Decimal::from_str(snum)?;
        }
    };
    //get full rate 7ma
    if let Some(rv) = record.get(11) {
        if let Some(snum) = rv.as_str() {
            v.cases_full_vac_rate_7ma = Decimal::from_str(snum)?;
        }
    };

    let _ = v.validate()?;
    Ok(v)
}

fn compute_total_population_from_cases_and_rate(cases: i64, rate: Decimal) -> Decimal {
    //rate is assumed at 100k
    //x * (rate / 100 000) = cases
    //x = cases / (rate / 100 000)
    //x = (cases * 100 000) / rate
    if rate.is_zero() {
        return Decimal::new(0, 0);
    }
    let case_count = Decimal::new(cases, 0);
    (case_count * HUNDRED_K) / rate
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct DayReport {
    pub cases: CasesByVacStatus,
    pub hosps: HospitalizationByVacStatus,
}

impl DayReport {
    //checks the structs for sanity
    pub fn validate(&self) -> Result<()> {
        self.cases.validate()?;
        self.hosps.validate()?;
        if self.cases.date != self.hosps.date {
            return Err(DataError::Invalid(
                "cases and hospitalization dates do not match".into(),
            ));
        }
        //test calculations
        if let Some(unvac_population) = self.cases.calc_unvac_population() {
            let num = (unvac_population * (self.cases.cases_unvac_rate_per100k / HUNDRED_K))
                .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
            if num != Decimal::new(self.cases.covid19_cases_unvac.unwrap_or(0), 0) {
                let msg = format!(
                    "The unvac cases for {} did not match calculated: {} expected: {}",
                    self.key(),
                    num,
                    self.cases.covid19_cases_unvac.unwrap_or(0),
                );
                return Err(DataError::Problem(msg));
            }
        }
        let num = (self.cases.calc_full_vac_population()
            * (self.cases.cases_full_vac_rate_per100k / HUNDRED_K))
            .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero);
        if num != Decimal::new(self.cases.covid19_cases_full_vac, 0) {
            let msg = format!(
                "The full vac cases did not match calculated: {} expected: {}",
                num, self.cases.covid19_cases_full_vac
            );
            return Err(DataError::Problem(msg));
        }

        Ok(())
    }

    pub fn key(&self) -> String {
        self.cases.date.format("%Y%m%d").to_string()
    }

    pub fn from(cases: CasesByVacStatus, hosps: HospitalizationByVacStatus) -> Self {
        DayReport { cases, hosps }
    }

    pub fn icu_unvac_rate_per100k(&self) -> Option<Decimal> {
        let calc_pop = self.cases.calc_unvac_population();
        calc_pop.map(|pop| {
            if pop.is_zero() {
                return Decimal::new(0, 0);
            }
            (Decimal::new(self.hosps.icu_unvac, 0) * HUNDRED_K) / pop
        })
    }

    pub fn icu_full_vac_rate_per100k(&self) -> Decimal {
        let pop = self.cases.calc_full_vac_population();
        if pop.is_zero() {
            return Decimal::new(0, 0);
        }
        (Decimal::new(self.hosps.icu_full_vac, 0) * HUNDRED_K) / pop
    }

    pub fn icu_partial_vac_rate_per100k(&self) -> Decimal {
        let pop = self.cases.calc_partial_vac_population();
        if pop.is_zero() {
            return Decimal::new(0, 0);
        }
        (Decimal::new(self.hosps.icu_partial_vac, 0) * HUNDRED_K) / pop
    }

    pub fn nonicu_unvac_rate_per100k(&self) -> Option<Decimal> {
        self.cases.calc_unvac_population().map(|pop| {
            if pop.is_zero() {
                return Decimal::new(0, 0);
            }
            (Decimal::new(self.hosps.hospitalnonicu_unvac, 0) * HUNDRED_K) / pop
        })
    }

    pub fn nonicu_full_vac_rate_per100k(&self) -> Decimal {
        let pop = self.cases.calc_full_vac_population();
        if pop.is_zero() {
            return Decimal::new(0, 0);
        }
        (Decimal::new(self.hosps.hospitalnonicu_full_vac, 0) * HUNDRED_K) / pop
    }

    pub fn nonicu_partial_vac_rate_per100k(&self) -> Decimal {
        let pop = self.cases.calc_partial_vac_population();
        if pop.is_zero() {
            return Decimal::new(0, 0);
        }
        (Decimal::new(self.hosps.hospitalnonicu_partial_vac, 0) * HUNDRED_K) / pop
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Index {
    keys: Vec<String>,
    pub updated: DateTime<Utc>,
}

impl Index {
    pub fn from<T: AsRef<str>>(keys: &[T]) -> Self {
        let mut v: Vec<String> = Vec::new();
        for s in keys {
            v.push(String::from(s.as_ref()));
        }
        v.sort_unstable();
        Index {
            keys: v,
            updated: Utc::now(),
        }
    }

    pub fn most_recent(&self) -> String {
        match self.keys.len() {
            0 => String::from("error empty index"),
            n => String::from(&self.keys[n - 1]),
        }
    }

    pub fn next(&self, key: String) -> Option<String> {
        match self.keys.binary_search(&key) {
            Ok(idx) => self.keys.get(idx + 1).map(|v| v.to_string()),
            Err(_) => None,
        }
    }

    pub fn prev(&self, key: String) -> Option<String> {
        match self.keys.binary_search(&key) {
            Ok(0) => None,
            Ok(idx) => self.keys.get(idx - 1).map(|v| v.to_string()),
            Err(_) => None,
        }
    }

    pub fn max_idx(&self) -> usize {
        self.keys.len() - 1
    }

    pub fn get(&self, idx: usize) -> Option<String> {
        self.keys.get(idx).map(|v| v.to_string())
    }

    pub fn idx(&self, key: String) -> Option<usize> {
        match self.keys.binary_search(&key) {
            Ok(idx) => Some(idx),
            Err(_) => None,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct HospitalizationByVacStatusRoot {
    pub fields: Vec<HeaderField>,
    pub records: Vec<Vec<serde_json::Value>>,
}

impl HospitalizationByVacStatusRoot {
    pub fn validate(&self) -> Result<()> {
        let expected = self.expected_header();
        if self.fields.len() != expected.len() {
            let msg = format!("the expected headers and received headers have different length expected: {} actual: {}", expected.len(), self.fields.len());
            return Err(DataError::Problem(msg));
        }
        //ok if the same length zip together and compare
        let zipped = self.fields.iter().zip(expected.iter());
        for (a, b) in zipped {
            if *a != *b {
                let msg = format!(
                    "The two header fields do not match left: {:?} right: {:?}",
                    a, b
                );
                return Err(DataError::Problem(msg));
            }
        }
        Ok(())
    }

    fn expected_header(&self) -> Vec<HeaderField> {
        vec![
            HeaderField::new("_id", "int", Default::default()),
            HeaderField::new(
                "date",
                "timestamp",
                HeaderFieldInfo::new("", "timestamp", ""),
            ),
            HeaderField::new(
                "icu_unvac",
                "numeric",
                HeaderFieldInfo::new("", "numeric", ""),
            ),
            HeaderField::new(
                "icu_partial_vac",
                "numeric",
                HeaderFieldInfo::new("", "numeric", ""),
            ),
            HeaderField::new(
                "icu_full_vac",
                "numeric",
                HeaderFieldInfo::new("", "numeric", ""),
            ),
            HeaderField::new(
                "hospitalnonicu_unvac",
                "numeric",
                HeaderFieldInfo::new("", "numeric", ""),
            ),
            HeaderField::new(
                "hospitalnonicu_partial_vac",
                "numeric",
                HeaderFieldInfo::new("", "numeric", ""),
            ),
            HeaderField::new(
                "hospitalnonicu_full_vac",
                "numeric",
                HeaderFieldInfo::new("", "numeric", ""),
            ),
        ]
    }
}

impl IntoIterator for HospitalizationByVacStatusRoot {
    type Item = Result<HospitalizationByVacStatus>;
    type IntoIter = HospitalizationByVacStatusRootIterator;

    fn into_iter(self) -> Self::IntoIter {
        HospitalizationByVacStatusRootIterator {
            root: self,
            index: 0,
        }
    }
}

pub struct HospitalizationByVacStatusRootIterator {
    root: HospitalizationByVacStatusRoot,
    index: usize,
}

impl Iterator for HospitalizationByVacStatusRootIterator {
    type Item = Result<HospitalizationByVacStatus>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.root.records.get(self.index) {
            Some(rec) => {
                self.index += 1;
                Some(transform_hosp_record(rec))
            }
            None => None,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub struct HospitalizationByVacStatus {
    id: i64,
    pub date: NaiveDate,
    pub icu_unvac: i64,
    pub icu_partial_vac: i64,
    pub icu_full_vac: i64,
    pub hospitalnonicu_unvac: i64,
    pub hospitalnonicu_partial_vac: i64,
    pub hospitalnonicu_full_vac: i64,
}

impl Default for HospitalizationByVacStatus {
    fn default() -> Self {
        HospitalizationByVacStatus {
            id: Default::default(),
            date: NaiveDate::from_ymd(2019, 12, 8),
            icu_unvac: Default::default(),
            icu_partial_vac: Default::default(),
            icu_full_vac: Default::default(),
            hospitalnonicu_unvac: Default::default(),
            hospitalnonicu_partial_vac: Default::default(),
            hospitalnonicu_full_vac: Default::default(),
        }
    }
}

impl HospitalizationByVacStatus {
    //checks the struct for sanity
    pub fn validate(&self) -> Result<()> {
        if self.id < 1 {
            return Err(DataError::Invalid(self.id.to_string()));
        };
        if self.date < NaiveDate::from_ymd(2020, 7, 1) {
            return Err(DataError::Invalid(self.date.format("%Y-%m-%d").to_string()));
        };

        if self.icu_unvac < 0 {
            return Err(DataError::Invalid(self.icu_unvac.to_string()));
        };
        if self.icu_partial_vac < 0 {
            return Err(DataError::Invalid(self.icu_partial_vac.to_string()));
        };
        if self.icu_full_vac < 0 {
            return Err(DataError::Invalid(self.icu_full_vac.to_string()));
        };
        if self.hospitalnonicu_unvac < 0 {
            return Err(DataError::Invalid(self.hospitalnonicu_unvac.to_string()));
        };
        if self.hospitalnonicu_partial_vac < 0 {
            return Err(DataError::Invalid(
                self.hospitalnonicu_partial_vac.to_string(),
            ));
        };
        if self.hospitalnonicu_full_vac < 0 {
            return Err(DataError::Invalid(self.hospitalnonicu_full_vac.to_string()));
        };

        Ok(())
    }
}

fn transform_hosp_record(record: &[serde_json::Value]) -> Result<HospitalizationByVacStatus> {
    let mut v: HospitalizationByVacStatus = Default::default();
    //get id
    if let Some(idv) = record.get(0) {
        if let Some(id) = idv.as_i64() {
            v.id = id;
        }
    };
    //get date
    if let Some(datev) = record.get(1) {
        if let Some(date) = datev.as_str() {
            let dt = NaiveDateTime::parse_from_str(date, "%Y-%m-%dT%H:%M:%S")?;
            v.date = dt.date();
        }
    };
    if let Some(rv) = record.get(2) {
        if let Some(snum) = rv.as_i64() {
            v.icu_unvac = snum;
        }
    };

    if let Some(rv) = record.get(3) {
        if let Some(snum) = rv.as_i64() {
            v.icu_partial_vac = snum;
        }
    };

    if let Some(rv) = record.get(4) {
        if let Some(snum) = rv.as_i64() {
            v.icu_full_vac = snum;
        }
    };
    if let Some(rv) = record.get(5) {
        if let Some(snum) = rv.as_i64() {
            v.hospitalnonicu_unvac = snum;
        }
    };

    if let Some(rv) = record.get(6) {
        if let Some(snum) = rv.as_i64() {
            v.hospitalnonicu_partial_vac = snum;
        }
    };

    if let Some(rv) = record.get(7) {
        if let Some(snum) = rv.as_i64() {
            v.hospitalnonicu_full_vac = snum;
        }
    };

    let _ = v.validate()?;
    Ok(v)
}

//"Date","covid19_cases_unvac","covid19_cases_partial_vac","covid19_cases_full_vac","covid19_cases_vac_unknown","cases_unvac_rate_per100K","cases_partial_vac_rate_per100K","cases_full_vac_rate_per100K","cases_unvac_rate_7ma","cases_partial_vac_rate_7ma","cases_full_vac_rate_7ma"

//new version
//"Date","covid19_cases_unvac","covid19_cases_partial_vac","covid19_cases_notfull_vac","covid19_cases_full_vac","covid19_cases_boost_vac","covid19_cases_vac_unknown","cases_unvac_rate_per100K","cases_partial_vac_rate_per100K","cases_notfull_vac_rate_per100K","cases_full_vac_rate_per100K","cases_boost_vac_rate_per100K","cases_unvac_rate_7ma","cases_partial_vac_rate_7ma","cases_notfull_vac_rate_7ma","cases_full_vac_rate_7ma","cases_boost_vac_rate_7ma"

#[derive(Deserialize, Debug)]
pub struct CsvCase {
    #[serde(rename(deserialize = "Date"))]
    pub date: String,
    pub covid19_cases_unvac: Option<i64>,
    pub covid19_cases_partial_vac: Option<i64>,
    pub covid19_cases_notfull_vac: Option<i64>,
    pub covid19_cases_full_vac: i64,
    pub covid19_cases_boost_vac: Option<i64>,
    pub covid19_cases_vac_unknown: Option<i64>,
    #[serde(rename(deserialize = "cases_unvac_rate_per100K"))]
    pub cases_unvac_rate_per100k: Option<Decimal>,
    #[serde(rename(deserialize = "cases_partial_vac_rate_per100K"))]
    pub cases_partial_vac_rate_per100k: Option<Decimal>,
    #[serde(rename(deserialize = "cases_notfull_vac_rate_per100K"))]
    pub cases_notfull_vac_rate_per100k: Option<Decimal>,
    #[serde(rename(deserialize = "cases_full_vac_rate_per100K"))]
    pub cases_full_vac_rate_per100k: Option<Decimal>,
    #[serde(rename(deserialize = "cases_boost_vac_rate_per100K"))]
    pub cases_boost_vac_rate_per100k: Option<Decimal>,
    pub cases_unvac_rate_7ma: Option<Decimal>,
    pub cases_partial_vac_rate_7ma: Option<Decimal>,
    pub cases_notfull_vac_rate_7ma: Option<Decimal>,
    pub cases_full_vac_rate_7ma: Option<Decimal>,
    pub cases_boost_vac_rate_7ma: Option<Decimal>,
}

fn transform_csv_record(r: &CsvCase) -> Result<CasesByVacStatus> {
    let mut v: CasesByVacStatus = Default::default();
    v.date = NaiveDate::parse_from_str(&r.date, "%Y-%m-%d")?;
    v.covid19_cases_unvac = r.covid19_cases_unvac;
    v.covid19_cases_partial_vac = r.covid19_cases_partial_vac.unwrap_or(0);
    v.covid19_cases_full_vac = r.covid19_cases_full_vac;
    v.covid19_cases_vac_unknown = r.covid19_cases_vac_unknown.unwrap_or(0);
    v.cases_unvac_rate_per100k = r.cases_unvac_rate_per100k.unwrap_or_else(Decimal::zero);
    v.cases_partial_vac_rate_per100k = r
        .cases_partial_vac_rate_per100k
        .unwrap_or_else(Decimal::zero);
    v.cases_full_vac_rate_per100k = r.cases_full_vac_rate_per100k.unwrap_or_else(Decimal::zero);
    v.cases_unvac_rate_7ma = r.cases_unvac_rate_7ma.unwrap_or_else(Decimal::zero);
    v.cases_partial_vac_rate_7ma = r.cases_partial_vac_rate_7ma.unwrap_or_else(Decimal::zero);
    v.cases_full_vac_rate_7ma = r.cases_full_vac_rate_7ma.unwrap_or_else(Decimal::zero);
    Ok(v)
}

#[derive(Debug)]
pub struct CsvCasesRoot(pub Vec<CsvCase>);

impl IntoIterator for CsvCasesRoot {
    type Item = Result<CasesByVacStatus>;
    type IntoIter = CsvCasesRootIterator;

    fn into_iter(self) -> Self::IntoIter {
        CsvCasesRootIterator {
            records: self.0,
            index: 0,
        }
    }
}

pub struct CsvCasesRootIterator {
    records: Vec<CsvCase>,
    index: usize,
}

impl Iterator for CsvCasesRootIterator {
    type Item = Result<CasesByVacStatus>;
    fn next(&mut self) -> Option<Self::Item> {
        match self.records.get(self.index) {
            Some(rec) => {
                self.index += 1;
                Some(transform_csv_record(rec))
            }
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decimal_scale() {
        let d = Decimal::from_str("0.05").unwrap();
        assert_eq!(d.scale(), 2);
        let divi = Decimal::from_str("1000").unwrap();
        let r = d / divi;
        assert_eq!(r.scale(), 5);
        assert_eq!(r.to_string(), "0.00005");
        let d = Decimal::from_str("5").unwrap();
        assert_eq!(d.scale(), 0);
        let divi = Decimal::from_str("100000").unwrap();
        let r = d / divi;
        assert_eq!(r.scale(), 5);
        assert_eq!(r.to_string(), "0.00005");
    }

    #[test]
    fn population_calc() {
        let cases: i64 = 1;
        let rate = Decimal::from_str("1.0").unwrap();
        assert_eq!(
            compute_total_population_from_cases_and_rate(cases, rate),
            Decimal::new(100000, 0)
        );
        let cases: i64 = 2;
        let rate = Decimal::from_str("1.0").unwrap();
        assert_eq!(
            compute_total_population_from_cases_and_rate(cases, rate),
            Decimal::new(200000, 0)
        );
        let cases: i64 = 0;
        let rate = Decimal::from_str("1.0").unwrap();
        assert_eq!(
            compute_total_population_from_cases_and_rate(cases, rate),
            Decimal::new(0, 0)
        );
        let cases: i64 = 1;
        let rate = Decimal::from_str("0.0").unwrap();
        assert_eq!(
            compute_total_population_from_cases_and_rate(cases, rate),
            Decimal::new(0, 0)
        );
    }

    #[test]
    fn index_use() {
        let i = Index::from(&["20211201", "20211215"]);
        let mr = i.most_recent();
        assert_eq!("20211215", mr);
        let i = Index::from(&["20211201", "20211215", "20211130"]);
        let mr = i.most_recent();
        assert_eq!("20211215", mr);
    }

    #[test]
    fn index_next_prev() {
        let i = Index::from(&["20211201", "20211215", "20211130"]);
        let next = i.next("20211201".to_string());
        let prev = i.prev("20211201".to_string());
        assert_eq!(Some("20211215".to_string()), next);
        assert_eq!(Some("20211130".to_string()), prev);
        let next = i.next("20211130".to_string());
        let prev = i.prev("20211130".to_string());
        assert_eq!(Some("20211201".to_string()), next);
        assert_eq!(None, prev);
        let next = i.next("20211215".to_string());
        let prev = i.prev("20211215".to_string());
        assert_eq!(None, next);
        assert_eq!(Some("20211201".to_string()), prev);
    }
}
