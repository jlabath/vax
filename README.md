# vax
source code for https://vax.labath.ca

### Data source

https://data.ontario.ca/dataset/covid-19-vaccine-data-in-ontario

Uses public data provided under the following licence https://www.ontario.ca/page/open-government-licence-ontario

### Motivation

I was looking for a quick way to compare what impact did the vaccination program have on the province of Ontario where I live.

I was also looking for a way to write some code in Rust and try [cloudflare workers](https://workers.cloudflare.com/) :-).

### Source code organization

- folder import contains the script that downloads (dwn.sh) and processes (cargo run) the ontario source data into the desired form
- ontariopublic contains the data structures that are used both by the import script and the cloudflare worker
- cfworker contains the source code for the cloudflare worker which is the web app powering https://vax.labath.ca
