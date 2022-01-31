use chrono::NaiveDate;
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use ontariopublic::{CasesByVacStatus, DayReport, HospitalizationByVacStatus, Index};
use rust_decimal::Decimal;
use vax::render_report_str;

pub fn render_benchmark(c: &mut Criterion) {
    let idx = Index::from(&vec!["2022-01-01", "2022-01-02"]);
    let cases = CasesByVacStatus {
        id: 33,
        date: NaiveDate::from_ymd(2022, 01, 01),
        covid19_cases_unvac: 4,
        covid19_cases_partial_vac: 2,
        covid19_cases_full_vac: 1,
        covid19_cases_vac_unknown: 0,
        cases_unvac_rate_per100k: Decimal::new(4, 0),
        cases_partial_vac_rate_per100k: Decimal::new(2, 0),
        cases_full_vac_rate_per100k: Decimal::new(1, 0),
        cases_unvac_rate_7ma: Decimal::new(4, 0),
        cases_partial_vac_rate_7ma: Decimal::new(2, 0),
        cases_full_vac_rate_7ma: Decimal::new(1, 0),
    };
    let hosps = HospitalizationByVacStatus::default();
    let report = DayReport::from(cases, hosps);

    c.bench_function("render_report_str", |b| {
        b.iter(|| render_report_str(black_box(&idx), black_box(&report)))
    });
}

criterion_group!(benches, render_benchmark);
criterion_main!(benches);
