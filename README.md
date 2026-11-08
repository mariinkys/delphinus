<p align="center">
 <a href="https://delphinus.mariinkys.dev" target="_blank"><img alt="Delphinus" src="https://github.com/mariinkys/delphinus/blob/main/assets/delphinus_banner.png?raw=true"></a>
</p>

<p align="center">
 Turn a list of Chinese or Japanese words into flashcards for Anki, Quizlet or Vaia.
 Written in Rust with <a href="https://leptos.dev/">Leptos</a>
</p>

## Running

```sh
# Dictionaries
curl -L http://ftp.edrdg.org/pub/Nihongo/JMdict_e.gz | gunzip > dictionaries/JMdict_e
curl -L https://www.mdbg.net/chinese/export/cedict/cedict_1_0_ts_utf-8_mdbg.txt.gz | gunzip > dictionaries/cedict_ts.u8

# Build Tools
rustup target add wasm32-unknown-unknown
cargo install cargo-leptos

# Run
cargo leptos watch                               # development, no OCR
LEPTOS_HASH_FILES=true cargo leptos build --release --bin-features ocr-bundled   # with OCR
```

### Docker

```sh
docker build -t delphinus .
docker run -p 8080:8080 delphinus
```

The image downloads the latest JMdict_e, CC-CEDICT and OCR models at build time.
Build with `--build-arg FEATURES=ssr` for an image without OCR.

### Configuration

| Variable | Default | |
|---|---|---|
| `DELPHINUS_JMDICT` | `dictionaries/JMdict_e` | Path to the JMdict XML (uncompressed) |
| `DELPHINUS_CEDICT` | `dictionaries/cedict_ts.u8` | Path to CC-CEDICT |
| `DELPHINUS_THREADS` | `2` | Worker threads |
| `DELPHINUS_OCR_DIR` | `ocr_models` | Folder with the three OCR model files |
| `DELPHINUS_OCR_IDLE_SECS` | `300` | Unload OCR models after this long unused |
| `DISABLE_OCR` | `false` | Turn image scanning off |
| `MALLOC_ARENA_MAX` | (set to `2` in Docker) | Fewer glibc arenas, less retained memory |

Plus the usual `LEPTOS_*` variables (`LEPTOS_SITE_ADDR`, `LEPTOS_SITE_ROOT`, …).

A missing dictionary disables that language

## Development Notes

* **Memory-mapped, indexed once.** At start-up each file is mmaped and scanned once for the
  byte offset of every headword (kanji forms and readings for JMdict, traditional and simplified
  for CC-CEDICT). The offsets are sorted by the text they point at, so a lookup is a binary search
  over the map, and only matching entries are parsed. The index costs 4 bytes per headword
  (≈ 2 MB for JMdict, ≈ 1 MB for CC-CEDICT); the dictionaries themselves stay in the kernel page
  cache, outside the process heap. After indexing, the scanned pages are released from the
  process (`MADV_DONTNEED`) and read-ahead is switched off (`MADV_RANDOM`).
* **OCR is lazy.** The models load on the first scan, one scan runs at a time, and they are
  unloaded after `DELPHINUS_OCR_IDLE_SECS` without use. Images are scaled down in the browser
  before upload.


## Thanks

You can help this and other projects: [Buy me a coffee](https://www.buymeacoffee.com/mariinkys)

## Acknowledgements

This site uses the [JMdict/EDICT](https://www.edrdg.org/wiki/index.php/JMdict-EDICT_Dictionary_Project)
dictionary files. These files are the property of the
[Electronic Dictionary Research and Development Group](https://www.edrdg.org/), and are used in
conformance with the Group's [licence](https://www.edrdg.org/edrdg/licence.html).

This site uses the [CC-CEDICT](https://cc-cedict.org/wiki/) dictionary, licensed under
Creative Commons Attribution-ShareAlike 4.0.

OCR uses the [oar-ocr](https://github.com/GreatV/oar-ocr) crate and its PP-OCRv5 models
(Apache-2.0); see [ocr_models/README.md](ocr_models/README.md).
