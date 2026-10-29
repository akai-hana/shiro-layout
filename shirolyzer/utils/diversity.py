"""Cluster compute results by score tolerance and layout distance."""
import math


def layout_distance(a, b):
    """Number of key positions whose characters differ."""
    if len(a) != len(b):
        raise ValueError("layouts must have the same length")
    return sum(x != y for x, y in zip(a, b))


def merge_results(results):
    """Collapse duplicate layouts, keeping the best primary and secondary."""
    by_layout = {}
    for row in results:
        layout = row["layout"]
        cur = by_layout.get(layout)
        if cur is None:
            cur = {"start": row["start"], "layout": layout,
                   "primary": float(row["primary"]),
                   "secondary": row.get("secondary")}
            by_layout[layout] = cur
            continue
        if row["primary"] < cur["primary"]:
            cur["primary"] = float(row["primary"])
            cur["start"] = row["start"]
        sec = row.get("secondary")
        if sec is not None and (cur["secondary"] is None or sec < cur["secondary"]):
            cur["secondary"] = float(sec)
    for row in by_layout.values():
        row["final"] = (row["secondary"] if row["secondary"] is not None
                        else row["primary"])
    return list(by_layout.values())


def comparable(row, reference, primary_tolerance, secondary_tolerance):
    """Whether a row is no worse than the reference by more than tolerance.

    A better primary or secondary score never disqualifies a row; only a
    materially worse score does.
    """
    if row["primary"] - reference["primary"] > primary_tolerance:
        return False
    if reference.get("secondary") is not None:
        if row.get("secondary") is None:
            return False
        if row["secondary"] - reference["secondary"] > secondary_tolerance:
            return False
    return True


def select_valid_layouts(results, reference, primary_tolerance=0.002,
                         secondary_tolerance=0.005, min_distance=4,
                         max_valid=8):
    """Return distinct comparable layouts plus a truncation flag.

    The reference layout is always included first.  Other layouts must be
    within both tolerances and at least ``min_distance`` key positions away
    from every layout already selected.  ``truncated`` is true when comparable
    layouts were dropped only because ``max_valid`` was reached.
    """
    rows = merge_results(results)
    ref = next((row for row in rows if row["layout"] == reference["layout"]), None)
    if ref is None:
        ref = {"start": reference.get("start", "reference"),
               "layout": reference["layout"],
               "primary": float(reference["primary"]),
               "secondary": reference.get("secondary")}
        ref["final"] = (ref["secondary"] if ref["secondary"] is not None
                        else ref["primary"])
    rest = [row for row in rows if row is not ref]
    rest.sort(key=lambda row: (row["primary"],
                               row["secondary"] if row["secondary"] is not None
                               else math.inf))
    selected = []
    truncated = False
    for row in [ref] + rest:
        if row is not ref and not comparable(
                row, ref, primary_tolerance, secondary_tolerance):
            continue
        if any(layout_distance(row["layout"], kept["layout"]) < min_distance
               for kept in selected):
            continue
        if max_valid and len(selected) >= max_valid:
            truncated = True
            break
        selected.append(row)
    return selected, truncated


def valid_payload(rows):
    """JSON-ready rows for the valid-layout list."""
    return [{"name": row.get("reference_name"), "start": row.get("start"),
             "layout": row["layout"], "primary": row["primary"],
             "secondary": row.get("secondary")} for row in rows]


def format_valid_layouts(rows, status, truncated=False):
    """Render the valid-layout summary block used by compute.py."""
    lines = [f"valid_layouts={len(rows)} status={status}"]
    for i, row in enumerate(rows, 1):
        sec = row.get("secondary")
        sec_text = f"{sec:.9f}" if sec is not None else "nan"
        lines.append(f"  valid{i}: primary={row['primary']:.9f} "
                     f"secondary={sec_text} {row['layout']}")
    if truncated:
        lines.append("  additional comparable layouts omitted")
    return lines
