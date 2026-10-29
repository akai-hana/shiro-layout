"""Run a shirolyzer command.

Usage:
    python3 -m shirolyzer <command> [args...]

Commands:
    compute          Search for a layout using the current configuration
    analyze [LAYOUT] Score one layout (defaults to general.current_layout)
    render LAYOUT    Print a layout as a finger-annotated ASCII grid
    xkb              Convert a layout to an XKB symbols file
    compare          Rank the layouts in a reference file
    audit            Run questionnaire probes and model rulings
    check-baseline   Verify references and the shipped-model baseline
"""
import importlib
import runpy
import sys

COMMANDS = {
    "analyze": "shirolyzer.commands.analyze",
    "compute": "shirolyzer.commands.compute",
    "render": "shirolyzer.commands.render",
    "xkb": "shirolyzer.commands.xkb",
    "compare": "shirolyzer.commands.compare",
    "audit": "shirolyzer.commands.audit",
    "check-baseline": "shirolyzer.commands.check_baseline",
    "check_baseline": "shirolyzer.commands.check_baseline",
}


def main(argv=None):
    argv = list(sys.argv[1:] if argv is None else argv)
    if not argv or argv[0] in ("-h", "--help"):
        print(__doc__.strip())
        return 0
    name = argv.pop(0)
    module_name = COMMANDS.get(name)
    if module_name is None:
        print(f"unknown command: {name}\n", file=sys.stderr)
        print(__doc__.strip(), file=sys.stderr)
        return 2
    if name == "audit":
        sys.argv = [f"python -m {module_name}", *argv]
        runpy.run_module(module_name, run_name="__main__")
        return 0
    module = importlib.import_module(module_name)
    result = module.main(argv)
    return result if isinstance(result, int) else 0


if __name__ == "__main__":
    raise SystemExit(main())
