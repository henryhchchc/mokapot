use super::attribute::Attribute;
use crate::{
    elements::{
        field::Field, references::ClassReference, Class, ClassAccessFlags, ClassVersion, Method,
    },
    errors::ClassFileParsingError,
    macros::fill_once,
    reader_utils::{read_u16, read_u32},
};

use super::{attribute::AttributeList, parsing_context::ParsingContext};

impl Class {
    pub fn from_reader<R>(reader: R) -> Result<Class, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let mut reader = reader;
        let magic = read_u32(&mut reader)?;
        if magic != JAVA_CLASS_MAIGC {
            return Err(ClassFileParsingError::NotAClassFile);
        }
        let version = ClassVersion::parse(&mut reader)?;
        let parsing_context = ParsingContext::parse(&mut reader, &version)?;
        let access = read_u16(&mut reader)?;
        let Some(access_flags) = ClassAccessFlags::from_bits(access) else {
            return Err(ClassFileParsingError::UnknownFlags(access, "class"));
        };
        let this_class_idx = read_u16(&mut reader)?;
        let ClassReference { binary_name } = parsing_context.get_class_ref(this_class_idx)?;
        let super_class_idx = read_u16(&mut reader)?;
        let super_class = match super_class_idx {
            0 if binary_name == "java/lang/Object" => None,
            0 if access_flags.contains(ClassAccessFlags::MODULE) => None,
            0 => Err(ClassFileParsingError::MalformedClassFile(
                "Class must have a super type except for java/lang/Object or a module",
            ))?,
            it @ _ => Some(parsing_context.get_class_ref(it)?),
        };

        let interfaces_count = read_u16(&mut reader)?;
        let interfaces = (0..interfaces_count)
            .map(|_| {
                let interface_idx = read_u16(&mut reader)?;
                parsing_context.get_class_ref(interface_idx)
            })
            .collect::<Result<_, ClassFileParsingError>>()?;
        let fields_count = read_u16(&mut reader)?;
        let fields = (0..fields_count)
            .into_iter()
            .map(|_| Field::parse(&mut reader, &parsing_context))
            .collect::<Result<_, ClassFileParsingError>>()?;

        let methods_count = read_u16(&mut reader)?;
        let methods = (0..methods_count)
            .map(|_| Method::parse(&mut reader, &parsing_context))
            .collect::<Result<_, ClassFileParsingError>>()?;

        let attributes = AttributeList::parse(&mut reader, &parsing_context)?;

        let mut may_remain: [u8; 1] = [0];
        let remain = reader.read(&mut may_remain)?;
        if remain == 1 {
            return Err(ClassFileParsingError::UnexpectedData);
        }

        let mut source_file = None;
        let mut inner_classes = None;
        let mut enclosing_method = None;
        let mut source_debug_extension = None;
        let mut rt_visible_anno = None;
        let mut rt_invisible_anno = None;
        let mut rt_visible_type_anno = None;
        let mut rt_invisible_type_anno = None;
        let mut bootstrap_methods = None;
        let mut module = None;
        let mut module_packages = None;
        let mut module_main_class = None;
        let mut nest_host = None;
        let mut nest_members = None;
        let mut permitted_subclasses = None;
        let mut is_synthetic = false;
        let mut is_deprecated = false;
        let mut signature = None;
        let mut record = None;
        for attr in attributes.into_iter() {
            use Attribute::*;
            match attr {
                SourceFile(it) => fill_once!(source_file, it, "SourceFile"),
                InnerClasses(it) => fill_once!(inner_classes, it, "InnerClasses"),
                EnclosingMethod(it) => fill_once!(enclosing_method, it, "EnclosingMethod"),
                SourceDebugExtension(it) => {
                    fill_once!(source_debug_extension, it, "SourceDebugExtension")
                }
                RuntimeVisibleAnnotations(it) => {
                    fill_once!(rt_visible_anno, it, "RuntimeVisibleAnnotations")
                }
                RuntimeInvisibleAnnotations(it) => {
                    fill_once!(rt_invisible_anno, it, "RuntimeInvisibleAnnotations")
                }
                RuntimeVisibleTypeAnnotations(it) => {
                    fill_once!(rt_visible_type_anno, it, "RuntimeVisibleTypeAnnotations")
                }
                RuntimeInvisibleTypeAnnotations(it) => fill_once!(
                    rt_invisible_type_anno,
                    it,
                    "RuntimeInvisibleTypeAnnotations"
                ),
                BootstrapMethods(bm) => fill_once!(bootstrap_methods, bm, "BootstrapMethods"),
                Module(it) => fill_once!(module, it, "Module"),
                ModulePackages(mp) => fill_once!(module_packages, mp, "ModulePackages"),
                ModuleMainClass(mmc) => fill_once!(module_main_class, mmc, "ModuleMainClass"),
                NestHost(nh) => fill_once!(nest_host, nh, "NestHost"),
                NestMembers(nm) => fill_once!(nest_members, nm, "NestMembers"),
                PermittedSubclasses(ps) => {
                    fill_once!(permitted_subclasses, ps, "PermittedSubclasses")
                }
                Synthetic => is_synthetic = true,
                Deprecated => is_deprecated = true,
                Signature(sig) => fill_once!(signature, sig, "Signature"),
                Record(rec) => fill_once!(record, rec, "Recoed"),
                unexpected => Err(ClassFileParsingError::UnexpectedAttribute(
                    unexpected.name(),
                    "class_file",
                ))?,
            }
        }

