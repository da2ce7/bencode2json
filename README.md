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

To run the binary:

### With input and output file

```console
cargo run -- -i tests/sample.bencode -o output.json
```

### With stdin and stdout

```console
echo "4:spam" | cargo run
```

## Test

To run unit and integration tests:

```console
cargo test
```

## TODO

- Counter for number of items in a list for debugging and errors.
- Return errors with position.
- Use Property-Based Testing. Generate random valid bencoded values.
- Refactor: Use only one tests with a data provider containing all cases.

## Alternatives

- <https://chocobo1.github.io/bencode_online/>
- <https://adrianstoll.com/post/bencoding/>
- <https://www.nayuki.io/page/bittorrent-bencode-format-tools>
- <https://gist.github.com/camilleoudot/840929699392b3d25afbec25d850c94a>
- <https://github.com/denis-selimovic/bencode>

Bencode online:

- <https://adrianstoll.com/post/bencoding/>
- <https://chocobo1.github.io/bencode_online/>

## Credits

THis implementation is basically a port to Rust from <https://gist.github.com/camilleoudot/840929699392b3d25afbec25d850c94a> with some changes like:

- It does not use magic numbers (explicit enum for states).
- It prints non UTF-8 string in hexadecimal.
