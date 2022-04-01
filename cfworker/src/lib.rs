use num_format::{Locale, ToFormattedString};
use ontariopublic::{DayReport, Index};
use rust_decimal::{prelude::ToPrimitive, Decimal, RoundingStrategy};
use worker::*;

//how long in seconds to cache key value store get results for
const TTL_CACHE: u64 = 60;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or_else(|| "unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env, _ctx: Context) -> Result<Response> {
    log_request(&req);

    // Optionally, use the Router to handle matching endpoints, use ":name" placeholders, or "*name"
    // catch-alls to match on specific patterns. Alternatively, use `Router::with_data(D)` to
    // provide arbitrary data that will be accessible in each route via the `ctx.data()` method.
    let router = Router::new();

    // Add as many routes as your Worker needs! Each route will get a `Request` for handling HTTP
    // functionality and a `RouteContext` which you can use to  and get route parameters and
    // Environment bindings like KV Stores, Durable Objects, Secrets, and Variables.
    let res = router
        .get_async("/", index_view)
        .get_async("/d/:date/", day_view)
        .get_async("/dd/:date/", day_detail_view)
        .get_async("/di/:idx/", idx_view)
        .get_async("/ch/ca/", chart_cases_view)
        .get_async("/ch/ni/", chart_nonicu_view)
        .get_async("/ch/ii/", chart_icu_view)
        .get("/style.css", css_view)
        .get("/worker-version", |_, ctx| {
            let version = ctx.var("WORKERS_RS_VERSION")?.to_string();
            Response::ok(version)
        })
        .run(req, env)
        .await;
    // evaluate how our router did - were there any errors - if so log them
    match res {
        Ok(response) => Ok(response),
        Err(err) => {
            console_log!("main error => {} | {:?}", err, err);
            Response::error("Sorry, there are some technical difficulties.", 500)
        }
    }
}

static SIMPLETOP: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="X-UA-Compatible" content="ie=edge">
    <title>vax.labath.ca</title>
    <link rel="stylesheet" href="/style.css">"#;

static BOTTOM: &str = r#"</body></html>"#;

fn dec_to_string(d: Decimal) -> String {
    d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
        .to_string()
}

fn human_string(d: i64) -> String {
    d.to_formatted_string(&Locale::en)
}

async fn index_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("VAXKV")?;
    let index = get_index(&kv).await?;
    let report = get_report(&kv, &index.most_recent()).await?;
    let body = render_report_str(&index, &report);
    craft_response("text/html", &body)
}

