"""Shirolyzer package root for module-mode tooling.

User-facing entry points remain directly executable under ``shirolyzer/`` and
import shared code through the ``utils`` package.  The package marker lets
non-runtime tools in ``shirolyzer/dev`` be invoked with
``python -m shirolyzer.dev.<tool>`` without path bootstrapping.
"""
