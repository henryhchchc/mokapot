//! JVM type and name parsing without an external corpus.

use std::{fmt, hint::black_box, sync::LazyLock};

use divan::{
    Bencher,
    counter::{BytesCount, ItemsCount},
};
use mokapot::{
    jvm::JavaString,
    types::{
        class_name::ClassName, field_type::FieldType, method_descriptor::MethodDescriptor,
        reference_type::ReferenceType,
    },
};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const PARSES_PER_BATCH: usize = 1_024;

// Measure repeated parsing to amortize startup and cold-cache costs.
fn bench_batch<T>(bencher: Bencher, bytes: usize, parse: impl Fn() -> T + Sync) {
    bencher
        .counter(BytesCount::new(bytes * PARSES_PER_BATCH))
        .counter(ItemsCount::new(PARSES_PER_BATCH))
        .bench(|| {
            for _ in 0..PARSES_PER_BATCH {
                drop(black_box(parse()));
            }
        });
}

const FIELD_DESCRIPTORS: &[&str] = &[
    "Ljava/lang/String;",
    "Ljava/util/Map$Entry;",
    "Lexample/日本語;",
    "Lexample/😀;",
    "[I",
    "[[[D",
    "[[Ljava/lang/String;",
];

const METHOD_DESCRIPTORS: &[&str] = &[
    "()I",
    "()Ljava/lang/Object;",
    "()[I",
    "(JFD)V",
    "([[I[[Ljava/lang/String;)[[D",
    "(II)I",
    "(ZBCSIJFD)D",
    "([Ljava/lang/String;)V",
    "(Ljava/lang/Object;Ljava/lang/Object;)Ljava/lang/Object;",
    "(I[BLjava/lang/String;[[D)[Ljava/lang/Object;",
    "(Lexample/日本語;)Lexample/日本語;",
];

struct Case {
    name: &'static str,
    descriptor: String,
    valid: bool,
}

impl fmt::Display for Case {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name)
    }
}

static HEAVY_FIELDS: LazyLock<Vec<Case>> = LazyLock::new(|| {
    let object = format!("L{}日本語;", "example/package/".repeat(64));
    let array = format!("{}{}", "[".repeat(255), object);
    [
        ("long_object", object.clone(), true),
        ("array_255_dimensions", array.clone(), true),
        ("empty_object_name", "L;".to_owned(), false),
        (
            "empty_name_component",
            "Ljava//lang/String;".to_owned(),
            false,
        ),
        (
            "missing_semicolon",
            object.trim_end_matches(';').to_owned(),
            false,
        ),
        (
            "invalid_name_at_end",
            format!("{}.;", &object[..object.len() - 1]),
            false,
        ),
        ("array_missing_element", "[".repeat(255), false),
        ("array_void_element", format!("{}V", "[".repeat(255)), false),
        ("trailing_input", format!("{array}I"), false),
    ]
    .into_iter()
    .map(|(name, descriptor, valid)| Case {
        name,
        descriptor,
        valid,
    })
    .collect()
});

static HEAVY_METHODS: LazyLock<Vec<Case>> = LazyLock::new(|| {
    let parameters = "Ljava/lang/String;".repeat(255);
    let mixed = "I[BLjava/lang/String;[[D".repeat(32);
    [
        ("parameters_255", format!("({parameters})V"), true),
        (
            "mixed_parameters",
            format!("({mixed})[[Ljava/lang/Object;"),
            true,
        ),
        (
            "deep_array_return",
            format!("(){0}I", "[".repeat(255)),
            true,
        ),
        (
            "truncated_object_parameter",
            "(Ljava/lang/String)V".to_owned(),
            false,
        ),
        ("missing_close_paren", format!("({parameters}"), false),
        ("invalid_last_parameter", format!("({parameters}V)V"), false),
        ("missing_return", format!("({parameters})"), false),
        ("invalid_return", format!("({parameters})[V"), false),
        ("trailing_input", format!("({parameters})VI"), false),
    ]
    .into_iter()
    .map(|(name, descriptor, valid)| Case {
        name,
        descriptor,
        valid,
    })
    .collect()
});

mod field_type {
    use super::{
        Bencher, Case, FIELD_DESCRIPTORS, FieldType, HEAVY_FIELDS, JavaString, bench_batch,
        black_box,
    };

    #[divan::bench(args = HEAVY_FIELDS.iter())]
    fn heavy_from_str_batched(bencher: Bencher, case: &Case) {
        assert_eq!(case.descriptor.parse::<FieldType>().is_ok(), case.valid);
        bench_batch(bencher, case.descriptor.len(), || {
            black_box(case.descriptor.as_str()).parse::<FieldType>()
        });
    }

    #[divan::bench(args = HEAVY_FIELDS.iter())]
    fn heavy_from_java_str_batched(bencher: Bencher, case: &Case) {
        let encoded = JavaString::from(case.descriptor.clone());
        let descriptor = encoded.as_java_str();
        assert_eq!(FieldType::try_from(descriptor).is_ok(), case.valid);
        bench_batch(bencher, descriptor.as_bytes().len(), || {
            FieldType::try_from(black_box(descriptor))
        });
    }

    #[divan::bench(args = FIELD_DESCRIPTORS)]
    fn from_str_batched(bencher: Bencher, descriptor: &str) {
        bench_batch(bencher, descriptor.len(), || {
            black_box(descriptor).parse::<FieldType>().unwrap()
        });
    }

