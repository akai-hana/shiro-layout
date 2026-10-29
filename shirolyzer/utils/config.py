"""Load, validate, and apply the user-facing YAML configuration."""
from __future__ import annotations

import json
import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import numpy as np
import yaml

from . import paths
from .ansi import PHYSICAL_LAYOUTS
from .fingers import FINGERS, FI, KEYS, KI

FINGER_RE = re.compile(r"^(?:[LR])?([PRMI])$")
PRIORITY_NAMES = (
    "sfbs-sfts", "scissors", "redirects", "rolls", "alternation",
    "finger_balance",
)
DEFAULT_PRIORITY = [
    ["sfbs-sfts"],
    ["scissors", "redirects"],
    ["rolls", "alternation"],
    ["finger_balance"],
]
DEFAULT_ALT_GATES = {
    ("X", "M"): ("only", {"C", "A"}, False),
    ("W", "M"): ("only", {"Q", "X", "S"}, False),
    ("L", "I"): ("only", {"O"}, False),
    ("J", "M"): ("block", {"T", "I", "G"}, True),
    ("M", "M"): ("block", {"J"}, False),
    ("C", "M"): ("block", {"R"}, False),
    ("G", "M"): ("never", None, False),
    ("V", "M"): ("block", {"G"}, False),
}


class ConfigError(ValueError):
    """Actionable configuration error."""


@dataclass
class Config:
    path: Path
    raw: dict
    general: dict
    model: dict
    fingermap: dict[str, str]
    finger_tokens: list[str]
    alt_fingers: list[dict]
    conditions: list[dict]
    priority_list: list[list[str]]
    finger_balance: dict[str, float]
    flat_share: dict[str, float]
    primary_corpus: list[dict]
    secondary_corpus: list[dict]

    @property
    def physical_layout(self) -> str:
        return self.general["physical_layout"]

    @property
    def default_layout(self) -> str:
        return self.general["default_layout"]

    @property
    def primary_sacrifice(self) -> float:
        return float(self.general["primary_sacrifice"])

    @property
    def model_variant(self) -> str:
        return str(self.model["variant"])

    @property
    def theta_path(self) -> Path:
        path = Path(self.model["theta_file"])
        return path if path.is_absolute() else paths.CONFIG_DIR / path

    @property
    def gates(self) -> dict:
        return {entry["key"] + entry["finger"][1]: (entry["mode"], set(entry.get("context") or []))
                for entry in self.alt_fingers if "key" in entry}

    @property
    def explicit_gates(self) -> set:
        return {entry["key"] + entry["finger"][1] for entry in self.alt_fingers
                if entry.get("explicit")}

    @property
    def conditions_are_default(self) -> bool:
        default = _default_data()
        return self.conditions == default.get("conditions")

    def priority_multipliers(self) -> dict[str, float]:
        if self.priority_list == DEFAULT_PRIORITY:
            return {name: 1.0 for name in PRIORITY_NAMES}
        scales = {name: 1.0 for name in PRIORITY_NAMES}
        for tier, names in enumerate(self.priority_list):
            scale = 1.0 + 0.5 * (len(self.priority_list) - tier - 1)
            for name in names:
                scales[name] = scale
        return scales

    def corpus_path(self, entry: dict) -> Path:
        value = entry.get("path") or entry.get("name")
        path = Path(value).expanduser()
        if path.is_file() or path.is_dir():
            return path
        candidate = paths.CORPUS_ROOT / str(value) / "parsed"
        if candidate.is_dir():
            return candidate
        return path


def _fail(path: Path, where: str, message: str):
    raise ConfigError(f"{path}: {where}: {message}")


def _as_mapping(path: Path, where: str, value: Any) -> dict:
    if not isinstance(value, dict):
        _fail(path, where, "expected a mapping")
    return value


def _number(path: Path, where: str, value: Any, *, minimum=None, maximum=None) -> float:
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        _fail(path, where, f"expected a number, got {value!r}")
    number = float(value)
    if minimum is not None and number < minimum:
        _fail(path, where, f"must be >= {minimum}, got {number}")
    if maximum is not None and number > maximum:
        _fail(path, where, f"must be <= {maximum}, got {number}")
    return number


def _token(path: Path, where: str, value: Any) -> tuple[str, str]:
    if not isinstance(value, str) or len(value) != 2:
        _fail(path, where, f"expected a two-character key/finger token, got {value!r}")
    key, finger = value[0], value[1]
    if key not in KI:
        _fail(path, where, f"unknown QWERTY key {key!r}")
    if finger not in FINGERS:
        _fail(path, where, f"unknown finger {finger!r}; expected one of {FINGERS}")
    return key, finger


