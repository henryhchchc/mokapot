from pathlib import Path
import shutil
import subprocess
import tempfile

import pytest


@pytest.fixture(scope="session")
def java_classes_dir() -> Path:
    javac = shutil.which("javac")
    if javac is None:
        pytest.skip("javac is required for Java fixture compilation")

    repo_root = Path(__file__).resolve().parents[3]
    source_root = repo_root / "crates" / "mokapot" / "test_data" / "mokapot"
    java_sources = sorted(source_root.rglob("*.java"))

    if not java_sources:
        pytest.fail(f"No Java test sources found under {source_root}")

    tmp_dir = Path(tempfile.mkdtemp(prefix="mokapot_py_java_"))
    output_dir = tmp_dir / "java_classes"

    subprocess.run(
        [
            javac,
            "-g",
            "-d",
            str(output_dir),
            *[str(path) for path in java_sources],
        ],
        cwd=source_root,
        check=True,
    )

    return output_dir