pub fn render_report_str(index: &Index, report: &DayReport) -> String {
    let date = report.cases.date.format("%A, %-d %B, %C%y").to_string();
    let updated = index.updated.to_rfc2822();
    let inf_rate_unvax = report
        .cases
        .cases_unvac_rate_per100k
        .map_or_else(|| String::from("n/a"), dec_to_string);
    let inf_rate_lt_2vax = report
        .cases
        .cases_notfull_vac_rate_per100k
        .map_or_else(|| String::from("n/a"), dec_to_string);
    let inf_rate_2vax = dec_to_string(report.cases.cases_full_vac_rate_per100k);
    let icu_rate_unvax = report
        .icu_unvac_rate_per100k()
        .map_or_else(|| String::from("n/a"), dec_to_string);
    let icu_rate_lt_2vax = report
        .icu_notfull_vac_rate_per100k()
        .map_or_else(|| String::from("n/a"), dec_to_string);
    let icu_rate_2vax = dec_to_string(report.icu_full_vac_rate_per100k());
    let hosp_rate_unvax = report
        .nonicu_unvac_rate_per100k()
        .map_or_else(|| String::from("n/a"), dec_to_string);
    let hosp_rate_2vax = dec_to_string(report.nonicu_full_vac_rate_per100k());
    let hosp_rate_lt_2vax = report
        .nonicu_notfull_vac_rate_per100k()
        .map_or_else(|| String::from("n/a"), dec_to_string);
    let max_idx = index.max_idx();
    let idx = index.idx(report.key()).unwrap_or_else(|| index.max_idx());
    let cur_key = report.key();
    let prev = match index.prev(report.key()) {
        Some(prev) => {
            let mut s = String::from("<A HREF=\"/d/");
            s.push_str(&prev);
            s.push_str("/\">Previous</A>");
            s
        }
        None => "".to_string(),
    };
    let next = match index.next(report.key()) {
        Some(next) => {
            let mut s = String::from("<A HREF=\"/d/");
            s.push_str(&next);
            s.push_str("/\">Next</A>");
            s
        }
        None => "".to_string(),
    };
    format!(
        r#"{SIMPLETOP}
    <script>
      window.onload = (event) => {{
        var slider = document.getElementById("dayRange");
        slider.onchange = (event) => {{
          let url = "/di/"+slider.value+"/";
          window.location.assign(url);
        }};
      }};
    </script>
  </head>
<body>
<h3>Report for {date}</h3>
<h3>COVID-19 per capita comparison by vaccination status in Ontario, Canada.</h3>
<div><a href="/dd/{cur_key}/">Click here for detailed report</a></div>
<div id="main">
<table>
  <tr>
    <td>Rate per 100,000</td>
    <td>0 doses</td>
    <td>&lt; 2 doses</td>
    <td>2 doses</td>
  </tr>
  <tr>
    <td><a href="/ch/ca/">Tested positive</a></td>
    <th>{inf_rate_unvax}</th>
    <th>{inf_rate_lt_2vax}</th>
    <th>{inf_rate_2vax}</th>
  </tr>
  <tr>
    <td><a href="/ch/ni/">Hospitalized not in ICU</a></td>
    <th>{hosp_rate_unvax}</th>
    <th>{hosp_rate_lt_2vax}</th>
    <th>{hosp_rate_2vax}</th>
  </tr>
  <tr>
    <td><a href="/ch/ii/">Hospitalized in ICU</a></td>
    <th>{icu_rate_unvax}</th>
    <th>{icu_rate_lt_2vax}</th>
    <th>{icu_rate_2vax}</th>
  </tr>
</table>
</div>
<div class="slidecontainer">
  <input type="range" min="0" max="{max_idx}" value="{idx}" class="slider" id="dayRange">
</div>
<div id="nav_buttons">
{prev}
{next}
</div>
<div id="footer"><a href="https://github.com/jlabath/vax">source code</a></div>
<div id="updated"><h5>Last updated: {updated}</h5></div>
{BOTTOM}
"#
    )
}

