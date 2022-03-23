# ULP!

> An untitled-log-parser.

Started working on this years ago now, but here is a proper open source version of the project. More features to come.

*TLDR; A Rust based parsing tool that pulls together other open source parsers to read forensic artifacts, type map/cast data, upload to elastic search for analysis or detection.*

## Change log

| Version | Date | Change(s) |
| :-----: | :--: | :-------: |
| 0.1 | 2022-03-23| Firstl release! Super early, alot needs further testing but if you know what you're looking at you'll be fine. |

## Features

Below is a table of features, both currently implemented and to be implemented.

| Feature | Is Implemented |
| :-----: | :------------: |
| Type casting | Yes! :) |
| Type mapping | Yes! :) |
| MFT Parsing | Yes! :) |
| EVTX Parsing | Yes! :) |
| Elastic Search Ingestion[^1] | Yes! :) |
| Elastic Search Indexing[^1] | Yes! :) |
| WinReg Parsing | No :( |
| Docker File / Compose[^2] | Partial! :/ |
| Custom Index pattern[^3] | Partial :/ |
| Custom Parser options | No :( |
| Custom DB options | No :( |
| Custom Fields | No :( |
| CLI interface | No :( |
| Basic API routes | Yes! :) |
| Adv API management routes[^4] |  No :( |
| Enrichment options |  No :( |

[^1]: Further testing required to validate type casting covers all edge cases and is resiliant.
[^2]: Still need to sort out enviroment variables and ensure they're being used properly by ULP.
[^3]: Pattern string (ie. `evtx_{{Event.System.ProviderName}}`) parsing is implimented but the mechanism of passing them through to parsing jobs isn't supported. A redesign of the input methods is required, simple but will take time.
[^4]: Expect more options on parsing and grouping elastic jobs, combining index maps is supported so in future having data from different artifacts in the same file will be possible if needed. Additionally more data that can be submitted via the API 


Plenty more to add as this project grows.

## Usage

### Docker / Docker-Compose

### Making API requests

There are two main API requests that are used (as of `v0.1`), `POST /job/{path glob}` and `POST /elastic/{uuid}`
cargo run as usual and do one of these to point at a file path/glob

```bash
# Parse all EVTX files in /forensic_data/ and child directories.
$ curl -XPOST "0.0.0.0:3030/job" -H 'content-type: application/json' -d '"/forensic_data/**/*.evtx"'
# Parse all MFTs in /forensic_data/ and child directories.
$ curl -XPOST "0.0.0.0:3030/job" -H 'content-type: application/json' -d '"/forensic_data/**/$MFT"'
# Read all data assigned to a particular job to Elastic
$ curl -XPOST "0.0.0.0:3030/elastic " -H 'content-type: application/json' -d '"e24c14c0-342f-4c24-8b57-d9dcd3ec5936"'
```

Run the binary with `RUST_LOG=ulp=info`, `RUST_LOG=ulp=debug`, or `RUST_LOG=ulp=error` for different views on the data.