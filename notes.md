# Notes

## info

- [batch-resolve](https:-github.com/mersinvald/batch_resolve)
- [batch-resolve](https:-www.reddit.com/r/rust/comments/6a9i9a/batch_resolve_fast_asynchronous_dns_resolver/)

- [async](https:-bryangilbert.com/post/code/rust/adventures-futures-tokio-rust/)
- [async](https:-tech.marksblogg.com/rdns-domain-name-tld-extract-rust.html)
- [async](https:-askubuntu.com/questions/813275/how-to-check-a-bulk-of-ip-for-reverse-dns)

- [threads](https:-users.rust-lang.org/t/please-recommend-a-queue-with-backpressure-for-simple-threads-no-async-yet/68654/3)

## geoip

### curl [geoip](https://api.ipgeolocation.io/ipgeo?apiKey=API_KEY&ip=8.8.8.8)

## road map

- ~~add indices~~
- ~~separate out exists fn~~
- ~~filter out of all_ips those already looked up~~
- add cli options to choose what kind of output
- add db selection by time
- serializing Datetimes [dt](https://docs.rs/bson/latest/bson/struct.DateTime.html)

## cli

- options -D daemon-mode -o onlyheaders subcommand [time -s date -e date -d # -h #] [ip string] [org string] [country string] (-f) FILE(w default)

## Rust error handling with anyhow

- [errors](https://antoinerr.github.io/blog-website/2023/01/28/rust-anyhow.html)