pub fn render_detail_report_str(index: &Index, report: &DayReport) -> String {
    let date = report.cases.date.format("%A, %-d %B, %C%y").to_string();
    let updated = index.updated.to_rfc2822();
    let inf_rate_unvax = report
        .cases
        .cases_unvac_rate_per100k
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let inf_rate_notfull_vax = report
        .cases
        .cases_notfull_vac_rate_per100k
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let inf_rate_2vax = dec_to_string(report.cases.cases_full_vac_rate_per100k);
    let inf_rate_1vax = report
        .cases
        .cases_partial_vac_rate_per100k
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let inf_rate_boost_vax = report
        .cases
        .cases_boost_vac_rate_per100k
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let inf_rate_unvax_ma = report
        .cases
        .cases_unvac_rate_7ma
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let inf_rate_2vax_ma = report
        .cases
        .cases_full_vac_rate_7ma
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let inf_rate_1vax_ma = report
        .cases
        .cases_partial_vac_rate_7ma
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let inf_rate_notfull_ma = report
        .cases
        .cases_notfull_vac_rate_7ma
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let inf_rate_boost_ma = report
        .cases
        .cases_boost_vac_rate_7ma
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let icu_rate_unvax = report
        .icu_unvac_rate_per100k()
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let icu_rate_1vax = report
        .icu_partial_vac_rate_per100k()
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let icu_rate_lt_2vax = report
        .icu_notfull_vac_rate_per100k()
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let icu_rate_2vax = dec_to_string(report.icu_full_vac_rate_per100k());
    let hosp_rate_unvax = report
        .nonicu_unvac_rate_per100k()
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let hosp_rate_1vax = report
        .nonicu_partial_vac_rate_per100k()
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let hosp_rate_lt_2vax = report
        .nonicu_notfull_vac_rate_per100k()
        .map_or_else(|| String::from("N/A"), dec_to_string);
    let hosp_rate_2vax = dec_to_string(report.nonicu_full_vac_rate_per100k());
    let cases_unvax = report
        .cases
        .covid19_cases_unvac
        .map_or_else(|| String::from("N/A"), human_string);
    let cases_partial_vax = report
        .cases
        .covid19_cases_partial_vac
        .map_or_else(|| String::from("N/A"), human_string);
    let cases_notfull_vax = report
        .cases
        .covid19_cases_notfull_vac
        .map_or_else(|| String::from("N/A"), human_string);
    let cases_full_vax = human_string(report.cases.covid19_cases_full_vac);
    let cases_boost_vax = report
        .cases
        .covid19_cases_boost_vac
        .map_or_else(|| String::from("N/A"), human_string);
    let cases_unknown_vax = report
        .cases
        .covid19_cases_vac_unknown
        .map_or_else(|| String::from("N/A"), human_string);
    let icu_unvax = human_string(report.hosps.icu_unvac);
    let icu_1vax = human_string(report.hosps.icu_partial_vac);
    let icu_2vax = human_string(report.hosps.icu_full_vac);
    let hosp_unvax = human_string(report.hosps.hospitalnonicu_unvac);
    let hosp_1vax = human_string(report.hosps.hospitalnonicu_partial_vac);
    let hosp_2vax = human_string(report.hosps.hospitalnonicu_full_vac);
    let pop_unvax = report.cases.calc_unvac_population().map_or_else(
        || String::from("N/A"),
        |v| {
            human_string(
                v.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
                    .to_i64()
                    .unwrap_or(0),
            )
        },
    );
    let pop_1vax = report.cases.calc_partial_vac_population().map_or_else(
        || String::from("N/A"),
        |v| {
            human_string(
                v.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
                    .to_i64()
                    .unwrap_or(0),
            )
        },
    );
    let pop_lt_2vax = report.cases.calc_notfull_vac_population().map_or_else(
        || String::from("N/A"),
        |v| {
            human_string(
                v.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
                    .to_i64()
                    .unwrap_or(0),
            )
        },
    );
    let pop_2vax = human_string(
        report
            .cases
            .calc_full_vac_population()
            .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
            .to_i64()
            .unwrap_or(0),
    );
    let pop_3vax = report.cases.calc_boost_vac_population().map_or_else(
        || String::from("N/A"),
        |v| {
            human_string(
                v.round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
                    .to_i64()
                    .unwrap_or(0),
            )
        },
    );
    let prev = match index.prev(report.key()) {
        Some(prev) => {
            let mut s = String::from("<A HREF=\"/dd/");
            s.push_str(&prev);
            s.push_str("/\">Previous</A>");
            s
        }
        None => "".to_string(),
    };
    let next = match index.next(report.key()) {
        Some(next) => {
            let mut s = String::from("<A HREF=\"/dd/");
            s.push_str(&next);
            s.push_str("/\">Next</A>");
            s
        }
        None => "".to_string(),
    };
    let cur_key = report.key();
    format!(
        r#"{SIMPLETOP}
  </head>
<body>
<h3>Detailed report for {date}, Ontario, Canada.</h3>
<div>
<a href="/d/{cur_key}/">Back to compare view</a>
<h3>COVID-19 cases by vaccination status</h3>
<table>
  <tr>
    <td class="label">Unvaccinated</td>
    <td class="num">{cases_unvax}</td>
    <td>Number of people who tested positive for COVID-19 on this date. Individuals are considered unvaccinated if they have not had a dose, or if their first dose was less than fourteen days ago.</td>
  </tr>
  <tr>
    <td class="label">Partially Vaccinated</td>
    <td class="num">{cases_partial_vax}</td>
    <td>Number of people who tested positive for COVID-19 on this date.  Individuals are considered partially vaccinated if they have had one dose at least fourteen days ago, or two doses where the second dose was less than fourteen days ago.</td>
  </tr>
  <tr>
    <td class="label">Not fully vaccinated (less than 2 doses)</td>
    <td class="num">{cases_notfull_vax}</td>
    <td>Number of not fully vaccinated people who tested positive for COVID-19 on this date. Individuals are considered fully vaccinated if they have had two doses and the second dose was at least fourteen days ago. New datapoint as of March 11, 2022.</td>
  </tr>
  <tr>
    <td class="label">Fully vaccinated</td>
    <td class="num">{cases_full_vax}</td>
    <td>Number of people who tested positive for COVID-19 on this date. Individuals are considered fully vaccinated if they have had two doses and the second dose was at least fourteen days ago.</td>
  </tr>
  <tr>
    <td class="label">Boosted (3 doses)</td>
    <td class="num">{cases_boost_vax}</td>
    <td>Number of boosted people who tested positive for COVID-19 on this date. Individuals are considered fully vaccinated if they have had three doses and the third dose was at least fourteen days ago. New datapoint as of March 11, 2022.</td>
  </tr>
  <tr>
    <td class="label">Unknown vaccination status</td>
    <td class="num">{cases_unknown_vax}</td>
    <td>Number of people who tested positive for COVID-19 on this date, but their vaccination status is unknown.</td>
  </tr>
  <tr>
    <td class="label">Unvaccinated rate per 100,000</td>
    <td class="num">{inf_rate_unvax}</td>
    <td>Rate of COVID-19 cases per 100,000 of unvaccinated people (calculated by dividing the number of cases for a vaccination status, by the total number of people with the same vaccination status and then multiplying by 100,000).</td>
  </tr>
  <tr>
    <td class="label">Partially vaccinated rate per 100,000</td>
    <td class="num">{inf_rate_1vax}</td>
    <td>Rate of COVID-19 cases per 100,000 of partially vaccinated people (calculated by dividing the number of cases for a vaccination status, by the total number of people with the same vaccination status and then multiplying by 100,000).</td>
  </tr>
  <tr>
    <td class="label">Not fully vaccinated rate per 100,000</td>
    <td class="num">{inf_rate_notfull_vax}</td>
    <td>Rate of COVID-19 cases per 100,000 of not fully vaccinated people (calculated by dividing the number of cases for a vaccination status, by the total number of people with the same vaccination status and then multiplying by 100,000). New as of March 11, 2022.</td>
  </tr>
  <tr>
    <td class="label">Fully vaccinated rate per 100,000</td>
    <td class="num">{inf_rate_2vax}</td>
    <td>Rate of COVID-19 cases per 100,000 of fully vaccinated people (calculated by dividing the number of cases for a vaccination status, by the total number of people with the same vaccination status and then multiplying by 100,000).</td>
  </tr>
  <tr>
    <td class="label">Boosted rate per 100,000</td>
    <td class="num">{inf_rate_boost_vax}</td>
    <td>Rate of COVID-19 cases per 100,000 of fully vaccinated people (calculated by dividing the number of cases for a vaccination status, by the total number of people with the same vaccination status and then multiplying by 100,000). New as of March 11, 2022.</td>
  </tr>
  <tr>
    <td class="label">Unvaccinated rate per 100,000 (7 day moving average)</td>
    <td class="num">{inf_rate_unvax_ma}</td>
    <td>The average rate of COVID-19 cases per 100,000 for the previous 7 days for this vaccination status.</td>
  </tr>
  <tr>
    <td class="label">Partially vaccinated rate per 100,000 (7 day moving average)</td>
    <td class="num">{inf_rate_1vax_ma}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Not fully vaccinated rate per 100,000 (7 day moving average)</td>
    <td class="num">{inf_rate_notfull_ma}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Fully vaccinated rate per 100,000 (7 day moving average)</td>
    <td class="num">{inf_rate_2vax_ma}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Boosted rate per 100,000 (7 day moving average)</td>
    <td class="num">{inf_rate_boost_ma}</td>
    <td>&#x3003;</td>
  </tr>
</table>
<h3>COVID-19 hospitalizations by vaccination status</h3>
<h5>Due to incomplete weekend and holiday reporting, vaccination status data for hospital and ICU admissions is not updated on Sundays, Mondays and the day after holidays.</h5>
<table>
 <tr>
    <td class="label">Unvaccinated hospitalized but not in ICU</td>
    <td class="num">{hosp_unvax}</td>
    <td>Number of people admitted to a hospital but not requiring a stay in ICU. In order to understand the vaccination status of patients currently hospitalized, a new data collection process was developed and this may cause discrepancies between other hospitalization numbers being collected using a different data collection process.</td>
  </tr>
 <tr>
    <td class="label">Partially vaccinated hospitalized but not in ICU</td>
    <td class="num">{hosp_1vax}</td>
    <td>&#x3003;</td>
  </tr>
 <tr>
    <td class="label">Fully vaccinated hospitalized but not in ICU</td>
    <td class="num">{hosp_2vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Unvaccinated in ICU</td>
    <td class="num">{icu_unvax}</td>
    <td>Number of people hospitalized in ICU with COVID-19. Data on patients in ICU are being collected from two different data sources with different extraction times and public reporting cycles. The existing data source (Critical Care Information System, CCIS) does not have vaccination status.</td>
  </tr>
 <tr>
    <td class="label">Partially vaccinated in ICU</td>
    <td class="num">{icu_1vax}</td>
    <td>&#x3003;</td>
  </tr>
 <tr>
    <td class="label">Fully vaccinated in ICU</td>
    <td class="num">{icu_2vax}</td>
    <td>&#x3003;</td>
  </tr>
</table>
<h3>Computed metrics based on government data above</h3>
<h5>The per 100,000 scale is used when the more commonly used per cent (per hundred) scale would result in very small decimal numbers. E.g. 1 in 100,000 equals 0.001%, or 1000 in 100,000 equals 1% of the population.</h5>
<table>
  <tr>
    <td class="label">Number of unvaccinated people in Ontario</td>
    <td class="num">{pop_unvax}</td>
    <td>Calculated as case count for this vaccination status times 100,000 and then divided by rate and rounded.</td>
  </tr>
  <tr>
    <td class="label">Number of partially vaccinated people in Ontario</td>
    <td class="num">{pop_1vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Number of not fully vaccinated people in Ontario (less than 2 doses)</td>
    <td class="num">{pop_lt_2vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Number of fully vaccinated people in Ontario</td>
    <td class="num">{pop_2vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Number of boosted people in Ontario</td>
    <td class="num">{pop_3vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Non ICU hospitalization rate of unvaccinated per 100,000</td>
    <td class="num">{hosp_rate_unvax}</td>
    <td>Calculated as number of non ICU hospitalizations for this vaccination status times 100,000 and then divided by the population for this vaccination status.</td>
  </tr>
  <tr>
    <td class="label">Non ICU hospitalization rate of partially vaccinated per 100,000</td>
    <td class="num">{hosp_rate_1vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Non ICU hospitalization rate of not fully vaccinated per 100,000</td>
    <td class="num">{hosp_rate_lt_2vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">Non ICU hospitalization rate of fully vaccinated per 100,000</td>
    <td class="num">{hosp_rate_2vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">ICU hospitalization rate of unvaccinated per 100,000</td>
    <td class="num">{icu_rate_unvax}</td>
    <td>Calculated as number of ICU hospitalizations for this vaccination status times 100,000 and then divided by the population for this vaccination status.</td>
  </tr>
  <tr>
    <td class="label">ICU hospitalization rate of partially vaccinated per 100,000</td>
    <td class="num">{icu_rate_1vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">ICU hospitalization rate of not fully vaccinated per 100,000</td>
    <td class="num">{icu_rate_lt_2vax}</td>
    <td>&#x3003;</td>
  </tr>
  <tr>
    <td class="label">ICU hospitalization rate of fully vaccinated per 100,000</td>
    <td class="num">{icu_rate_2vax}</td>
    <td>&#x3003;</td>
  </tr>
</table>
<a href="/d/{cur_key}/">Back to compare view</a>
</div>
<div id="nav_buttons">
{prev}
{next}
</div>
<div id="footer"><a href="https://github.com/jlabath/vax">source code</a></div>
<div id="updated"><h5>Last updated: {updated}</h5></div>
{BOTTOM}
"#
    )
}

async fn get_index(kv: &kv::KvStore) -> Result<Index> {
    let index_opt = kv.get("index").cache_ttl(TTL_CACHE).json().await?;
    index_opt.ok_or_else(|| "Could not fetch index".into())
}

async fn get_report(kv: &kv::KvStore, key: &str) -> Result<DayReport> {
    let opt = kv.get(key).cache_ttl(TTL_CACHE).json().await?;
    opt.ok_or_else(|| {
        let mut msg = String::from("Could not fetch report: ");
        msg.push_str(key);
        msg.into()
    })
}

async fn day_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    match ctx.param("date") {
        Some(key) => {
            let kv = ctx.kv("VAXKV")?;
            let index = get_index(&kv).await?;
            let report = get_report(&kv, key).await?;
            let body = render_report_str(&index, &report);
            craft_response("text/html", &body)
        }
        None => Response::error("Bad Request", 400),
    }
}

async fn day_detail_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    match ctx.param("date") {
        Some(key) => {
            let kv = ctx.kv("VAXKV")?;
            let index = get_index(&kv).await?;
            let report = get_report(&kv, key).await?;
            let body = render_detail_report_str(&index, &report);
            craft_response("text/html", &body)
        }
        None => Response::error("Bad Request", 400),
    }
}

async fn idx_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    match ctx.param("idx") {
        Some(sidx) => {
            let kv = ctx.kv("VAXKV")?;
            let index = get_index(&kv).await?;
            if let Ok(idx) = sidx.parse::<usize>() {
                let key = match index.get(idx) {
                    Some(k) => k,
                    None => "fail".to_string(),
                };
                let report = get_report(&kv, &key).await?;
                let body = render_report_str(&index, &report);
                craft_response("text/html", &body)
            } else {
                Response::error("Bad Request", 400)
            }
        }
        None => Response::error("Bad Request", 400),
    }
}

static CSS: &str = r#"
body {
  background-color: white;
  padding-left: 1em;
}
table, th, td {
  border: 1px solid black;
}
table {
  border-collapse: collapse;
}
td, th {
  text-align: center;
  padding: 1em;
  vertical-align: middle;
}
td.label {
  text-align: left;
}
td.num {
  text-align: right;
}
.slidecontainer {
  height: 4em;
  /* Center vertically */
  display: flex;
  align-items: center;
  width: 21em;
}
#nav_buttons {
  height: 4em;
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 21em;
}
#footer {
  height: 8em;
  display: flex;
  align-items: end;
  width: 21em;
  justify-content: center;
}
#updated {
  height: 4em;
  display: flex;
  align-items: end;
  width: 21em;
  justify-content: center;
}
#dayRange {
  width: 100%;
}
#main {
  margin-top: 1em;
}
"#;

fn css_view(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    craft_response("text/css", CSS)
}

async fn chart_cases_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("VAXKV")?;
    let dose0 = get_kvval(&kv, "cases_dose0").await?;
    let dose2 = get_kvval(&kv, "cases_dose2").await?;
    let dose_lt2 = get_kvval(&kv, "cases_dose_lt2").await?;
    let title =
        "<h3>COVID-19 cases by vaccination status per 100,000 people in Ontario, Canada.</h3>";
    chart_response(&kv, title, &dose0, &dose2, &dose_lt2).await
}

