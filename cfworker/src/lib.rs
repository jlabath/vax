use ontariopublic::{DayReport, Index};
use rust_decimal::{Decimal, RoundingStrategy};
use worker::*;

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

static REPORT_JS: &str = r#"
    <script>
      window.onload = (event) => {
        var slider = document.getElementById("dayRange");
        slider.onchange = (event) => {
          let url = "/di/"+slider.value+"/";
          window.location.assign(url);
        };
      };
    </script>
  </head>
  <body>"#;

static BOTTOM: &str = r#"</body></html>"#;

static TABLE: &str = r#"
<h3>Report for _DATE_VAR_</h3>
<h3>Covid-19 per capita comparison by vaccination status in Ontario, Canada.</h3>
<div>
<table>
  <tr>
    <td>Rate per 100,000</td>
    <td>0 doses</td>
    <td>2 doses</td>
  </tr>
  <tr>
    <td><a href="/ch/ca/">Tested positive</a></td>
    <th>_INF_RATE_UNVAX_</th>
    <th>_INF_RATE_2VAX_</th>
  </tr>
  <tr>
    <td><a href="/ch/ni/">Hospitalized not in ICU</a></td>
    <th>_HOSP_RATE_UNVAX_</th>
    <th>_HOSP_RATE_2VAX_</th>
  </tr>
  <tr>
    <td><a href="/ch/ii/">Hospitalized in ICU</a></td>
    <th>_ICU_RATE_UNVAX_</th>
    <th>_ICU_RATE_2VAX_</th>
  </tr>
</table>
</div>
<div class="slidecontainer">
  <input type="range" min="0" max="_MAX_RANGE_VAR_" value="_CUR_RANGE_VAR_" class="slider" id="dayRange">
</div>
<div id="nav_buttons">
_PREV_VAR_
_NEXT_VAR_
</div>

<div id="footer"><a href="https://github.com/jlabath/vax">source code</a></div>
<div id="updated"><h5>Last updated: _UPDATED_VAR_</h5></div>
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
    let updated = index.updated.to_rfc2822();
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
    let body = body.replace("_MAX_RANGE_VAR_", &index.max_idx().to_string());
    let idx = index.idx(report.key()).unwrap_or_else(|| index.max_idx());
    let body = body.replace("_CUR_RANGE_VAR_", &idx.to_string());

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
    let body = body.replace("_UPDATED_VAR_", &updated);
    let mut body = body;
    body.insert_str(0, SIMPLETOP);
    body.insert_str(SIMPLETOP.len(), REPORT_JS);
    body.push_str(BOTTOM);
    craft_response("text/html", &body)
}

async fn get_index(kv: &kv::KvStore) -> Result<Index> {
    let index_opt = kv.get("index").json().await?;
    index_opt.ok_or_else(|| "Could not fetch index".into())
}

async fn get_report(kv: &kv::KvStore, key: &str) -> Result<DayReport> {
    let opt = kv.get(key).json().await?;
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
            render_report(&index, &report)
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
                render_report(&index, &report)
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
"#;

fn css_view(_req: Request, _ctx: RouteContext<()>) -> Result<Response> {
    craft_response("text/css", CSS)
}

async fn chart_cases_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("VAXKV")?;
    let dose0 = get_kvval(&kv, "cases_dose0").await?;
    let dose2 = get_kvval(&kv, "cases_dose2").await?;
    let title =
        "<h3>Covid-19 cases by vaccination status per 100,000 people in Ontario, Canada.</h3>";
    chart_response(&kv, title, &dose0, &dose2).await
}

static CHART_JS: &str = r#"
window.onload = (event) => {
  const data = {
    labels: labels,
    datasets: [{
      label: 'Population with 0 doses of the vaccine',
      data: dose0,
      fill: false,
      borderColor: 'red',
      tension: 0.1
    },{
      label: 'Population with 2 doses of the vaccine',
      data: dose2,
      fill: false,
      borderColor: 'green',
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
) -> Result<Response> {
    let labels = get_kvval(&kv, "labels").await?;
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
    body.push_str(";");
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
    let value = kv.get(key).text().await?;
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
    let title =
        "<h3>Covid-19 hospitalizations (not in ICU) by vaccination status per 100,000 people in Ontario, Canada.</h3>";
    chart_response(&kv, title, &dose0, &dose2).await
}

async fn chart_icu_view(_req: Request, ctx: RouteContext<()>) -> Result<Response> {
    let kv = ctx.kv("VAXKV")?;
    let dose0 = get_kvval(&kv, "icu_dose0").await?;
    let dose2 = get_kvval(&kv, "icu_dose2").await?;
    let title =
        "<h3>Covid-19 hospitalization in ICU by vaccination status per 100,000 people in Ontario, Canada.</h3>";
    chart_response(&kv, title, &dose0, &dose2).await
}
