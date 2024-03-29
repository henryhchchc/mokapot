## MokaIR

MokaIR is an intermediate representation of JVM bytecode in [mokapot](https://github.com/henryhchchc/mokapot).
It is in a register-based, SSA form, and is designed to be easy to analyze.
Please checkout the module [`mokapot::ir`](https://docs.rs/mokapot/latest/mokapot/ir/index.html) for more information.

To generate MokaIR from a method, use the following code.

```rust
use mokapot::ir::{MokaIRMethod, MokaIRMethodExt};

fn moka_ir() -> Result<MokaIRMethod, Box<dyn std::error::Error>> {
    let method: Method = todo!("Some method");
    let moka_ir_method = method.brew()?;
    Ok(moka_ir_method)
}
```

The following is an example of the generated IR from the method `test()` in [test_data/TestAnalysis.java](test_data/TestAnalysis.java).

You may notice that there are lots of `nop`s in the generated MokaIR.
This because we indent to maintain a bijection between the original bytecode and the generated MokaIR.
Such a bijection facilitates the analysis involving dynamic execution - the runtime information (e.g., coverage) can be applied to MokaIR without needing any remapping.

```
#00000: ldc              => %0 = String("233")
#00002: astore_3         => nop
#00003: iconst_2         => %3 = int(2)
#00004: istore           => nop
#00006: iload_1          => nop
#00007: iload            => nop
#00009: iadd             => %9 = %3 + %arg0
#00010: istore           => nop
#00012: iload_1          => nop
#00013: ifge             => if %arg0 >= 0 goto #00019
#00016: iconst_3         => %16 = int(3)
#00017: istore           => nop
#00019: aload_0          => nop
#00020: aload_3          => nop
#00021: iload            => nop
#00023: iload_2          => nop
#00024: invokevirtual    => %24 = call int %this@org/mokapot/test/TestAnalysis::callMe(%0, Phi(%3, %16), %arg1)
#00027: istore           => nop
#00029: iload            => nop
#00031: ireturn          => return %24
#00032: astore_3         => nop
#00033: getstatic        => %33 = read java/lang/System.out
#00036: aload_3          => nop
#00037: invokevirtual    => %37 = call void %33@java/io/PrintStream::println(%caught_exception)
#00040: iconst_0         => %40 = int(0)
#00041: istore_3         => nop
#00042: iload_3          => nop
#00043: iload_2          => nop
#00044: if_icmpge        => if %arg1 >= Phi(%40, %64) goto #00070
#00047: getstatic        => %47 = read java/lang/System.out
#00050: ldc              => %50 = String(0x61 0x02 0xED 0xA0 0x80 0x62 0x63 0x64 0x65 0x66) // Invalid UTF-8
#00052: invokevirtual    => %52 = call void %47@java/io/PrintStream::println(%50)
#00055: aload_0          => nop
#00056: ldc              => %56 = String("233")
#00058: iconst_0         => %58 = int(0)
#00059: iconst_0         => %59 = int(0)
#00060: invokevirtual    => %60 = call int %this@org/mokapot/test/TestAnalysis::callMe(%56, %58, %59)
#00063: pop              => nop
#00064: iinc             => %64 = Phi(%40, %64) + 1
#00067: goto             => goto #00042
#00070: iload_1          => nop
#00071: ifle             => if %arg0 <= 0 goto #00081
#00074: iload_2          => nop
#00075: ifle             => if %arg1 <= 0 goto #00087
#00078: goto             => goto #00085
#00081: iload_2          => nop
#00082: ifge             => if %arg1 >= 0 goto #00087
#00085: iconst_0         => %85 = int(0)
#00086: ireturn          => return %85
#00087: iload_1          => nop
#00088: invokedynamic    => %88 = closure java/util/function/IntUnaryOperator applyAsInt#0(%arg0)
#00093: astore_3         => nop
#00094: aload_3          => nop
#00095: iconst_0         => %95 = int(0)
#00096: invokeinterface  => %96 = call int %88@java/util/function/IntUnaryOperator::applyAsInt(%95)
#00101: istore           => nop
#00103: iconst_3         => %103 = int(3)
#00104: newarray         => %104 = new I[%103]
#00106: dup              => nop
#00107: iconst_0         => %107 = int(0)
#00108: iconst_0         => %108 = int(0)
#00109: iastore          => %109 = %104[%107] = %108
#00110: dup              => nop
#00111: iconst_1         => %111 = int(1)
#00112: iconst_1         => %112 = int(1)
#00113: iastore          => %113 = %104[%111] = %112
#00114: dup              => nop
#00115: iconst_2         => %115 = int(2)
#00116: iconst_2         => %116 = int(2)
#00117: iastore          => %117 = %104[%115] = %116
#00118: astore           => nop
#00120: aload            => nop
#00122: iconst_0         => %122 = int(0)
#00123: iload_1          => nop
#00124: iadd             => %124 = %arg0 + %122
#00125: iaload           => %125 = %104[%124]
#00126: istore           => nop
#00128: aload            => nop
#00130: iload            => nop
#00132: iload            => nop
#00134: iastore          => %134 = %104[%125] = %125
#00135: iload_2          => nop
#00136: ireturn          => return %arg1
```
