


/// Declared `public`; may be accessed from outside its package.
pub const ACC_PUBLIC: u16 = 0x0001;
/// Declared `final`; no subclasses allowed.
pub const ACC_FINAL: u16 = 0x0010;
/// Treat superclass methods specially when invoked by the invokespecial instruction.
pub const ACC_SUPER: u16 = 0x0020;
/// Is an interface, not a class.
pub const ACC_INTERFACE: u16 = 0x0200;
/// Declared `abstract`; must not be instantiated.
pub const ACC_ABSTRACT: u16 = 0x0400;
/// Declared synthetic; not present in the source code.
pub const ACC_SYNTHETIC: u16 = 0x1000;
/// Declared as an annotation interface.
pub const ACC_ANNOTATION: u16 = 0x2000;
/// Declared as an enum class.
pub const ACC_ENUM: u16 = 0x4000;
/// Is a module, not a class or interface.
pub const ACC_MODULE: u16 = 0x8000;
