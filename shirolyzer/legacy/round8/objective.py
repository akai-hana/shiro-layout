#!/usr/bin/env python3
"""Calibrated objective: English Shai primary, Spanish near-tie secondary.

`--primary-sacrifice X` (see anneal_shai.py) means at most X primary-score
regression is treated as a near-tie; within that band Spanish breaks ties.
The same value is used as the composite weight: one full secondary-score
unit is worth X primary-score units.
"""
from collections import Counter

from scorer import Corpus

ROOT = "/home/akai/git/layout-opencode-investigation/"
PRIMARY_PATH = ROOT + "corpus/shai/"
SECONDARY_PATH = ROOT + "corpus/spanish/"
DEFAULT_PRIMARY_SACRIFICE = 1e-5
SECONDARY_WEIGHT = DEFAULT_PRIMARY_SACRIFICE


def _add(dst, src, weight):
    for k, v in src.items():
        dst[k] += v * weight


def _blend(parts):
    L, B, T = Counter(), Counter(), Counter()
    for corpus, weight in parts:
        _add(L, corpus.L, weight)
        _add(B, corpus.B, weight)
        _add(T, corpus.T, weight)
    return L, B, T


def load_primary(path=PRIMARY_PATH):
    return Corpus.load(path)


def load_secondary(path=SECONDARY_PATH):
    return Corpus.load(path)


def build_composite(primary=None, secondary=None,
                    sacrifice=DEFAULT_PRIMARY_SACRIFICE):
    primary = primary or load_primary()
    secondary = secondary or load_secondary()
    p_total = primary.total_trigrams
    s_total = secondary.total_trigrams
    scale = sacrifice * p_total / max(s_total, 1.0)
    L, B, T = _blend([(primary, 1.0), (secondary, scale)])
    return Corpus(L, B, T), primary, secondary, scale


def build_evaluators(dead_keys, sacrifice=DEFAULT_PRIMARY_SACRIFICE,
                     primary=None, secondary=None):
    from search2 import Evaluator
    composite, primary_c, secondary_c, scale = build_composite(
        primary=primary, secondary=secondary, sacrifice=sacrifice)
    return Evaluator(composite, dead_keys), Evaluator(primary_c, dead_keys), scale


def build_evaluators_full(dead_keys, sacrifice=DEFAULT_PRIMARY_SACRIFICE,
                          primary=None, secondary=None):
    from search2 import Evaluator
    composite, primary_c, secondary_c, scale = build_composite(
        primary=primary, secondary=secondary, sacrifice=sacrifice)
    return (Evaluator(composite, dead_keys), Evaluator(primary_c, dead_keys),
            Evaluator(secondary_c, dead_keys), scale)


def within_primary_budget(primary_score, best_primary, sacrifice):
    return primary_score <= best_primary + sacrifice + 1e-12
