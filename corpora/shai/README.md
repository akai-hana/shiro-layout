# Shai's Cleaned iWeb corpus (90m words)

Source: [AKL (Alt Keyboard Layouts) Discord server](https://discord.gg/kXyGkVaez)
Direct Download Link: <https://colemak.com/pub/corpus/iweb-corpus-samples-cleaned.txt.xz>

## Build

```sh
python3 corpora/shai/parse_shai.py            # download, parse, write parsed/
python3 corpora/shai/parse_shai.py --check    # download, parse, compare only
python3 corpora/shai/parse_shai.py --source FILE.xz [--keep-source]
```

The parser downloads the `.xz` source to a temporary file, streams the
decompression, and deletes the downloaded file on success. `--source` uses an
existing local `.txt` or `.txt.xz` file instead, and `--keep-source` keeps a
downloaded archive for debugging.

Output: `corpora/shai/parsed/letter_freq.json`,
`corpora/shai/parsed/bigram_freq.json`, and
`corpora/shai/parsed/trigram_freq.json`.  Entries are ordered from most to
least frequent (ties by key), and the parser reports which files it changed
or that the parsed data was already unchanged.

## Totals

```text
Letters:  418,381,828
Bigrams:  327,253,737
Trigrams: 240,524,895
Runs:      91,128,091
```

Processing: lowercase, maximal `[a-z]+` runs; non-letters break runs.