static CHART_JS: &str = r#"
window.onload = (event) => {
  const data = {
    labels: labels,
    datasets: [{
      label: '0 doses',
      data: dose0,
      fill: false,
      borderColor: 'red',
      tension: 0.1
    },{
      label: '2 doses',
      data: dose2,
      fill: false,
      borderColor: 'green',
      tension: 0.1
    },{
      label: 'less than 2 doses',
      data: dose_lt2,
      fill: false,
      borderColor: 'maroon',
      tension: 0.1
    }]
  };
  const config = {
    type: 'line',
    data: data,
    options: {
      responsive: true
    }
  };
  const myChart = new Chart(
    document.getElementById('myChart'),
     config
  );
}
"#;

async fn chart_response(
    kv: &kv::KvStore,
    title: &str,
    dose0: &str,
    dose2: &str,
    dose_lt2: &str,
) -> Result<Response> {
    let labels = get_kvval(kv, "labels").await?;
    let mut body = String::with_capacity(1024 * 5); //5k
    body.push_str(SIMPLETOP);
    body.push_str(
        r#"
<script src="https://cdnjs.cloudflare.com/ajax/libs/Chart.js/3.7.0/chart.min.js"></script>
<script>
  const labels = "#,
    );
    body.push_str(&labels);
    body.push_str(
        r#";
  const dose0 = "#,
    );
    body.push_str(dose0);
    body.push_str(
        r#";
  const dose2 = "#,
    );
    body.push_str(dose2);
    body.push(';');
    body.push_str(
        r#";
  const dose_lt2 = "#,
    );
    body.push_str(dose_lt2);
    body.push(';');
    body.push_str(CHART_JS);
    body.push_str("</script></head><body>");
    body.push_str(title);
    body.push_str(
        r#"
<div><a href="/" alt="home">&#8701; home</a></div>
<div class="chart-container" style="position: relative; height:80vh; width:95vw">
  <canvas id="myChart"></canvas>
</div>
"#,
    );
    body.push_str(BOTTOM);
    craft_response("text/html", &body)
}

