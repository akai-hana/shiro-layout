"""Load and validate reference layouts from config/references.toml."""
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