def _finger_token(path: Path, where: str, value: Any) -> str:
    if not isinstance(value, str):
        _fail(path, where, f"expected a finger token, got {value!r}")
    match = FINGER_RE.match(value.upper())
    if not match:
        _fail(path, where, f"invalid finger token {value!r}; expected P/R/M/I or LP/LR/LM/LI/RP/RR/RM/RI")
    return match.group(1)


def _hand_finger(path: Path, where: str, key: str, value: Any) -> str:
    token = str(value).upper()
    match = re.match(r"^([LR]?)([PRMI])$", token)
    if not match:
        _fail(path, where, f"invalid finger {value!r}; expected P/R/M/I or LP/LR/LM/LI/RP/RR/RM/RI")
    hand, finger = match.group(1), match.group(2)
    if hand and hand != ("L" if key in "QWERTASDFGZXCVB" else "R"):
        _fail(path, where, f"finger {token!r} does not match the hand of key {key!r}")
    return finger


def _normalise_corpora(path: Path, where: str, value: Any) -> list[dict]:
    if isinstance(value, (str, dict)):
        value = [value]
    if not isinstance(value, list) or not value:
        _fail(path, where, "expected a corpus name/path or a non-empty list of them")
    out = []
    for i, item in enumerate(value):
        if isinstance(item, str):
            item = {"name": item, "weight": 1.0}
        elif isinstance(item, dict):
            item = dict(item)
        else:
            _fail(path, f"{where}[{i}]", "expected a name/path string or a mapping")
        if not item.get("name") and not item.get("path"):
            _fail(path, f"{where}[{i}]", "needs a name or path")
        item["weight"] = _number(path, f"{where}[{i}].weight", item.get("weight", 1.0), minimum=0.0)
        if item["weight"] <= 0:
            _fail(path, f"{where}[{i}].weight", "must be greater than zero")
        out.append(item)
    if not any(item["weight"] > 0 for item in out):
        _fail(path, where, "corpus weights must have a positive total")
    return out


def _parse_fingermap(path: Path, value: Any) -> tuple[dict[str, str], list[str]]:
    if isinstance(value, list):
        tokens = [str(x) for x in value]
    elif isinstance(value, str):
        tokens = value.split()
    else:
        _fail(path, "fingermap", "expected a multiline string or a list of tokens")
    if len(tokens) != len(KEYS):
        _fail(path, "fingermap", f"expected {len(KEYS)} tokens, got {len(tokens)}")
    mapping, order = {}, []
    for i, token in enumerate(tokens):
        key, finger = _token(path, f"fingermap[{i}]", token)
        if key != KEYS[i]:
            _fail(path, f"fingermap[{i}]", f"expected key {KEYS[i]!r} at this position, got {key!r}")
        mapping[key] = finger
        order.append(finger)
    return mapping, order


def _parse_alt_fingers(path: Path, value: Any) -> list[dict]:
    if not isinstance(value, list):
        _fail(path, "alt_fingers", "expected a list")
    out = []
    for i, item in enumerate(value):
        where = f"alt_fingers[{i}]"
        item = dict(_as_mapping(path, where, item))
        has_sequence = "sequence" in item
        has_key = "key" in item
        if has_sequence == has_key:
            _fail(path, where, "needs exactly one of 'sequence' (explicit override) or 'key' (context gate)")
        if has_key:
            key = str(item["key"]).upper()
            if key not in KI:
                _fail(path, f"{where}.key", f"unknown QWERTY key {key!r}")
            if "finger" not in item:
                _fail(path, where, "context gate needs a finger")
            finger = _hand_finger(path, f"{where}.finger", key, item["finger"])
            mode = str(item.get("mode", ""))
            if mode not in ("only", "block", "never"):
                _fail(path, f"{where}.mode", "expected only, block, or never")
            context = item.get("context") or []
            if not isinstance(context, list):
                _fail(path, f"{where}.context", "expected a list of QWERTY keys")
            for j, ctx in enumerate(context):
                if str(ctx).upper() not in KI:
                    _fail(path, f"{where}.context[{j}]", f"unknown QWERTY key {ctx!r}")
            if mode == "never" and context:
                _fail(path, f"{where}.context", "mode 'never' does not take a context")
            hand = "L" if key in "QWERTASDFGZXCVB" else "R"
            out.append({"key": key, "finger": hand + finger, "mode": mode,
                        "context": [str(x).upper() for x in context],
                        "explicit": bool(item.get("explicit", False))})
        else:
            sequence = str(item["sequence"]).upper()
            if not sequence or len(sequence) > 30:
                _fail(path, f"{where}.sequence", "expected 1-30 QWERTY position letters")
            for j, key in enumerate(sequence):
                if key not in KI:
                    _fail(path, f"{where}.sequence[{j}]", f"unknown QWERTY key {key!r}")
            fingers = item.get("fingers")
            if not isinstance(fingers, list) or len(fingers) != len(sequence):
                _fail(path, f"{where}.fingers", "expected one finger token per sequence position")
            parsed = []
            for j, (key, finger) in enumerate(zip(sequence, fingers)):
                _hand_finger(path, f"{where}.fingers[{j}]", key, finger)
                parsed.append(str(finger).upper())
            out.append({"sequence": sequence, "fingers": parsed})
    return out


