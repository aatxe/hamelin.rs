# hamelin.rs [![Build Status](https://travis-ci.org/aatxe/hamelin.rs.svg)](https://travis-ci.org/aatxe/hamelin.rs) #
An implementation of [Hamelin](https://github.com/Hardmath123/hamelin) in Rust.

Here's the same example from the reference implementation:

```
$ ./target/net localhost 30000 grep --line-buffered "filter" &
$ cat /usr/share/dict/words | nc localhost 30000
filter
filterability
filterable
filterableness
filterer
filtering
filterman
infilter
nonultrafilterable
prefilter
refilter
ultrafilter
ultrafilterability
ultrafilterable
unfiltered
```

Here's an example configuration file for the IRC implementation:

```json
{
    "nickname": "hamelin",
    "server": "irc.pdgn.co",
    "use_ssl": true,
    "channels": ["#pdgn"],
    "options": {}
}
```
