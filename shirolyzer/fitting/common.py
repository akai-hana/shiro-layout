"""Shared paths for the model-refitting tools."""
import os
from pathlib import Path

from ..utils import paths

MODEL_DIR = paths.MODEL_DIR
WORK_DIR = Path(os.environ.get("SHIRO_FITTING_DIR", paths.RESULTS_DIR / "fitting"))


def work_dir():
    WORK_DIR.mkdir(parents=True, exist_ok=True)
    return WORK_DIR


def input_theta(name):
    work = WORK_DIR / name
    return work if work.exists() else MODEL_DIR / name


def output_theta(name):
    return work_dir() / name