def _parse_conditions(path: Path, value: Any) -> list[dict]:
    if not isinstance(value, list) or not value:
        _fail(path, "conditions", "expected a non-empty list")
    out = []
    for i, item in enumerate(value):
        where = f"conditions[{i}]"
        item = dict(_as_mapping(path, where, item))
        kind = str(item.get("type", ""))
        if kind not in ("scissors", "spread", "separation", "crossing"):
            _fail(path, f"{where}.type", "expected scissors, spread, separation, or crossing")
        if not isinstance(item.get("allow"), bool):
            _fail(path, f"{where}.allow", "expected true or false")
        if kind == "scissors":
            if item.get("distance") not in ("full", "half"):
                _fail(path, f"{where}.distance", "expected full or half")
            if "reverse" in item and not isinstance(item["reverse"], bool):
                _fail(path, f"{where}.reverse", "expected true or false")
            fingers = item.get("fingers") or []
            if not isinstance(fingers, list):
                _fail(path, f"{where}.fingers", "expected a list of hand/finger tokens")
            for j, token in enumerate(fingers):
                _finger_token(path, f"{where}.fingers[{j}]", token)
        elif kind == "spread":
            _number(path, f"{where}.max_extra", item.get("max_extra", 2.0), minimum=0.0)
        elif kind == "separation":
            _number(path, f"{where}.min_width", item.get("min_width", 1.9), minimum=0.0)
        else:
            _number(path, f"{where}.deficit", item.get("deficit", 0.30), minimum=0.0)
        out.append(item)
    return out


def _parse_priority(path: Path, value: Any) -> list[list[str]]:
    if not isinstance(value, list) or not value:
        _fail(path, "priority_list", "expected a non-empty list")
    out, seen = [], set()
    for i, item in enumerate(value):
        names = [item] if isinstance(item, str) else item
        if not isinstance(names, list) or not names:
            _fail(path, f"priority_list[{i}]", "expected a name or a non-empty list of names")
        tier = []
        for j, name in enumerate(names):
            if not isinstance(name, str) or name not in PRIORITY_NAMES:
                _fail(path, f"priority_list[{i}][{j}]",
                      f"unknown priority {name!r}; expected one of {PRIORITY_NAMES}")
            if name in seen:
                _fail(path, f"priority_list[{i}][{j}]", f"duplicate priority {name!r}")
            seen.add(name)
            tier.append(name)
        out.append(tier)
    missing = set(PRIORITY_NAMES) - seen
    if missing:
        _fail(path, "priority_list", f"missing priorities: {sorted(missing)}")
    return out


def _parse_balance(path: Path, value: Any) -> tuple[dict[str, float], dict[str, float]]:
    data = _as_mapping(path, "finger_balance", value)
    allowed = {h + f for h in "LR" for f in FINGERS} | set(FINGERS)
    vals = {}
    for key, raw in data.items():
        if key not in allowed:
            _fail(path, f"finger_balance.{key}", "expected LP/LR/LM/LI/RP/RR/RM/RI or P/R/M/I")
        vals[key] = _number(path, f"finger_balance.{key}", raw, minimum=0.0)
    if not any(v > 0 for v in vals.values()):
        _fail(path, "finger_balance", "at least one percentage must be positive")
    per_finger = {}
    for finger in FINGERS:
        readings = []
        if finger in vals:
            readings.append(vals[finger])
        for hand in "LR":
            if hand + finger in vals:
                readings.append(vals[hand + finger])
        if not readings:
            _fail(path, f"finger_balance.{finger}", "missing percentage for this finger")
        per_finger[finger] = sum(readings) / len(readings)
    total = sum(per_finger.values())
    flat = {f: per_finger[f] / total for f in FINGERS}
    return per_finger, flat


def _default_data() -> dict:
    return yaml.safe_load(paths.CONFIG_FILE.read_text(encoding="utf-8"))


@dataclass
class _State:
    active: Config | None = None


_STATE = _State()


