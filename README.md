# MokaPot

[![Cargo Build & Test](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml/badge.svg)](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml)
![Crates.io](https://img.shields.io/crates/v/mokapot)
![docs.rs](https://img.shields.io/docsrs/mokapot)

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
use mokapot::elements::Class;

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
#00000: ldc              => v0 = String("233")
#00002: astore_3         => nop
#00003: iconst_2         => v3 = Integer(2)
#00004: istore           => nop
#00006: iload_1          => nop
#00007: iload            => nop
#00009: iadd             => v9 = v3 + arg0
#00010: istore           => nop
#00012: iload_1          => nop
#00013: ifge             => if arg0 >= 0 goto #00019
#00016: iconst_3         => v16 = Integer(3)
#00017: istore           => nop
#00019: aload_0          => nop
#00020: aload_3          => nop
#00021: iload            => nop
#00023: iload_2          => nop
#00024: invokevirtual    => v24 = call [org/mokapot/test/TestAnalysis]::callMe(this, v0, v3, arg1) // descriptor: (Ljava/lang/String;II)I
#00027: istore           => nop
#00029: iload            => nop
#00031: ireturn          => return v24
#00032: astore_3         => nop
#00033: getstatic        => v33 = [java/lang/System].out
#00036: aload_3          => nop
#00037: invokevirtual    => call [java/io/PrintStream]::println(v33, exception) // descriptor: (Ljava/lang/Object;)V
#00040: iconst_0         => v40 = Integer(0)
#00041: istore_3         => nop
#00042: iload_3          => nop
#00043: iload_2          => nop
#00044: if_icmpge        => if arg1 >= v40 goto #00062
#00047: aload_0          => nop
#00048: ldc              => v48 = String("233")
#00050: iconst_0         => v50 = Integer(0)
#00051: iconst_0         => v51 = Integer(0)
#00052: invokevirtual    => v52 = call [org/mokapot/test/TestAnalysis]::callMe(this, v48, v50, v51) // descriptor: (Ljava/lang/String;II)I
#00055: pop              => nop
#00056: iinc             => v56 = Phi(v40, v56) + 1
#00059: goto             => goto #00042
#00062: iload_1          => nop
#00063: ifle             => if arg0 <= 0 goto #00073
#00066: iload_2          => nop
#00067: ifle             => if arg1 <= 0 goto #00079
#00070: goto             => goto #00077
#00073: iload_2          => nop
#00074: ifge             => if arg1 >= 0 goto #00079
#00077: iconst_0         => v77 = Integer(0)
#00078: ireturn          => return v77
#00079: iload_1          => nop
#00080: invokedynamic    => v80 = get_closure#0[applyAsInt](arg0) // descriptor: (I)Ljava/util/function/IntUnaryOperator;
#00085: astore_3         => nop
#00086: aload_3          => nop
#00087: iconst_0         => v87 = Integer(0)
#00088: invokeinterface  => v88 = call [java/util/function/IntUnaryOperator]::applyAsInt(v80, v87) // descriptor: (I)I
#00093: istore           => nop
#00095: iconst_3         => v95 = Integer(3)
#00096: newarray         => v96 = new I[v95]
#00098: dup              => nop
#00099: iconst_0         => v99 = Integer(0)
#00100: iconst_0         => v100 = Integer(0)
#00101: iastore          => v96[v99] = v100
#00102: dup              => nop
#00103: iconst_1         => v103 = Integer(1)
#00104: iconst_1         => v104 = Integer(1)
#00105: iastore          => v96[v103] = v104
#00106: dup              => nop
#00107: iconst_2         => v107 = Integer(2)
#00108: iconst_2         => v108 = Integer(2)
#00109: iastore          => v96[v107] = v108
#00110: astore           => nop
#00112: aload            => nop
#00114: iconst_0         => v114 = Integer(0)
#00115: iload_1          => nop
#00116: iadd             => v116 = arg0 + v114
#00117: iaload           => v117 = v96[v116]
#00118: istore           => nop
#00120: aload            => nop
#00122: iload            => nop
#00124: iload            => nop
#00126: iastore          => v96[v117] = v117
#00127: iload_2          => nop
#00128: ireturn          => return arg1
```
