use ontariopublic::{DayReport, Index};
use rust_decimal::{Decimal, RoundingStrategy};
use worker::*;

mod utils;

fn log_request(req: &Request) {
    console_log!(
        "{} - [{}], located at: {:?}, within: {}",
        Date::now().to_string(),
        req.path(),
        req.cf().coordinates().unwrap_or_default(),
        req.cf().region().unwrap_or("unknown region".into())
    );
}

#[event(fetch)]
pub async fn main(req: Request, env: Env) -> Result<Response> {
    log_request(&req);

    // Optionally, get more helpful error messages written to the console in the case of a panic.
    utils::set_panic_hook();

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

static TOP: &str = r#"<!DOCTYPE html>
<html lang="en">
  <head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <meta http-equiv="X-UA-Compatible" content="ie=edge">
    <title>vax.labath.ca</title>
    <link rel="stylesheet" href="/style.css">
  </head>
  <body>"#;

static BOTTOM: &str = r#"</body></html>"#;

static TABLE: &str = r#"
<h3>Report for _DATE_VAR_</h3>
<h3>Covid-19 per capita comparison by vaccination status in Ontario, Canada.</h3>
<div>
<table>
  <tr>
    <th>Rate per 100,000</th>
    <th>0 doses</th>
    <th>2 doses</th>
  </tr>
  <tr>
    <th>Tested positive</th>
    <td>_INF_RATE_UNVAX_</td>
    <td>_INF_RATE_2VAX_</td>
  </tr>
  <tr>
    <th>Hospitalized not in ICU</th>
    <td>_HOSP_RATE_UNVAX_</td>
    <td>_HOSP_RATE_2VAX_</td>
  </tr>
  <tr>
    <th>Hospitalized in ICU</th>
    <td>_ICU_RATE_UNVAX_</td>
    <td>_ICU_RATE_2VAX_</td>
  </tr>
</table>
</div>
<div id="nav_buttons">
_PREV_VAR_
_NEXT_VAR_
</div>
"#;

fn dec_to_string(d: Decimal) -> String {
    d.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
        .to_string()
}

async fn index_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("VAXKV")?;
    let index = get_index(&kv).await?;
    let report = get_report(&kv, &index.most_recent()).await?;
    render_report(&index, &report)
}

fn render_report(index: &Index, report: &DayReport) -> Result<Response> {
    let body = String::from(TABLE);
    let date = report.cases.date.format("%A, %-d %B, %C%y").to_string();
    let body = body.replace("_DATE_VAR_", &date);
    let body = body.replace(
        "_INF_RATE_UNVAX_",
        &dec_to_string(report.cases.cases_unvac_rate_per100k),
    );
    let body = body.replace(
        "_INF_RATE_2VAX_",
        &dec_to_string(report.cases.cases_full_vac_rate_per100k),
    );
    let body = body.replace(
        "_ICU_RATE_UNVAX_",
        &dec_to_string(report.icu_unvac_rate_per100k()),
    );
    let body = body.replace(
        "_ICU_RATE_2VAX_",
        &dec_to_string(report.icu_full_vac_rate_per100k()),
    );
    let body = body.replace(
        "_HOSP_RATE_UNVAX_",
        &dec_to_string(report.nonicu_unvac_rate_per100k()),
    );
    let body = body.replace(
        "_HOSP_RATE_2VAX_",
        &dec_to_string(report.nonicu_full_vac_rate_per100k()),
    );

    let prev = match index.prev(report.key()) {
        Some(prev) => {
            let mut s = String::from("<A HREF=\"/d/");
            s.push_str(&prev);
            s.push_str("/\">Previous</A>");
            s
        }
        None => "".to_string(),
    };
    let body = body.replace("_PREV_VAR_", &prev);
    let next = match index.next(report.key()) {
        Some(next) => {
            let mut s = String::from("<A HREF=\"/d/");
            s.push_str(&next);
            s.push_str("/\">Next</A>");
            s
        }
        None => "".to_string(),
    };
    let body = body.replace("_NEXT_VAR_", &next);
    let mut body = body;
    body.insert_str(0, TOP);
    body.push_str(BOTTOM);
    Response::from_html(body)
}

async fn get_index(kv: &kv::KvStore) -> Result<Index> {
    let value = kv.get("index").await?;
    match value {
        Some(kval) => {
            let index: Index = kval.as_json()?;
            Ok(index)
        }
        None => Err("Index object not found".into()),
    }
}

async fn get_report(kv: &kv::KvStore, key: &str) -> Result<DayReport> {
    let value = kv.get(key).await?;
    match value {
        Some(kval) => {
            let rep: DayReport = kval.as_json()?;
            Ok(rep)
        }
        None => {
            let msg = String::from("Report object ");
            Err(msg.into())
        }
    }
}

async fn day_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    match ctx.param("date") {
        Some(key) => {
            let kv = ctx.kv("VAXKV")?;
            let index = get_index(&kv).await?;
            let report = get_report(&kv, key).await?;
            render_report(&index, &report)
        }
        None => Response::error("Bad Request", 400),
    }
}

static CSS: &str = r#"
body {
  background-color: white;
}
table, th, td {
  border: 1px solid black;
}
table {
  border-collapse: collapse;
}
td, th {
  text-align: center;
  padding: 15px;
  vertical-align: middle;
}
"#;

fn css_view(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    let resp = Response::ok(CSS)?;
    let mut headers = Headers::new();
    headers.set("content-type", "text/css")?;
    //tell browser to cache for few seconds
    headers.set("cache-control", "max-age=60")?;
    let resp = resp.with_headers(headers);
    Ok(resp)
}