        Ok(Class {
            version,
            access_flags,
            binary_name,
            super_class,
            interfaces,
            fields,
            methods,
            source_file,
            inner_classes: inner_classes.unwrap_or_default(),
            enclosing_method,
            source_debug_extension: source_debug_extension.unwrap_or_default(),
            runtime_visible_annotations: rt_visible_anno.unwrap_or_default(),
            runtime_invisible_annotations: rt_invisible_anno.unwrap_or_default(),
            runtime_visible_type_annotations: rt_visible_type_anno.unwrap_or_default(),
            runtime_invisible_type_annotations: rt_invisible_type_anno.unwrap_or_default(),
            bootstrap_methods: bootstrap_methods.unwrap_or_default(),
            module,
            module_packages: module_packages.unwrap_or_default(),
            module_main_class,
            nest_host,
            nest_members: nest_members.unwrap_or_default(),
            permitted_subclasses: permitted_subclasses.unwrap_or_default(),
            is_synthetic,
            is_deprecated,
            signature,
            record: record.unwrap_or_default(),
        })
    }
}

impl ClassVersion {
    /// Returns `true` if this class file is compiled with `--enable-preview`.
    pub fn is_preview_enabled(&self) -> bool {
        self.minor == 65535
    }
    fn parse<R>(reader: &mut R) -> std::io::Result<Self>
    where
        R: std::io::Read,
    {
        let minor = read_u16(reader)?;
        let major = read_u16(reader)?;
        Ok(Self { major, minor })
    }
}

const JAVA_CLASS_MAIGC: u32 = 0xCAFEBABE;

#[cfg(test)]
mod test {
    use std::io::BufReader;

    use crate::{
        elements::{
            class::{Class, ClassAccessFlags},
            method::ReturnType,
            references::ClassReference,
        },
        errors::ClassFileParsingError,
        types::{FieldType, PrimitiveType},
    };

    /// Parses the class file compiled from `MyClass.java` from the `test_data` directory.
    /// The source code ot the class files is as follows.
    fn parse_my_class() -> Result<Class, ClassFileParsingError> {
        let bytes = include_bytes!(concat!(
            env!("OUT_DIR"),
            "/java_classes/org/pkg/MyClass.class"
        ));
        let reader = BufReader::new(&bytes[..]);
        Class::from_reader(reader)
    }

    #[test]
    fn test_parse_file() {
        assert!(parse_my_class().is_ok());
    }

    #[test]
    fn test_parse_version() {
        let my_class = parse_my_class().unwrap();
        assert_eq!(65, my_class.version.major);
        assert_eq!(0, my_class.version.minor);
        assert!(!my_class.version.is_preview_enabled());
    }

    #[test]
    fn test_access_flag() {
        let my_class = parse_my_class().unwrap();
        let expected = ClassAccessFlags::PUBLIC | ClassAccessFlags::SUPER;
        assert_eq!(expected, my_class.access_flags);
    }

    #[test]
    fn test_class_name() {
        let my_class = parse_my_class().unwrap();
        assert_eq!("org/pkg/MyClass", my_class.binary_name);
    }

    #[test]
    fn test_super_class_name() {
        let my_class = parse_my_class().unwrap();
        assert_eq!(
            Some(ClassReference::new("java/lang/Object")),
            my_class.super_class
        );
    }

    #[test]
    fn test_interfaces() {
        let my_class = parse_my_class().unwrap();
        let mut interfaces = my_class.interfaces.into_iter();
        assert_eq!(
            Some(ClassReference::new("java/lang/Cloneable")),
            interfaces.next()
        );
    }

    #[test]
    fn test_fields() {
        let my_class = parse_my_class().unwrap();
        assert_eq!(2, my_class.fields.len());
        let test_field = &my_class
            .fields
            .iter()
            .filter(|f| f.name == "test")
            .next()
            .unwrap();
        assert_eq!(FieldType::Base(PrimitiveType::Long), test_field.field_type);
    }

    #[test]
    fn test_methods() {
        let my_class = parse_my_class().unwrap();
        assert_eq!(4, my_class.methods.len());
        let main_method = &my_class
            .methods
            .iter()
            .filter(|f| f.name == "main")
            .next()
            .expect("main method not found");
        assert_eq!(ReturnType::Void, main_method.descriptor.return_type);
        assert_eq!(
            FieldType::Object(ClassReference::new("java/lang/String")).make_array_type(),
            main_method.descriptor.parameters_types[0]
        );
    }

    #[test]
    fn not_a_class_file() {
        let bytes = include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/test_data/MyClass.java"
        ));
        assert!(matches!(
            Class::from_reader(bytes.as_slice()),
            Err(ClassFileParsingError::NotAClassFile)
        ));
    }
}
