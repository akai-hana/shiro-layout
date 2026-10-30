"""Repository-relative path helpers for the retained development tools.

Paths are resolved from this module so the corpus parsers and model-refitting
tools behave the same regardless of the current working directory. The shipped
runtime is the Rust binary and does not use this module.
"""
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
SHIROLYZER_ROOT = REPO_ROOT / "shirolyzer"
UTILS_DIR = SHIROLYZER_ROOT / "utils"
CORPUS_ROOT = REPO_ROOT / "corpora"
CONFIG_DIR = SHIROLYZER_ROOT / "config"
CONFIG_FILE = CONFIG_DIR / "config.yaml"
MODEL_DIR = CONFIG_DIR / "model"
RESULTS_DIR = SHIROLYZER_ROOT / "results"
XKB_DIR = REPO_ROOT / "xkb"
REFERENCES_FILE = CONFIG_DIR / "references.toml"


def repo_root():
    return REPO_ROOT


def corpus_root():
    return CORPUS_ROOT


def config_dir():
    return CONFIG_DIR


def model_dir():
    return MODEL_DIR


def results_dir():
    return RESULTS_DIR


def _slug(value):
    text = "".join(ch if ch.isalnum() else "_" for ch in str(value)).strip("_")
    return text or "layout"


def result_path(command, name=None, suffix=".txt"):
    """Return a default output path derived from the command and optional name."""
    stem = command if name is None else f"{command}_{_slug(name)}"
    return RESULTS_DIR / f"{stem}{suffix}"
