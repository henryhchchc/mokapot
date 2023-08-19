#[derive(Debug)]
pub struct ModuleReference {
    pub name: String,
}

#[derive(Debug)]
pub struct PackageReference {
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ClassReference {
    pub name: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FieldReference {
    pub class: ClassReference,
    pub name: String,
    pub descriptor: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InterfaceMethodReference {
    pub class: ClassReference,
    pub name: String,
    pub descriptor: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ClassMethodReference {
    pub class: ClassReference,
    pub name: String,
    pub descriptor: String,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum MethodReference {
    Class(ClassMethodReference),
    Interface(InterfaceMethodReference),
}
