use chrono::{NaiveDate, NaiveDateTime};
use rust_decimal::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json;
use thiserror::Error;

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
}

//just some alias for smaller function signatures
pub type Result<T> = std::result::Result<T, DataError>;

#[derive(Deserialize, Debug)]
pub struct CasesByVacStatusRoot {
    pub fields: Vec<HeaderField>,
    pub records: Vec<Vec<serde_json::Value>>,
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

#[derive(Deserialize, Debug)]
pub struct HeaderField {}

#[derive(Deserialize, Serialize, Debug)]
pub struct CasesByVacStatus {
    id: i64,
    pub date: NaiveDate,
    covid19_cases_unvac: i64,
    covid19_cases_partial_vac: i64,
    covid19_cases_full_vac: i64,
    covid19_cases_vac_unknown: i64,
    pub cases_unvac_rate_per100k: Decimal,
    cases_partial_vac_rate_per100k: Decimal,
    pub cases_full_vac_rate_per100k: Decimal,
    cases_unvac_rate_7ma: Decimal,
    cases_partial_vac_rate_7ma: Decimal,
    cases_full_vac_rate_7ma: Decimal,
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
        if self.id < 1 {
            return Err(DataError::Invalid(self.id.to_string()));
        };
        if self.date < NaiveDate::from_ymd(2020, 07, 01) {
            return Err(DataError::Invalid(self.date.format("%Y-%m-%d").to_string()));
        };
        if self.covid19_cases_unvac < 0 {
            return Err(DataError::Invalid(self.covid19_cases_unvac.to_string()));
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
        let hundred_k = Decimal::new(100000, 0);
        if self.cases_unvac_rate_per100k < zero && self.cases_unvac_rate_per100k > hundred_k {
            return Err(DataError::Invalid(
                self.cases_unvac_rate_per100k.to_string(),
            ));
        };
        if self.cases_partial_vac_rate_per100k < zero
            && self.cases_partial_vac_rate_per100k > hundred_k
        {
            return Err(DataError::Invalid(
                self.cases_partial_vac_rate_per100k.to_string(),
            ));
        };
        if self.cases_full_vac_rate_per100k < zero && self.cases_full_vac_rate_per100k > hundred_k {
            return Err(DataError::Invalid(
                self.cases_full_vac_rate_per100k.to_string(),
            ));
        };
        if self.cases_unvac_rate_7ma < zero && self.cases_unvac_rate_7ma > hundred_k {
            return Err(DataError::Invalid(self.cases_unvac_rate_7ma.to_string()));
        };
        if self.cases_partial_vac_rate_7ma < zero && self.cases_partial_vac_rate_7ma > hundred_k {
            return Err(DataError::Invalid(
                self.cases_partial_vac_rate_7ma.to_string(),
            ));
        };
        if self.cases_full_vac_rate_7ma < zero && self.cases_full_vac_rate_7ma > hundred_k {
            return Err(DataError::Invalid(self.cases_full_vac_rate_7ma.to_string()));
        };
        Ok(())
    }

    pub fn calc_unvac_population(&self) -> Decimal {
        compute_total_population_from_cases_and_rate(
            self.covid19_cases_unvac,
            self.cases_unvac_rate_per100k,
        )
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
fn transform_record(record: &Vec<serde_json::Value>) -> Result<CasesByVacStatus> {
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
            v.covid19_cases_unvac = snum.parse::<i64>()?;
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
    if rate.is_zero() {
        return Decimal::new(0, 0);
    }
    let case_count = Decimal::new(cases, 0);
    let hundred_k = Decimal::new(100000, 0);
    case_count / (rate / hundred_k)
}

#[derive(Deserialize, Serialize, Debug)]
pub struct DayReport {
    pub cases: CasesByVacStatus,
    pub hosps: HospitalizationByVacStatus,
}

impl Default for DayReport {
    fn default() -> Self {
        DayReport {
            cases: Default::default(),
            hosps: Default::default(),
        }
    }
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
        Ok(())
    }

    pub fn key(&self) -> String {
        self.cases.date.format("%Y%m%d").to_string()
    }

    pub fn from(cases: CasesByVacStatus, hosps: HospitalizationByVacStatus) -> Self {
        DayReport {
            cases: cases,
            hosps: hosps,
        }
    }

    pub fn icu_unvac_rate_per100k(&self) -> Decimal {
        let hundred_k = Decimal::new(100000, 0);
        (Decimal::new(self.hosps.icu_unvac, 0) * hundred_k) / self.cases.calc_unvac_population()
    }

    pub fn icu_full_vac_rate_per100k(&self) -> Decimal {
        let hundred_k = Decimal::new(100000, 0);
        (Decimal::new(self.hosps.icu_full_vac, 0) * hundred_k)
            / self.cases.calc_full_vac_population()
    }

    pub fn nonicu_unvac_rate_per100k(&self) -> Decimal {
        let hundred_k = Decimal::new(100000, 0);
        (Decimal::new(self.hosps.hospitalnonicu_unvac, 0) * hundred_k)
            / self.cases.calc_unvac_population()
    }

    pub fn nonicu_full_vac_rate_per100k(&self) -> Decimal {
        let hundred_k = Decimal::new(100000, 0);
        (Decimal::new(self.hosps.hospitalnonicu_full_vac, 0) * hundred_k)
            / self.cases.calc_full_vac_population()
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Index {
    keys: Vec<String>,
}

impl Index {
    pub fn from<T: AsRef<str>>(keys: &[T]) -> Self {
        let mut v: Vec<String> = Vec::new();
        for s in keys {
            v.push(String::from(s.as_ref()));
        }
        v.sort_unstable();
        Index { keys: v }
    }

    pub fn most_recent(&self) -> String {
        match self.keys.len() {
            0 => String::from("error empty index"),
            n => String::from(&self.keys[n - 1]),
        }
    }

    pub fn next(&self, key: String) -> Option<String> {
        match self.keys.binary_search(&key) {
            Ok(idx) => match self.keys.get(idx + 1) {
                Some(v) => Some(v.to_string()),
                None => None,
            },
            Err(_) => None,
        }
    }

    pub fn prev(&self, key: String) -> Option<String> {
        match self.keys.binary_search(&key) {
            Ok(0) => None,
            Ok(idx) => match self.keys.get(idx - 1) {
                Some(v) => Some(v.to_string()),
                None => None,
            },
            Err(_) => None,
        }
    }
}

#[derive(Deserialize, Debug)]
pub struct HospitalizationByVacStatusRoot {
    pub fields: Vec<HeaderField>,
    pub records: Vec<Vec<serde_json::Value>>,
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
    icu_unvac: i64,
    icu_partial_vac: i64,
    icu_full_vac: i64,
    hospitalnonicu_unvac: i64,
    hospitalnonicu_partial_vac: i64,
    hospitalnonicu_full_vac: i64,
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
        if self.date < NaiveDate::from_ymd(2020, 07, 01) {
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

fn transform_hosp_record(record: &Vec<serde_json::Value>) -> Result<HospitalizationByVacStatus> {
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
