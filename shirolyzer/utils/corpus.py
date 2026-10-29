"""N-gram corpus loading for the pattern scorer.

``Corpus.load`` accepts a parsed corpus directory, a corpus directory that
contains ``parsed/``, or a raw UTF-8 text source (optionally ``.xz`` or a zip
containing a text file).  Raw sources are normalised with NFKD, lowercased,
and split into maximal ``[a-z]+`` runs, matching the corpus parsers.
"""
import json
import lzma
import re
import unicodedata
import zipfile
from collections import Counter
from pathlib import Path

import numpy as np

SPACE = "@space"
PARSED_FILES = ("letter_freq.json", "bigram_freq.json", "trigram_freq.json")
RUN = re.compile(r"[a-z]+")


def _parsed_dir(path):
    path = Path(path).expanduser()
    if path.is_file():
        if path.name in PARSED_FILES:
            return path.parent
        return None
    if path.is_dir():
        if all((path / name).is_file() for name in PARSED_FILES):
            return path
        parsed = path / "parsed"
        if all((parsed / name).is_file() for name in PARSED_FILES):
            return parsed
    return None


def _count_text(text):
    letters, bigrams, trigrams = Counter(), Counter(), Counter()
    for match in RUN.finditer(text):
        run = match.group()
        for ch in run:
            letters[ch] += 1
        for i in range(len(run) - 1):
            bigrams[run[i:i + 2]] += 1
        for i in range(len(run) - 2):
            trigrams[run[i:i + 3]] += 1
    return letters, bigrams, trigrams


def _read_source(path):
    path = Path(path)
    suffix = path.suffix.lower()
    if suffix == ".xz":
        with lzma.open(path, "rt", encoding="utf-8", errors="ignore") as src:
            return src.read()
    if suffix == ".zip":
        with zipfile.ZipFile(path) as archive:
            names = [n for n in archive.namelist() if n.lower().endswith(".txt")]
            if not names:
                raise FileNotFoundError(f"no .txt member in {path}")
            with archive.open(names[0]) as src:
                return src.read().decode("utf-8", errors="ignore")
    return path.read_text(encoding="utf-8", errors="ignore")


def _source_file(path):
    path = Path(path).expanduser()
    if path.is_file():
        return path
    if path.is_dir():
        candidates = [p for p in path.iterdir()
                      if p.is_file() and p.suffix.lower() in (".txt", ".xz", ".zip")]
        if len(candidates) == 1:
            return candidates[0]
    return None


class Corpus:
    """N-gram counts over the keystroke stream (a-z + space)."""

    def __init__(self, letter_freq, bigram_freq, trigram_freq):
        self.L, self.B, self.T = letter_freq, bigram_freq, trigram_freq
        self.chars = [chr(c) for c in range(ord("a"), ord("z") + 1)] + [SPACE]
        self.cid = {c: i for i, c in enumerate(self.chars)}
        self.total_letters = sum(self.L.values())
        self.total_bigrams = sum(self.B.values())
        self.total_trigrams = sum(self.T.values())
        bi, bc, tri, tc = [], [], [], []
        for bg, n in self.B.items():
            if len(bg) == 2 and bg[0] in self.cid and bg[1] in self.cid:
                bi.append((self.cid[bg[0]], self.cid[bg[1]])); bc.append(n)
        for tg, n in self.T.items():
            if len(tg) == 3 and all(c in self.cid for c in tg):
                tri.append(tuple(self.cid[c] for c in tg)); tc.append(n)
        self.bi_ids = np.array(bi, dtype=np.int32)
        self.bi_counts = np.array(bc, dtype=np.float64)
        self.tri_ids = np.array(tri, dtype=np.int32)
        self.tri_counts = np.array(tc, dtype=np.float64)

    @classmethod
    def load(cls, path):
        parsed = _parsed_dir(path)
        if parsed is not None:
            return cls(*(json.load(open(parsed / name)) for name in PARSED_FILES))
        source = _source_file(path)
        if source is not None:
            text = unicodedata.normalize("NFKD", _read_source(source).lower())
            text = "".join(ch for ch in text if not unicodedata.combining(ch))
            L, B, T = _count_text(text)
            return cls(L, B, T)
        raise FileNotFoundError(
            f"cannot resolve corpus {path!r}: expected a parsed directory, a corpus "
            "directory containing parsed/, or a raw .txt/.xz/.zip source")

    def scaled(self, factor):
        factor = float(factor)
        if factor == 1.0:
            return self
        return Corpus({k: v * factor for k, v in self.L.items()},
                      {k: v * factor for k, v in self.B.items()},
                      {k: v * factor for k, v in self.T.items()})

    @classmethod
    def merge(cls, corpora):
        corpora = list(corpora)
        if not corpora:
            raise ValueError("no corpora to merge")
        if len(corpora) == 1:
            return corpora[0]
        L, B, T = Counter(), Counter(), Counter()
        for corpus in corpora:
            L.update(corpus.L); B.update(corpus.B); T.update(corpus.T)
        return cls(L, B, T)


def load_many(paths):
    return Corpus.merge(Corpus.load(path) for path in paths)
