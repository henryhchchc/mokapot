#[rustfmt::skip]
#[must_use]
pub const fn empty_class_with_version(major: u16, minor: u16) -> [u8;40] {
    [
        0xCA, 0xFE, 0xBA, 0xBE, // Magic
        minor.to_be_bytes()[0], minor.to_be_bytes()[1], // Minor version
        major.to_be_bytes()[0], major.to_be_bytes()[1], // Major version
        // Constant pool
        0x00, 0x03, // Constant pool count 2+1
        0x07, // Tag: Class
        0x00, 0x02, // Name index: 2
        0x01, // Tag: Utf8
        0x00, 0x0A, // Length of string: 10
        0x48, 0x65, 0x6C, 0x6C, 0x6F, 0x57, 0x6F, 0x72, 0x6C, 0x64, // "Helloworld"
        0x00, 0x01, // Access flags: public 
        0x00, 0x01, // This class index
        0x00, 0x01, // Super class index
        0x00, 0x00, // Interfaces count
        0x00, 0x00, // Fields count
        0x00, 0x00, // Methods count
        0x00, 0x00, // Attributes count
    ]
}
