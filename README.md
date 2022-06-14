# New Relic Synthetic Monitor Export

This program queries all synthetic monitors the user has access to from the New Relic NerdGraph API and converts the data to CSV, which is written to the file `output/monitor.csv`. For each scripted monitor, the script is written to a file in `output/scripts`.

Environment variables:
* `NEWRELIC_API_KEY` – a New Relic user key; required

Running:
```sh
export NEWRELIC_API_KEY='your_key'
mkdir -p output/scripts
cargo run
```
