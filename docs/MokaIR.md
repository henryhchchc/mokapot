## MokaIR

MokaIR is an intermediate representation of JVM bytecode in [mokapot](https://github.com/henryhchchc/mokapot).
It is in a register-based, SSA form, and is designed to be easy to analyze.
Please checkout the module [`mokapot::ir`](https://docs.rs/mokapot/latest/mokapot/ir/index.html) for more information.

To generate MokaIR from a method, use the following code.

```rust
use mokapot:jvm::Method;
use mokapot::ir::{MokaIRMethod, MokaIRMethodExt};

fn moka_ir(method: &Method) -> Result<MokaIRMethod, Box<dyn std::error::Error>> {
    let moka_ir_method = method.brew()?;
    Ok(moka_ir_method)
}
```

The following is an example of the generated IR from the method `test()` in [TestAnalysis.java](/test_data/mokapot/org/mokapot/test/TestAnalysis.java).

You may notice that there are lots of `nop`s in the generated MokaIR.
This because we indent to maintain a bijection between the original bytecode and the generated MokaIR.
Such a bijection facilitates the analysis involving dynamic execution - the runtime information (e.g., coverage) can be applied to MokaIR without needing any remapping.

| Address | JVM Instruction   | MokaIR                                                                                |
| :------ | :---------------- | :------------------------------------------------------------------------------------ |
| `#0000` | `ldc`             | `%0 = String("233")`                                                                  |
| `#0002` | `astore_3`        | `nop`                                                                                 |
| `#0003` | `iconst_2`        | `%3 = int(2)`                                                                         |
| `#0004` | `istore`          | `nop`                                                                                 |
| `#0006` | `iload_1`         | `nop`                                                                                 |
| `#0007` | `iload`           | `nop`                                                                                 |
| `#0009` | `iadd`            | `%9 = %arg0 + %3`                                                                     |
| `#000A` | `istore`          | `nop`                                                                                 |
| `#000C` | `iload_1`         | `nop`                                                                                 |
| `#000D` | `ifge`            | `if %arg0 >= 0 goto #0013`                                                            |
| `#0010` | `iconst_3`        | `%16 = int(3)`                                                                        |
| `#0011` | `istore`          | `nop`                                                                                 |
| `#0013` | `aload_0`         | `nop`                                                                                 |
| `#0014` | `aload_3`         | `nop`                                                                                 |
| `#0015` | `iload`           | `nop`                                                                                 |
| `#0017` | `iload_2`         | `nop`                                                                                 |
| `#0018` | `invokevirtual`   | `%24 = call int %this@org/mokapot/test/TestAnalysis::callMe(%0, Phi(%3, %16), %arg1)` |
| `#001B` | `istore`          | `nop`                                                                                 |
| `#001D` | `iload`           | `nop`                                                                                 |
| `#001F` | `ireturn`         | `return %24`                                                                          |
| `#0020` | `astore_3`        | `nop`                                                                                 |
| `#0021` | `getstatic`       | `%33 = read java/lang/System.out`                                                     |
| `#0024` | `aload_3`         | `nop`                                                                                 |
| `#0025` | `invokevirtual`   | `%37 = call void %33@java/io/PrintStream::println(%caught_exception)`                 |
| `#0028` | `iconst_0`        | `%40 = int(0)`                                                                        |
| `#0029` | `istore_3`        | `nop`                                                                                 |
| `#002A` | `iload_3`         | `nop`                                                                                 |
| `#002B` | `iload_2`         | `nop`                                                                                 |
| `#002C` | `if_icmpge`       | `if Phi(%40, %64) >= %arg1 goto #0046`                                                |
| `#002F` | `getstatic`       | `%47 = read java/lang/System.out`                                                     |
| `#0032` | `ldc`             | `%50 = String(0x61 0x02 0xED 0xA0 0x80 0x62 0x63 0x64 0x65 0x66) // Invalid UTF-8`    |
| `#0034` | `invokevirtual`   | `%52 = call void %47@java/io/PrintStream::println(%50)`                               |
| `#0037` | `aload_0`         | `nop`                                                                                 |
| `#0038` | `ldc`             | `%56 = String("233")`                                                                 |
| `#003A` | `iconst_0`        | `%58 = int(0)`                                                                        |
| `#003B` | `iconst_0`        | `%59 = int(0)`                                                                        |
| `#003C` | `invokevirtual`   | `%60 = call int %this@org/mokapot/test/TestAnalysis::callMe(%56, %58, %59)`           |
| `#003F` | `pop`             | `nop`                                                                                 |
| `#0040` | `iinc`            | `%64 = Phi(%40, %64) + 1`                                                             |
| `#0043` | `goto`            | `goto #002A`                                                                          |
| `#0046` | `iload_1`         | `nop`                                                                                 |
| `#0047` | `ifle`            | `if %arg0 <= 0 goto #0051`                                                            |
| `#004A` | `iload_2`         | `nop`                                                                                 |
| `#004B` | `ifle`            | `if %arg1 <= 0 goto #0057`                                                            |
| `#004E` | `goto`            | `goto #0055`                                                                          |
| `#0051` | `iload_2`         | `nop`                                                                                 |
| `#0052` | `ifge`            | `if %arg1 >= 0 goto #0057`                                                            |
| `#0055` | `iconst_0`        | `%85 = int(0)`                                                                        |
| `#0056` | `ireturn`         | `return %85`                                                                          |
| `#0057` | `iload_1`         | `nop`                                                                                 |
| `#0058` | `invokedynamic`   | `%88 = closure java/util/function/IntUnaryOperator applyAsInt#0(%arg0)`               |
| `#005D` | `astore_3`        | `nop`                                                                                 |
| `#005E` | `aload_3`         | `nop`                                                                                 |
| `#005F` | `iconst_0`        | `%95 = int(0)`                                                                        |
| `#0060` | `invokeinterface` | `%96 = call int %88@java/util/function/IntUnaryOperator::applyAsInt(%95)`             |
| `#0065` | `istore`          | `nop`                                                                                 |
| `#0067` | `iconst_3`        | `%103 = int(3)`                                                                       |
| `#0068` | `newarray`        | `%104 = new int[%103]`                                                                |
| `#006A` | `dup`             | `nop`                                                                                 |
| `#006B` | `iconst_0`        | `%107 = int(0)`                                                                       |
| `#006C` | `iconst_0`        | `%108 = int(0)`                                                                       |
| `#006D` | `iastore`         | `%109 = %104[%107] = %108`                                                            |
| `#006E` | `dup`             | `nop`                                                                                 |
| `#006F` | `iconst_1`        | `%111 = int(1)`                                                                       |
| `#0070` | `iconst_1`        | `%112 = int(1)`                                                                       |
| `#0071` | `iastore`         | `%113 = %104[%111] = %112`                                                            |
| `#0072` | `dup`             | `nop`                                                                                 |
| `#0073` | `iconst_2`        | `%115 = int(2)`                                                                       |
| `#0074` | `iconst_2`        | `%116 = int(2)`                                                                       |
| `#0075` | `iastore`         | `%117 = %104[%115] = %116`                                                            |
| `#0076` | `astore`          | `nop`                                                                                 |
| `#0078` | `aload`           | `nop`                                                                                 |
| `#007A` | `iconst_0`        | `%122 = int(0)`                                                                       |
| `#007B` | `iload_1`         | `nop`                                                                                 |
| `#007C` | `iadd`            | `%124 = %122 + %arg0`                                                                 |
| `#007D` | `iaload`          | `%125 = %104[%124]`                                                                   |
| `#007E` | `istore`          | `nop`                                                                                 |
| `#0080` | `aload`           | `nop`                                                                                 |
| `#0082` | `iload`           | `nop`                                                                                 |
| `#0084` | `iload`           | `nop`                                                                                 |
| `#0086` | `iastore`         | `%134 = %104[%125] = %125`                                                            |
| `#0087` | `iload`           | `nop`                                                                                 |
| `#0089` | `lookupswitch`    | `switch %125 { 1 => #00A4, 3 => #00AF, else => #00BA }`                               |
| `#00A4` | `aload_0`         | `nop`                                                                                 |
| `#00A5` | `aconst_null`     | `%165 = null`                                                                         |
| `#00A6` | `iconst_2`        | `%166 = int(2)`                                                                       |
| `#00A7` | `iconst_3`        | `%167 = int(3)`                                                                       |
| `#00A8` | `invokevirtual`   | `%168 = call int %this@org/mokapot/test/TestAnalysis::callMe(%165, %166, %167)`       |
| `#00AB` | `pop`             | `nop`                                                                                 |
| `#00AC` | `goto`            | `goto #00BA`                                                                          |
| `#00AF` | `aload_0`         | `nop`                                                                                 |
| `#00B0` | `aconst_null`     | `%176 = null`                                                                         |
| `#00B1` | `iconst_2`        | `%177 = int(2)`                                                                       |
| `#00B2` | `iconst_3`        | `%178 = int(3)`                                                                       |
| `#00B3` | `invokevirtual`   | `%179 = call int %this@org/mokapot/test/TestAnalysis::callMe(%176, %177, %178)`       |
| `#00B6` | `pop`             | `nop`                                                                                 |
| `#00B7` | `goto`            | `goto #00BA`                                                                          |
| `#00BA` | `iload`           | `nop`                                                                                 |
| `#00BC` | `tableswitch`     | `switch %96 { 1 => #00D8, 2 => #00E2, 3 => #00EC, else => #00F6 }`                    |
| `#00D8` | `getstatic`       | `%216 = read java/lang/System.out`                                                    |
| `#00DB` | `iconst_1`        | `%219 = int(1)`                                                                       |
| `#00DC` | `invokevirtual`   | `%220 = call void %216@java/io/PrintStream::println(%219)`                            |
| `#00DF` | `goto`            | `goto #00F6`                                                                          |
| `#00E2` | `getstatic`       | `%226 = read java/lang/System.out`                                                    |
| `#00E5` | `iconst_2`        | `%229 = int(2)`                                                                       |
| `#00E6` | `invokevirtual`   | `%230 = call void %226@java/io/PrintStream::println(%229)`                            |
| `#00E9` | `goto`            | `goto #00F6`                                                                          |
| `#00EC` | `getstatic`       | `%236 = read java/lang/System.out`                                                    |
| `#00EF` | `iconst_3`        | `%239 = int(3)`                                                                       |
| `#00F0` | `invokevirtual`   | `%240 = call void %236@java/io/PrintStream::println(%239)`                            |
| `#00F3` | `goto`            | `goto #00F6`                                                                          |
| `#00F6` | `iload_2`         | `nop`                                                                                 |
| `#00F7` | `ireturn`         | `return %arg1`                                                                        |
