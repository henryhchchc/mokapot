fn main() -> pyo3_stub_gen::Result<()> {
    let stub = mokapot_py::stub_info()?;
    stub.generate()?;
    Ok(())
}
