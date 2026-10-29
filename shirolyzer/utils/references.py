"""Load, validate, and append reference layouts from config/references.toml."""
import json
from pathlib import Path
import tomllib

from . import paths


def load_references(path=None):
    path = Path(path) if path is not None else paths.REFERENCES_FILE
    if not path.is_file():
        raise FileNotFoundError(f"reference file not found: {path}")
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    entries = data.get("reference", [])
    if not isinstance(entries, list):
        raise ValueError(f"{path}: 'reference' must be an array of tables")
    seen = set()
    refs = []
    for i, entry in enumerate(entries, 1):
        name = entry.get("name")
        layout = entry.get("layout")
        if not name or not layout:
            raise ValueError(f"{path}: reference #{i} needs both name and layout")
        letters = [ch for ch in layout if ch.isalpha()]
        if len(layout) != 30 or len(letters) != 26 or len(set(letters)) != 26:
            raise ValueError(
                f"{path}: reference {name!r} needs 30 keys and 26 distinct letters")
        if name in seen:
            raise ValueError(f"{path}: duplicate reference name {name!r}")
        seen.add(name)
        refs.append(dict(entry))
    return refs


def load_reference_map(path=None):
    return {entry["name"]: entry["layout"] for entry in load_references(path)}


def _toml_string(value):
    return json.dumps(str(value), ensure_ascii=False)


def _unique_name(name, existing):
    if name not in existing:
        return name
    i = 2
    while f"{name}-{i}" in existing:
        i += 1
    return f"{name}-{i}"


def append_reference(name, layout, primary_score=None, secondary_score=None,
                     seed=None, path=None):
    """Append one [[reference]] table without rewriting existing content.

    Returns the written name, or None when the layout already exists.
    """
    path = Path(path) if path is not None else paths.REFERENCES_FILE
    refs = load_references(path)
    if any(entry["layout"] == layout for entry in refs):
        return None
    name = _unique_name(name, {entry["name"] for entry in refs})
    lines = ["", "[[reference]]", f"name = {_toml_string(name)}",
             f"layout = {_toml_string(layout)}"]
    if primary_score is not None:
        lines.append(f"primary_score = {float(primary_score):.9f}")
    lines.append("secondary_score = " +
                 (f"{float(secondary_score):.9f}" if secondary_score is not None else "nan"))
    if seed is not None:
        lines.append(f"seed = {int(seed)}")
    text = path.read_text(encoding="utf-8")
    if text and not text.endswith("\n"):
        lines.insert(0, "")
    with path.open("a", encoding="utf-8") as out:
        out.write("\n".join(lines) + "\n")
    load_references(path)
    return name
