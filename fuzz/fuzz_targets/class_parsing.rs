#![no_main]

use libfuzzer_sys::fuzz_target;
use mokapot::jvm::Class;

fuzz_target!(|data: &[u8]| {
    let _ = Class::from_reader(data);
});
