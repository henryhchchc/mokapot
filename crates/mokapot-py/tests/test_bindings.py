import pytest
from mokapot import Class, ClassAccessFlags, ClassRef, ParseError


def test_parse_invalid_bytes_raises_parse_error() -> None:
    with pytest.raises(ParseError):
        Class.from_bytes(b"")


def test_class_access_flags_constants_and_helpers() -> None:
    public = ClassAccessFlags.PUBLIC
    super_flag = ClassAccessFlags.SUPER

    assert isinstance(public.bits, int)
    assert int(public) == public.bits
    assert public.contains(ClassAccessFlags.PUBLIC)
    assert not public.contains(super_flag)

    merged = ClassAccessFlags.from_bits(public.bits | super_flag.bits)
    assert merged.contains(public)
    assert merged.contains(super_flag)


def test_class_ref_constructor_and_repr() -> None:
    ref = ClassRef("java/lang/Object")

    assert ref.binary_name == "java/lang/Object"
    assert "java/lang/Object" in repr(ref)


def test_parse_compiled_java_class_file(java_classes_dir) -> None:
    class_file = java_classes_dir / "org" / "mokapot" / "test" / "MyClass.class"
    cls = Class.from_file(str(class_file))

    assert cls.binary_name == "org/mokapot/test/MyClass"
    assert cls.super_class is not None
    assert cls.super_class.binary_name == "java/lang/Object"
