# Torrust Bencode2Json

A lib and console command to convert from bencoded data to JSON format.

Output is similar to: <https://github.com/Chocobo1/bencode_online>. When a bencoded string (byte string) contains only valid UTF-8 chars, the output will print those chars. If the string contains non valid UTF-8 chars, them the string will be printed in hexadecimal. For example:

Bencoded string (with 2 bytes):

```text
2:\xFF\xFE
```

JSON string:

```text
<hex>fffe</hex>
```

More info: <https://github.com/torrust/teps/discussions/15>

## Run

Run the binary with input and output file:

```console
cargo run -- -i tests/sample.bencode -o output.json
```

Run the binary with stdin and stdout:

```console
echo "4:spam" | cargo run
"spam"
```

Run the binary with stdin and stdout:

```console
printf "d3:bar2:\xFF\xFEe" | cargo run
{"bar":"<hex>fffe</hex>"}
```

```console
printf "d2:\xFF\xFE3:bare" | cargo run
{"<hex>fffe</hex>":"bar"}
```

> NOTICE: We need two escape the two bytes `FF` and `FE` with `\x` inside the string.

## Test

Run unit and integration tests:

```console
cargo test
```

We have included a copy of another [C implementation "be2json.c"](./contrib/be2json.c). You can execute it with the following:

```console
gcc ./contrib/be2json.c -o be2json
chmod +x ./be2json
echo "4:spam" | ./be2json
```

## TODO

- Return located errors with input and output positions and context.
- Counter for number of items in a list for debugging and errors.
- Use Property-Based Testing. Generate random valid bencoded values.
- Install tracing crate. Add verbose mode that enables debugging.
- Option to check if the final JSON it's valid at the end of the process.
- Benchmarking for this implementation anc the original C implementation.

## Alternatives

- <https://chocobo1.github.io/bencode_online/>
- <https://adrianstoll.com/post/bencoding/>
- <https://www.nayuki.io/page/bittorrent-bencode-format-tools>
- <https://gist.github.com/camilleoudot/840929699392b3d25afbec25d850c94a>
- <https://github.com/denis-selimovic/bencode>

Bencode online:

- <https://adrianstoll.com/post/bencoding/>
- <https://chocobo1.github.io/bencode_online/>

## Links

Bencode:

- <https://wiki.theory.org/BitTorrentSpecification#Bencoding>
- <https://en.wikipedia.org/wiki/Bencode>

## Credits

This implementation is basically a port to Rust from <https://gist.github.com/camilleoudot/840929699392b3d25afbec25d850c94a> with some changes like:

- It does not use magic numbers (explicit enum for states).
- It prints non UTF-8 string in hexadecimal.
