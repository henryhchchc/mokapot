use crate::{
    jvm::{
        Annotations, Class, ConstantPool, Method, class,
        code::{ExceptionTableEntry, Instruction, MethodBody, ProgramCounter},
        method::{AccessFlags, MethodData},
    },
    types::class_name::ClassName,
};

#[rustfmt::skip]
#[must_use]
/// Creates an empty class with the specified major and minor version numbers.
///
/// # Parameters
/// - `major`: The major version number.
/// - `minor`: The minor version number.
///
/// # Returns
/// A byte array representing the class file structure of an empty class.
pub const fn empty_class_with_version(major: u16, minor: u16) -> [u8; 40] {
    [
        0xCA, 0xFE, 0xBA, 0xBE, // Magic
        minor.to_be_bytes()[0], minor.to_be_bytes()[1], // Minor version
        major.to_be_bytes()[0], major.to_be_bytes()[1], // Major version
        // Constant pool
        0x00, 0x03, // Constant pool count 2+1
        0x07, // Tag: Class
        0x00, 0x02, // Name index: 2
        0x01, // Tag: Utf8
        0x00, 0x0A, // Length of string: 10
        0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x57, 0x6F, 0x72, 0x6C, 0x64, // "Helloworld"
        0x00, 0x01, // Access flags: public
        0x00, 0x01, // This class index
        0x00, 0x01, // Super class index
        0x00, 0x00, // Interfaces count
        0x00, 0x00, // Fields count
        0x00, 0x00, // Methods count
        0x00, 0x00, // Attributes count
    ]
}

impl Default for Class {
    fn default() -> Self {
        Self {
            version: class::Version::LATEST,
            constant_pool: ConstantPool::new(),
            access_flags: class::AccessFlags::empty(),
            name: ClassName::new("$default$").unwrap(),
            super_class: None,
            interfaces: Box::default(),
            fields: Box::default(),
            methods: Box::default(),
            source_file: None,
            inner_classes: Box::default(),
            enclosing_method: None,
            source_debug_extension: None,
            annotations: Annotations::default(),
            type_annotations: Annotations::default(),
            bootstrap_methods: Box::default(),
            module: None,
            module_packages: Box::default(),
            module_main_class: None,
            nest_host: None,
            nest_members: Box::default(),
            permitted_subclasses: Box::default(),
            is_synthetic: false,
            is_deprecated: false,
            signature: None,
            record: None,
            other_attributes: Box::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct TestMethod {
    class: Class,
}

impl TestMethod {
    pub(crate) fn set_name(&mut self, name: &str) {
        let id = self.class.constant_pool.insert_utf8(name.into());
        self.class.methods[0].name = id;
    }

    pub(crate) fn as_method(&self) -> Method<'_> {
        self.class
            .methods()
            .next()
            .expect("the test class contains one method")
    }
}

impl std::ops::Deref for TestMethod {
    type Target = MethodData;

    fn deref(&self) -> &Self::Target {
        self.class
            .methods
            .first()
            .expect("the test class contains one method")
    }
}

impl std::ops::DerefMut for TestMethod {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.class
            .methods
            .first_mut()
            .expect("the test class contains one method")
    }
}

/// Builds a method named `test` in `org/mokapot/Test` from its body.
pub(crate) fn method<I, PC>(
    instructions: I,
    descriptor: &str,
    exception_table: Vec<ExceptionTableEntry>,
    access_flags: AccessFlags,
) -> TestMethod
where
    I: IntoIterator<Item = (PC, Instruction)>,
    PC: Into<ProgramCounter>,
{
    let instructions = instructions
        .into_iter()
        .map(|(pc, instruction)| (pc.into(), instruction))
        .collect();
    let mut constant_pool = ConstantPool::new();
    let name = constant_pool.insert_utf8("test".into());
    let descriptor =
        constant_pool.insert_method_descriptor(descriptor.parse().expect("valid test descriptor"));
    let class = Class {
        constant_pool,
        name: "org/mokapot/Test"
            .parse()
            .expect("the owner must be a valid class reference"),
        methods: Box::new([MethodData {
            access_flags,
            name,
            descriptor,
            body: Some(MethodBody {
                max_stack: 4,
                max_locals: 4,
                instructions,
                exception_table: exception_table.into_boxed_slice(),
                line_number_table: None,
                local_variable_table: None,
                stack_map_table: None,
                type_annotations: Annotations::default(),
                other_attributes: Box::new([]),
            }),
            exceptions: Box::new([]),
            annotations: Annotations::default(),
            type_annotations: Annotations::default(),
            parameter_annotations: Annotations::default(),
            annotation_default: None,
            parameters: Box::new([]),
            is_synthetic: false,
            is_deprecated: false,
            signature: None,
            other_attributes: Box::new([]),
        }]),
        ..Class::default()
    };
    TestMethod { class }
}
