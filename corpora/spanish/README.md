# Ian Douglas' Spanish keyboard development corpus

Source: <https://zenodo.org/records/5501931>
Direct Download Link: <https://zenodo.org/records/5501931/files/Spanish-Keyboard-Corpus-1.0.0.zip?download=1>

---

## About

Corpus somewhat equivalent to Shai, but for Spanish.

## Parser

The parser extracts `clean-spanish-book-extracts.txt` from the archive.
This is the cleaned book corpus that matches the scale of the previous local Gutenberg sample.

<!-- TODO: make the parser auto-pull the direct download link, parse it, then erase the main source to save on space, leaving the lightweight .json data behind, and getting rid of the now useless source file. -->

## Build

```sh
python3 corpora/spanish/parse_spanish.py            # download, parse, write parsed/
python3 corpora/spanish/parse_spanish.py --check    # download, parse, compare only
python3 corpora/spanish/parse_spanish.py --source FILE.zip [--keep-source]
```

The parser downloads the zip to a temporary directory, extracts only the
selected text file, and deletes the archive and extracted files on success.
`--source` uses an existing local zip instead, and `--keep-source` keeps the
working files for debugging.

Output: `corpora/spanish/parsed/letter_freq.json`,
`corpora/spanish/parsed/bigram_freq.json`, and
`corpora/spanish/parsed/trigram_freq.json`.

## Totals

```text
Letters:  8,584,068
Spaces:   1,860,626
Bigrams:  6,723,441
Trigrams: 4,983,706
```

Processing: lowercase, Unicode NFKD with combining marks stripped, then
maximal `[a-z]+` runs; non-letters break runs.
