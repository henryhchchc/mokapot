use afl::fuzz;

use mokapot::jvm::Class;

fn main() {
    fuzz!(|data: &[u8]| {
        let _ = Class::read_from(data);
    });
}
