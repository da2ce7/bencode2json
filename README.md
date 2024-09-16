# Torrust Bencode2Json

A lib and console command to convert from bencoded data to JSON format.

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
