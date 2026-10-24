use crate::{
    jvm::{
        Class, ConstantPool, Method,
        annotation::AnnotationsData,
        class,
        code::{ExceptionTableEntryData, InstructionData, MethodBodyData, ProgramCounter},
        method::{AccessFlags, MethodData},
    },
    types::class_name::ClassName,
};

impl Default for Class {
    fn default() -> Self {
        let mut constant_pool = ConstantPool::new();
        let name = constant_pool.insert_class_name(ClassName::new("$default$".to_owned()).unwrap());
        Self {
            version: class::Version::LATEST,
            constant_pool,
            access_flags: class::AccessFlags::empty(),
            name,
            super_class: None,
            interfaces: Box::default(),
            fields: Box::default(),
            methods: Box::default(),
            source_file: None,
            inner_classes: Box::default(),
            enclosing_method: None,
            source_debug_extension: None,
            annotations: AnnotationsData::default(),
            type_annotations: AnnotationsData::default(),
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

pub(crate) struct TestMethod {
    class: Class,
}

impl TestMethod {
    pub(crate) fn as_method(&self) -> Method<'_> {
        self.class
            .methods()
            .next()
            .expect("the test class contains one method")
    }

    pub(crate) fn data_mut(&mut self) -> &mut MethodData {
        self.class
            .methods
            .first_mut()
            .expect("the test class contains one method")
    }
}

pub(crate) struct TestExceptionHandler {
    pub covered_pc: std::ops::Range<ProgramCounter>,
    pub handler_pc: ProgramCounter,
    pub catch_type: Option<ClassName>,
}

pub(crate) fn method_with_pool<I, PC>(
    mut constant_pool: ConstantPool,
    instructions: I,
    descriptor: &str,
    exception_table: Vec<TestExceptionHandler>,
    access_flags: AccessFlags,
) -> TestMethod
where
    I: IntoIterator<Item = (PC, InstructionData)>,
    PC: Into<ProgramCounter>,
{
    let instructions = instructions
        .into_iter()
        .map(|(pc, instruction)| (pc.into(), instruction))
        .collect();
    let name = constant_pool.insert_utf8("test".to_owned().into());
    let descriptor =
        constant_pool.insert_method_descriptor(descriptor.parse().expect("valid test descriptor"));
    let owner = constant_pool.insert_class_name(
        "org/mokapot/Test"
            .parse()
            .expect("the owner must be a valid class reference"),
    );
    let exception_table = exception_table
        .into_iter()
        .map(|entry| ExceptionTableEntryData {
            covered_pc: entry.covered_pc,
            handler_pc: entry.handler_pc,
            catch_type: entry
                .catch_type
                .map(|name| constant_pool.insert_class_name(name)),
        })
        .collect();
    let class = Class {
        constant_pool,
        name: owner,
        methods: Box::new([MethodData {
            access_flags,
            name,
            descriptor,
            body: Some(MethodBodyData {
                max_stack: 4,
                max_locals: 4,
                instructions,
                exception_table,
                line_number_table: None,
                local_variable_table: None,
                stack_map_table: None,
                type_annotations: AnnotationsData::default(),
                other_attributes: Box::new([]),
            }),
            exceptions: Box::new([]),
            annotations: AnnotationsData::default(),
            type_annotations: AnnotationsData::default(),
            parameter_annotations: AnnotationsData::default(),
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
