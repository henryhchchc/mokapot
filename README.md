# MokaPot

[![Cargo Build & Test](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml/badge.svg)](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/henryhchchc/mokapot/graph/badge.svg?token=6M09J26KSM)](https://codecov.io/gh/henryhchchc/mokapot)

[![Crates.io](https://img.shields.io/crates/v/mokapot)](https://crates.io/crates/mokapot)
[![docs.rs](https://img.shields.io/docsrs/mokapot)](https://docs.rs/mokapot)

MokaPot is a Java bytecode analysis library written in Rust.

> [!WARNING]
> **API Stability:** This project is in an early development stage and breaking changes can happen before v1.0.0.
> Documentations are incomplete, which will be added when the basic functionalities works.
> Using this project for production is currently NOT RECOMMENDED.

## Documentation

The documentation of the stable version is available at [docs.rs](https://docs.rs/mokapot).
The documentation of the latest commit is available at [github.io](https://henryhchchc.github.io/mokapot/mokapot/)


## Usage

### Adding the dependency

Add the following line to the `[dependencies]` section in your `Cargo.toml`.
```toml
mokapot = "0.*"
```

Add the following line instead to use the latest development version.
Before building your project, run `cargo update` to fetch the latest commit.
```toml
mokapot = { git = "https://github.com/henryhchchc/mokapot.git" }
```

### Parsing a class

```rust
use mokapot::jvm::class::Class;

let reader: std::io::Read = todo!("Some reader for the byte code");
let class = Class::from_reader(reader)?;
```

## Moka IR

Moka IR is an intermediate representation of JVM bytecode in [mokapot](https://github.com/henryhchchc/mokapot).
It is in a register-based, SSA form, and is designed to be easy to analyze.
Please checkout the module [`mokapot::ir`](https://docs.rs/mokapot/latest/mokapot/ir/index.html) for more information.

To generate Moka IR from a method, use the following code.

```rust
use mokapot::ir::MokaIRMethodExt;

let method: Method = todo!("Some method");
let moka_ir_method = method.generate_moka_ir()?;
```

The following is an example of the generated IR from the method `test()` in [test_data/TestAnalysis.java](test_data/TestAnalysis.java).

You may notice that there are lots of `nop`s in the generated Moka IR.
This because we intented to maintain a bijection between the original bytecode and the generated Moka IR.
Such a bijection facilitates the analysis involving dynamic execution - the runtime information (e.g., coverage) can be applied to Moka IR without needing any remapping.

```text
#00000: ldc              => %0 := String("233")
#00002: astore_3         => nop
#00003: iconst_2         => %3 := int(2)
#00004: istore           => nop
#00006: iload_1          => nop
#00007: iload            => nop
#00009: iadd             => %9 := %3 + %arg0
#00010: istore           => nop
#00012: iload_1          => nop
#00013: ifge             => if %arg0 >= 0 goto #00019
#00016: iconst_3         => %16 := int(3)
#00017: istore           => nop
#00019: aload_0          => nop
#00020: aload_3          => nop
#00021: iload            => nop
#00023: iload_2          => nop
#00024: invokevirtual    => %24 := call %this::callMe(%0, Phi(%3, %16), %arg1) // owner: org/mokapot/test/TestAnalysis, desc: (Ljava/lang/String;II)I
#00027: istore           => nop
#00029: iload            => nop
#00031: ireturn          => return %24
#00032: astore_3         => nop
#00033: getstatic        => %33 := java/lang/System.out
#00036: aload_3          => nop
#00037: invokevirtual    => %37 := call %33::println(%caught_exception) // owner: java/io/PrintStream, desc: (Ljava/lang/Object;)V
#00040: iconst_0         => %40 := int(0)
#00041: istore_3         => nop
#00042: iload_3          => nop
#00043: iload_2          => nop
#00044: if_icmpge        => if %arg1 >= Phi(%40, %64) goto #00070
#00047: getstatic        => %47 := java/lang/System.out
#00050: ldc              => %50 := String(0x61 0x02 0xED 0xA0 0x80 0x62 0x63 0x64 0x65 0x66) // Invalid UTF-8
#00052: invokevirtual    => %52 := call %47::println(%50) // owner: java/io/PrintStream, desc: (Ljava/lang/String;)V
#00055: aload_0          => nop
#00056: ldc              => %56 := String("233")
#00058: iconst_0         => %58 := int(0)
#00059: iconst_0         => %59 := int(0)
#00060: invokevirtual    => %60 := call %this::callMe(%56, %58, %59) // owner: org/mokapot/test/TestAnalysis, desc: (Ljava/lang/String;II)I
#00063: pop              => nop
#00064: iinc             => %64 := Phi(%40, %64) + 1
#00067: goto             => goto #00042
#00070: iload_1          => nop
#00071: ifle             => if %arg0 <= 0 goto #00081
#00074: iload_2          => nop
#00075: ifle             => if %arg1 <= 0 goto #00087
#00078: goto             => goto #00085
#00081: iload_2          => nop
#00082: ifge             => if %arg1 >= 0 goto #00087
#00085: iconst_0         => %85 := int(0)
#00086: ireturn          => return %85
#00087: iload_1          => nop
#00088: invokedynamic    => %88 := get_closure#0[applyAsInt](%arg0) // desc: (I)Ljava/util/function/IntUnaryOperator;
#00093: astore_3         => nop
#00094: aload_3          => nop
#00095: iconst_0         => %95 := int(0)
#00096: invokeinterface  => %96 := call %88::applyAsInt(%95) // owner: java/util/function/IntUnaryOperator, desc: (I)I
#00101: istore           => nop
#00103: iconst_3         => %103 := int(3)
#00104: newarray         => %104 := new I[%103]
#00106: dup              => nop
#00107: iconst_0         => %107 := int(0)
#00108: iconst_0         => %108 := int(0)
#00109: iastore          => %109 := %104[%107] = %108
#00110: dup              => nop
#00111: iconst_1         => %111 := int(1)
#00112: iconst_1         => %112 := int(1)
#00113: iastore          => %113 := %104[%111] = %112
#00114: dup              => nop
#00115: iconst_2         => %115 := int(2)
#00116: iconst_2         => %116 := int(2)
#00117: iastore          => %117 := %104[%115] = %116
#00118: astore           => nop
#00120: aload            => nop
#00122: iconst_0         => %122 := int(0)
#00123: iload_1          => nop
#00124: iadd             => %124 := %arg0 + %122
#00125: iaload           => %125 := %104[%124]
#00126: istore           => nop
#00128: aload            => nop
#00130: iload            => nop
#00132: iload            => nop
#00134: iastore          => %134 := %104[%125] = %125
#00135: iload_2          => nop
#00136: ireturn          => return %arg1
```