    #[divan::bench(args = FIELD_DESCRIPTORS)]
    fn from_java_str_batched(bencher: Bencher, descriptor: &str) {
        let encoded = JavaString::from(descriptor.to_owned());
        let descriptor = encoded.as_java_str();
        bench_batch(bencher, descriptor.as_bytes().len(), || {
            FieldType::try_from(black_box(descriptor)).unwrap()
        });
    }
}

mod method_descriptor {
    use super::{
        Bencher, Case, HEAVY_METHODS, JavaString, METHOD_DESCRIPTORS, MethodDescriptor,
        bench_batch, black_box,
    };

    #[divan::bench(args = HEAVY_METHODS.iter())]
    fn heavy_from_str_batched(bencher: Bencher, case: &Case) {
        assert_eq!(
            case.descriptor.parse::<MethodDescriptor>().is_ok(),
            case.valid
        );
        bench_batch(bencher, case.descriptor.len(), || {
            black_box(case.descriptor.as_str()).parse::<MethodDescriptor>()
        });
    }

    #[divan::bench(args = HEAVY_METHODS.iter())]
    fn heavy_from_java_str_batched(bencher: Bencher, case: &Case) {
        let encoded = JavaString::from(case.descriptor.clone());
        let descriptor = encoded.as_java_str();
        assert_eq!(MethodDescriptor::try_from(descriptor).is_ok(), case.valid);
        bench_batch(bencher, descriptor.as_bytes().len(), || {
            MethodDescriptor::try_from(black_box(descriptor))
        });
    }

    #[divan::bench(args = METHOD_DESCRIPTORS)]
    fn from_str_batched(bencher: Bencher, descriptor: &str) {
        bench_batch(bencher, descriptor.len(), || {
            black_box(descriptor).parse::<MethodDescriptor>().unwrap()
        });
    }

    #[divan::bench(args = METHOD_DESCRIPTORS)]
    fn from_java_str_batched(bencher: Bencher, descriptor: &str) {
        let encoded = JavaString::from(descriptor.to_owned());
        let descriptor = encoded.as_java_str();
        bench_batch(bencher, descriptor.as_bytes().len(), || {
            MethodDescriptor::try_from(black_box(descriptor)).unwrap()
        });
    }
}

fn main() {
    divan::main();
}

mod class_name {
    use super::{Bencher, Case, ClassName, LazyLock, bench_batch, black_box};

    static CASES: LazyLock<Vec<Case>> = LazyLock::new(|| {
        [
            ("ascii", "java/lang/String".to_owned(), true),
            ("unicode", "example/日本語".to_owned(), true),
            ("supplementary", "example/😀".to_owned(), true),
            (
                "long",
                format!("{}Type", "example/package/".repeat(64)),
                true,
            ),
            ("nested_class", "java/util/Map$Entry".to_owned(), true),
            ("leading_separator", "/java/lang/String".to_owned(), false),
            ("trailing_separator", "java/lang/String/".to_owned(), false),
            ("empty_component", "java//lang/String".to_owned(), false),
            ("empty", String::new(), false),
            ("invalid_separator", "java.lang.String".to_owned(), false),
            (
                "invalid_at_end",
                format!("{}Type;", "example/package/".repeat(64)),
                false,
            ),
        ]
        .into_iter()
        .map(|(name, descriptor, valid)| Case {
            name,
            descriptor,
            valid,
        })
        .collect()
    });

    #[divan::bench(args = CASES.iter())]
    fn from_str_batched(bencher: Bencher, case: &Case) {
        assert_eq!(case.descriptor.parse::<ClassName>().is_ok(), case.valid);
        bench_batch(bencher, case.descriptor.len(), || {
            black_box(case.descriptor.as_str()).parse::<ClassName>()
        });
    }
}

mod reference_type {
    use super::{Bencher, Case, LazyLock, ReferenceType, bench_batch, black_box};

    static CASES: LazyLock<Vec<Case>> = LazyLock::new(|| {
        [
            ("internal_name", "java/lang/String".to_owned(), true),
            ("object_descriptor", "Ljava/lang/String;".to_owned(), true),
            ("unicode", "Lexample/日本語;".to_owned(), true),
            ("supplementary", "Lexample/😀;".to_owned(), true),
            ("primitive_array", "[I".to_owned(), true),
            ("object_array", "[Ljava/lang/String;".to_owned(), true),
            ("multidimensional_primitive_array", "[[[D".to_owned(), true),
            ("unicode_array", "[Lexample/日本語;".to_owned(), true),
            ("supplementary_array", "[Lexample/😀;".to_owned(), true),
            ("array_trailing_input", "[II".to_owned(), false),
            (
                "deep_array",
                format!("{}Ljava/lang/String;", "[".repeat(255)),
                true,
            ),
            ("empty", String::new(), false),
            ("invalid_name", "java.lang.String".to_owned(), false),
        ]
        .into_iter()
        .map(|(name, descriptor, valid)| Case {
            name,
            descriptor,
            valid,
        })
        .collect()
    });

    #[divan::bench(args = CASES.iter())]
    fn from_str_batched(bencher: Bencher, case: &Case) {
        assert_eq!(case.descriptor.parse::<ReferenceType>().is_ok(), case.valid);
        bench_batch(bencher, case.descriptor.len(), || {
            black_box(case.descriptor.as_str()).parse::<ReferenceType>()
        });
    }
}
