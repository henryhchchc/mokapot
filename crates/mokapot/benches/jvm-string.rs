//! JVM string conversions and formatting.

use std::{
    fmt::{self, Write},
    hint::black_box,
};

use divan::{Bencher, counter::BytesCount};
use mokapot::jvm::{JStr, JString};

#[global_allocator]
static ALLOC: divan::AllocProfiler = divan::AllocProfiler::system();

const LONG_ASCII: &str = concat!(
    "(Ljava/lang/String;Ljava/util/List;Ljava/util/Map;Ljava/util/Set;",
    "Ljava/lang/String;Ljava/util/List;Ljava/util/Map;Ljava/util/Set;",
    "Ljava/lang/String;Ljava/util/List;Ljava/util/Map;Ljava/util/Set;",
    "Ljava/lang/String;Ljava/util/List;Ljava/util/Map;Ljava/util/Set;)V",
);

#[derive(Clone, Copy)]
struct Case<T> {
    name: &'static str,
    input: T,
}

impl<T> Case<T> {
    const fn new(name: &'static str, input: T) -> Self {
        Self { name, input }
    }
}

impl<T> fmt::Debug for Case<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name)
    }
}

const TEXT_CASES: &[Case<&str>] = &[
    Case::new("empty", ""),
    Case::new("ascii", "java/lang/String"),
    Case::new("long_ascii", LONG_ASCII),
    Case::new("bmp", "日本語"),
    Case::new("nul", "a\0b"),
    Case::new("supplementary", "a😀b"),
];

const BYTE_CASES: &[Case<&[u8]>] = &[
    Case::new("empty", b""),
    Case::new("ascii", b"java/lang/String"),
    Case::new("long_ascii", LONG_ASCII.as_bytes()),
    Case::new("bmp", "日本語".as_bytes()),
    Case::new("nul", b"a\xc0\x80b"),
    Case::new("supplementary", b"a\xed\xa0\xbd\xed\xb8\x80b"),
    Case::new("unpaired_surrogate", b"a\xed\xa0\x80b"),
    Case::new("malformed", b"a\xffb"),
];

#[divan::bench(args = TEXT_CASES)]
fn from_str(bencher: Bencher, case: Case<&str>) {
    bencher
        .counter(BytesCount::new(case.input.len()))
        .bench(|| JString::from(black_box(case.input)));
}

#[divan::bench(args = TEXT_CASES)]
fn from_string(bencher: Bencher, case: Case<&str>) {
    bencher
        .counter(BytesCount::new(case.input.len()))
        .with_inputs(|| case.input.to_owned())
        .bench_values(|input| JString::from(black_box(input)));
}

#[divan::bench(args = BYTE_CASES)]
fn to_str(bencher: Bencher, case: Case<&[u8]>) {
    let input = JStr::from_bytes(case.input);
    bencher
        .counter(BytesCount::new(case.input.len()))
        .bench(|| black_box(input).to_str());
}

#[divan::bench(args = BYTE_CASES)]
fn to_owned(bencher: Bencher, case: Case<&[u8]>) {
    let input = JStr::from_bytes(case.input);
    bencher
        .counter(BytesCount::new(case.input.len()))
        .bench(|| black_box(input).to_owned());
}

#[derive(Default)]
struct Sink(usize);

impl Write for Sink {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        self.0 = self.0.wrapping_add(black_box(text).len());
        Ok(())
    }
}

#[divan::bench(args = BYTE_CASES)]
fn display(bencher: Bencher, case: Case<&[u8]>) {
    let input = JStr::from_bytes(case.input);
    bencher
        .counter(BytesCount::new(case.input.len()))
        .bench_local(|| {
            let mut sink = Sink::default();
            write!(sink, "{}", black_box(input)).unwrap();
            black_box(sink.0);
        });
}

#[divan::bench(args = BYTE_CASES)]
fn debug(bencher: Bencher, case: Case<&[u8]>) {
    let input = JStr::from_bytes(case.input);
    bencher
        .counter(BytesCount::new(case.input.len()))
        .bench_local(|| {
            let mut sink = Sink::default();
            write!(sink, "{:?}", black_box(input)).unwrap();
            black_box(sink.0);
        });
}

fn main() {
    divan::main();
}