def load(path=None) -> Config:
    path = Path(path).expanduser() if path is not None else paths.CONFIG_FILE
    try:
        raw = yaml.safe_load(path.read_text(encoding="utf-8"))
    except FileNotFoundError:
        raise ConfigError(f"configuration file not found: {path}") from None
    except yaml.YAMLError as exc:
        raise ConfigError(f"{path}: invalid YAML: {exc}") from None
    raw = _as_mapping(path, "root", raw)
    known = {"general", "model", "fingermap", "alt_fingers", "conditions",
             "priority_list", "finger_balance"}
    unknown = set(raw) - known
    if unknown:
        _fail(path, "root", f"unknown section(s): {sorted(unknown)}")
    for section in known:
        if section not in raw:
            _fail(path, "root", f"missing required section {section!r}")

    general = _as_mapping(path, "general", raw["general"])
    layout = str(general.get("physical_layout", ""))
    if layout not in PHYSICAL_LAYOUTS:
        _fail(path, "general.physical_layout",
              f"unknown physical layout {layout!r}; expected one of {sorted(PHYSICAL_LAYOUTS)}")
    default_layout = str(general.get("default_layout", ""))
    if len(default_layout) != 30:
        _fail(path, "general.default_layout", "expected a 30-character layout")
    primary = _normalise_corpora(path, "general.primary_corpus", general.get("primary_corpus"))
    secondary = _normalise_corpora(path, "general.secondary_corpus", general.get("secondary_corpus"))
    sacrifice = _number(path, "general.primary_sacrifice",
                        general.get("primary_sacrifice", 0.00023), minimum=0.0)

    model = _as_mapping(path, "model", raw["model"])
    if model.get("type") != "pattern":
        _fail(path, "model.type", "only the 'pattern' model is implemented")
    variant = str(model.get("variant", "A"))
    if variant not in ("A", "B"):
        _fail(path, "model.variant", "expected A or B")
    theta_file = str(model.get("theta_file", ""))
    theta_path = Path(theta_file)
    if not theta_path.is_absolute():
        theta_path = paths.CONFIG_DIR / theta_path
    if not theta_path.is_file():
        _fail(path, "model.theta_file", f"theta file not found: {theta_path}")
    theta = json.loads(theta_path.read_text(encoding="utf-8"))
    from . import comfort as cg
    missing = sorted(set(cg.PNAMES) - set(theta))
    if missing:
        _fail(path, "model.theta_file", f"{theta_path} is missing parameters: {missing}")

    fingermap, finger_tokens = _parse_fingermap(path, raw["fingermap"])
    alt_fingers = _parse_alt_fingers(path, raw["alt_fingers"])
    conditions = _parse_conditions(path, raw["conditions"])
    priority = _parse_priority(path, raw["priority_list"])
    balance, flat = _parse_balance(path, raw["finger_balance"])

    general = dict(general)
    general["primary_sacrifice"] = sacrifice
    cfg = Config(path=path, raw=raw, general=general,
                 model={"type": "pattern", "variant": variant, "theta_file": theta_file},
                 fingermap=fingermap, finger_tokens=finger_tokens,
                 alt_fingers=alt_fingers, conditions=conditions,
                 priority_list=priority, finger_balance=balance, flat_share=flat,
                 primary_corpus=primary, secondary_corpus=secondary)
    return cfg


def apply(cfg: Config | None = None) -> Config:
    cfg = cfg or load()
    from . import comfort as cg
    from . import fingers as FG
    from . import gates as GT
    from . import search as SH
    from . import scorer as SG

    FG.DEFAULT_FINGER.clear()
    FG.DEFAULT_FINGER.update(cfg.fingermap)
    GT.configure(cfg.alt_fingers)
    SH.FLAT_SHARE.clear()
    SH.FLAT_SHARE.update(cfg.flat_share)
    from . import conditions as CD
    CD.configure(cfg.conditions, enabled=not cfg.conditions_are_default)
    scales = cfg.priority_multipliers()
    cg.W_SFB_SCALE = scales["sfbs-sfts"]
    cg.W_SCISSOR_SCALE = scales["scissors"]
    cg.W_REDIRECT_SCALE = scales["redirects"]
    cg.W_ROLL_SCALE = scales["rolls"]
    cg.W_ALT_SCALE = scales["alternation"]
    SH.W_BALANCE = 0.4 * scales["finger_balance"]
    SH.W_HAND = 0.5 * scales["finger_balance"]
    SG.set_active_model(cfg.model_variant, cfg.theta_path)
    SG.reset_tables()
    _STATE.active = cfg
    return cfg


def ensure_applied() -> Config:
    if _STATE.active is None:
        return apply(load())
    return _STATE.active


def active() -> Config | None:
    return _STATE.active


def load_and_apply(path=None) -> Config:
    return apply(load(path))


def corpus_entries(cfg: Config, role: str) -> list[dict]:
    if role == "primary":
        return cfg.primary_corpus
    if role == "secondary":
        return cfg.secondary_corpus
    raise ValueError(role)
