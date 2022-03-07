# ulp
An untitled-log-parser

cargo run as usual and do one of these to point at a file path/glob

curl -XPOST "0.0.0.0:3030/job" -H 'content-type: application/json' -d '"/home/hts/Git/ulp/.test_data/Logs/*.evtx"'