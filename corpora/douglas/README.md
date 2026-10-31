# Ian Douglas' Spanish keyboard development corpus

Source: <https://zenodo.org/records/5501931>
Direct Download Link: <https://zenodo.org/records/5501931/files/Spanish-Keyboard-Corpus-1.0.0.zip?download=1>

---

## About

Corpus somewhat equivalent to Shai, but for Spanish.

## Parser

The parser extracts `clean-spanish-book-extracts.txt` from the archive.
This is the cleaned book corpus that matches the scale of the previous local Gutenberg sample.

## Build

```sh
python3 corpora/douglas/parse_douglas.py            # download, parse, write parsed/
python3 corpora/douglas/parse_douglas.py --check    # download, parse, compare only
python3 corpora/douglas/parse_douglas.py --source FILE.zip [--keep-source]
```

The parser downloads the zip to a temporary directory, extracts only the
selected text file, and deletes the archive and extracted files on success.
`--source` uses an existing local zip instead, and `--keep-source` keeps the
working files for debugging.

Output: `corpora/douglas/parsed/letter_freq.json`,
`corpora/douglas/parsed/bigram_freq.json`, and
`corpora/douglas/parsed/trigram_freq.json`.  Entries are ordered from most to
least frequent (ties by key), and the parser reports which files it changed
or that the parsed data was already unchanged.

## Totals

```text
Letters:  8,584,068
Spaces:   1,860,626
Bigrams:  6,723,441
Trigrams: 4,983,706
```

Processing: lowercase, Unicode NFKD with combining marks stripped, then
maximal `[a-z]+` runs; non-letters break runs.