fn craft_response(content_type: &str, body: &str) -> Result<Response> {
    let data = body.as_bytes().to_vec();
    let mut resp = Response::from_body(ResponseBody::Body(data))?;
    let headers = resp.headers_mut();
    headers.set("content-type", content_type)?;
    //tell browser to cache for few seconds
    headers.set("cache-control", "max-age=180")?;
    Ok(resp)
}

async fn get_kvval(kv: &kv::KvStore, key: &str) -> Result<String> {
    let value = kv.get(key).cache_ttl(TTL_CACHE).text().await?;
    value.ok_or_else(|| {
        let mut msg = String::from("No such value in keystore -> ");
        msg.push_str(key);
        msg.into()
    })
}

async fn chart_nonicu_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("VAXKV")?;
    let dose0 = get_kvval(&kv, "nonicu_dose0").await?;
    let dose2 = get_kvval(&kv, "nonicu_dose2").await?;
    let dose_lt2 = get_kvval(&kv, "nonicu_dose_lt2").await?;
    let title =
        "<h3>COVID-19 hospitalizations (not in ICU) by vaccination status per 100,000 people in Ontario, Canada.</h3>";
    chart_response(&kv, title, &dose0, &dose2, &dose_lt2).await
}

async fn chart_icu_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("VAXKV")?;
    let dose0 = get_kvval(&kv, "icu_dose0").await?;
    let dose2 = get_kvval(&kv, "icu_dose2").await?;
    let dose_lt2 = get_kvval(&kv, "icu_dose_lt2").await?;
    let title =
        "<h3>COVID-19 hospitalization in ICU by vaccination status per 100,000 people in Ontario, Canada.</h3>";
    chart_response(&kv, title, &dose0, &dose2, &dose_lt2).await
}
