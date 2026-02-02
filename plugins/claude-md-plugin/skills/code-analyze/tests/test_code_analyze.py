"""Pytest-bdd test runner for code_analyze.feature."""

import pytest
from pytest_bdd import scenarios

# Load all scenarios from the feature file
scenarios("code_analyze.feature")
