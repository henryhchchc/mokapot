#![allow(unsafe_op_in_unsafe_fn)]
#![allow(clippy::useless_conversion)]

use std::{
    fs::File,
    io::{BufReader, Cursor},
};

use mokapot::jvm::{self, Class as RustClass};
use pyo3::{
    create_exception,
    exceptions::{PyException, PyOSError, PyValueError},
    prelude::*,
    types::{PyModule, PyType},
};
use pyo3_stub_gen::derive::{gen_stub_pyclass, gen_stub_pymethods};

create_exception!(mokapot, MokapotError, PyException);
create_exception!(mokapot, ParseError, MokapotError);

#[gen_stub_pyclass]
#[derive(Clone, Debug)]
#[pyclass(name = "ClassRef", module = "mokapot", skip_from_py_object)]
struct PyClassRef(jvm::references::ClassRef);

#[gen_stub_pymethods]
#[pymethods]
impl PyClassRef {
    #[new]
    fn new(binary_name: String) -> Self {
        Self(jvm::references::ClassRef::new(binary_name))
    }

    #[getter]
    fn binary_name(&self) -> String {
        self.0.binary_name.clone()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

impl From<jvm::references::ClassRef> for PyClassRef {
    fn from(inner: jvm::references::ClassRef) -> Self {
        Self(inner)
    }
}

#[gen_stub_pyclass]
#[derive(Clone, Debug)]
#[pyclass(name = "ClassAccessFlags", module = "mokapot", skip_from_py_object)]
struct PyClassAccessFlags(jvm::class::AccessFlags);

#[gen_stub_pymethods]
#[pymethods]
#[allow(non_snake_case)]
impl PyClassAccessFlags {
    #[new]
    fn new(bits: u16) -> PyResult<Self> {
        let inner = jvm::class::AccessFlags::from_bits(bits)
            .ok_or_else(|| PyValueError::new_err("invalid class access flags bits"))?;
        Ok(Self(inner))
    }

    #[classmethod]
    fn from_bits(_cls: &Bound<'_, PyType>, bits: u16) -> PyResult<Self> {
        Self::new(bits)
    }

    #[getter]
    fn bits(&self) -> u16 {
        self.0.bits()
    }

    fn contains(&self, other: &Self) -> bool {
        self.0.contains(other.0)
    }

    fn __int__(&self) -> u16 {
        self.0.bits()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }

    #[classattr]
    fn PUBLIC() -> PyClassAccessFlags {
        PyClassAccessFlags(jvm::class::AccessFlags::PUBLIC)
    }

    #[classattr]
    fn FINAL() -> PyClassAccessFlags {
        PyClassAccessFlags(jvm::class::AccessFlags::FINAL)
    }

    #[classattr]
    fn SUPER() -> PyClassAccessFlags {
        PyClassAccessFlags(jvm::class::AccessFlags::SUPER)
    }

    #[classattr]
    fn INTERFACE() -> PyClassAccessFlags {
        PyClassAccessFlags(jvm::class::AccessFlags::INTERFACE)
    }

    #[classattr]
    fn ABSTRACT() -> PyClassAccessFlags {
        PyClassAccessFlags(jvm::class::AccessFlags::ABSTRACT)
    }

    #[classattr]
    fn SYNTHETIC() -> PyClassAccessFlags {
        PyClassAccessFlags(jvm::class::AccessFlags::SYNTHETIC)
    }

    #[classattr]
    fn ANNOTATION() -> PyClassAccessFlags {
        PyClassAccessFlags(jvm::class::AccessFlags::ANNOTATION)
    }

    #[classattr]
    fn ENUM() -> PyClassAccessFlags {
        PyClassAccessFlags(jvm::class::AccessFlags::ENUM)
    }

    #[classattr]
    fn MODULE() -> PyClassAccessFlags {
        PyClassAccessFlags(jvm::class::AccessFlags::MODULE)
    }
}

impl From<jvm::class::AccessFlags> for PyClassAccessFlags {
    fn from(inner: jvm::class::AccessFlags) -> Self {
        Self(inner)
    }
}

#[gen_stub_pyclass]
#[derive(Clone, Debug)]
#[pyclass(name = "Version", module = "mokapot", skip_from_py_object)]
struct PyVersion(jvm::class::Version);

#[gen_stub_pymethods]
#[pymethods]
impl PyVersion {
    #[getter]
    fn major(&self) -> u16 {
        self.0.major()
    }

    #[getter]
    fn minor(&self) -> u16 {
        self.0.minor()
    }

    #[getter]
    fn is_preview_enabled(&self) -> bool {
        self.0.is_preview_enabled()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

impl From<jvm::class::Version> for PyVersion {
    fn from(inner: jvm::class::Version) -> Self {
        Self(inner)
    }
}

#[gen_stub_pyclass]
#[derive(Clone, Debug)]
#[pyclass(name = "Class", module = "mokapot", skip_from_py_object)]
struct PyClass(RustClass);

#[gen_stub_pymethods]
#[pymethods]
impl PyClass {
    #[classmethod]
    fn from_bytes(_cls: &Bound<'_, PyType>, data: Vec<u8>) -> PyResult<Self> {
        let mut cursor = Cursor::new(data);
        let inner = RustClass::from_reader(&mut cursor).map_err(map_parse_error)?;
        Ok(Self(inner))
    }

    #[classmethod]
    fn from_file(_cls: &Bound<'_, PyType>, path: &str) -> PyResult<Self> {
        let file = File::open(path).map_err(|e| PyOSError::new_err(e.to_string()))?;
        let mut reader = BufReader::new(file);
        let inner = RustClass::from_reader(&mut reader).map_err(map_parse_error)?;
        Ok(Self(inner))
    }

    #[getter]
    fn version(&self) -> PyVersion {
        self.0.version.into()
    }

    #[getter]
    fn access_flags(&self) -> PyClassAccessFlags {
        self.0.access_flags.into()
    }

    #[getter]
    fn binary_name(&self) -> String {
        self.0.binary_name.clone()
    }

    #[getter]
    fn super_class(&self) -> Option<PyClassRef> {
        self.0.super_class.clone().map(Into::into)
    }

    #[getter]
    fn interfaces(&self) -> Vec<PyClassRef> {
        self.0.interfaces.iter().cloned().map(Into::into).collect()
    }

    fn is_interface(&self) -> bool {
        self.0.is_interface()
    }

    fn is_abstract(&self) -> bool {
        self.0.is_abstract()
    }

    fn __repr__(&self) -> String {
        format!("{:?}", self.0)
    }
}

fn map_parse_error(err: jvm::bytecode::ParseError) -> PyErr {
    ParseError::new_err(err.to_string())
}

#[pymodule(name = "mokapot")]
fn mokapot_module(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add("__doc__", "Python bindings for mokapot")?;
    module.add("__version__", env!("CARGO_PKG_VERSION"))?;
    module.add("MAX_MAJOR_VERSION", jvm::class::MAX_MAJOR_VERSION)?;
    module.add("MokapotError", module.py().get_type::<MokapotError>())?;
    module.add("ParseError", module.py().get_type::<ParseError>())?;
    module.add_class::<PyClass>()?;
    module.add_class::<PyClassRef>()?;
    module.add_class::<PyClassAccessFlags>()?;
    module.add_class::<PyVersion>()?;

    Ok(())
}

pyo3_stub_gen::define_stub_info_gatherer!(stub_info);
