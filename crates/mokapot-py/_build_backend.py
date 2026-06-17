"""
PEP 517 build backend wrapper that generates mokapot.pyi before delegating to maturin.
"""

import subprocess
import sys

import maturin

# Pass-through hooks that need no stub generation
get_requires_for_build_wheel = maturin.get_requires_for_build_wheel
get_requires_for_build_sdist = maturin.get_requires_for_build_sdist
get_requires_for_build_editable = maturin.get_requires_for_build_editable
prepare_metadata_for_build_wheel = maturin.prepare_metadata_for_build_wheel
prepare_metadata_for_build_editable = maturin.prepare_metadata_for_build_editable
build_sdist = maturin.build_sdist


def _generate_stubs() -> None:
    subprocess.run(
        ["cargo", "run", "--bin", "stub_gen"],
        check=True,
        stdout=sys.stdout,
        stderr=sys.stderr,
    )


def build_wheel(wheel_directory, config_settings=None, metadata_directory=None):
    _generate_stubs()
    return maturin.build_wheel(wheel_directory, config_settings, metadata_directory)


def build_editable(wheel_directory, config_settings=None, metadata_directory=None):
    _generate_stubs()
    return maturin.build_editable(wheel_directory, config_settings, metadata_directory)
