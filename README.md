# New Relic Synthetic Monitor Export

This program queries all synthetic monitors the user has access to from the New Relic NerdGraph API and converts the data to CSV, which is written to stdout.

Environment variables:
* `NEWRELIC_API_KEY` – a New Relic user key; required
