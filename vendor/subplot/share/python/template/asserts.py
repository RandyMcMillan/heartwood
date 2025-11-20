# Check two values for equality and give an error if they are not equal
def assert_eq(a, b):
    assert a == b, "expected %r == %r" % (a, b)


# Check two values for inequality and give an error if they are equal
def assert_ne(a, b):
    assert a != b, "expected %r != %r" % (a, b)


# Check that two dict values are equal.
def _assert_dict_eq(a, b):
    for key in a:
        assert key in b, f"exected {key} in both dicts"
        av = a[key]
        bv = b[key]
        assert_eq(type(av), type(bv))
        if isinstance(av, list):
            _assert_list_eq(av, bv)
        elif isinstance(av, dict):
            _assert_dict_eq(av, bv)
        else:
            assert_eq(av, bv)
    for key in b:
        assert key in a, f"exected {key} in both dicts"


# Check that two list values are equal
def _assert_list_eq(a, b):
    assert_eq(len(a), len(b))
    for (av, bv) in zip(a, b):
        assert_eq(type(av), type(bv))
        if isinstance(av, list):
            _assert_list_eq(av, bv)
        elif isinstance(av, dict):
            _assert_dict_eq(av, bv)
        else:
            assert_eq(av, bv)


# Recursively check two dictionaries are equal
def assert_dict_eq(a, b):
    assert isinstance(a, dict)
    assert isinstance(b, dict)
    _assert_dict_eq(a, b)
