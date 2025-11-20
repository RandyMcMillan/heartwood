import unittest

from context import Context


class ContextTests(unittest.TestCase):
    def test_converts_to_empty_dict_initially(self):
        ctx = Context()
        self.assertEqual(ctx.as_dict(), {})

    def test_set_item(self):
        ctx = Context()
        ctx["foo"] = "bar"
        self.assertEqual(ctx["foo"], "bar")

    def test_does_not_contain_item(self):
        ctx = Context()
        self.assertFalse("foo" in ctx)

    def test_no_longer_contains_item(self):
        ctx = Context()
        ctx["foo"] = "bar"
        del ctx["foo"]
        self.assertFalse("foo" in ctx)

    def test_contains_item(self):
        ctx = Context()
        ctx["foo"] = "bar"
        self.assertTrue("foo" in ctx)

    def test_get_returns_default_if_item_does_not_exist(self):
        ctx = Context()
        self.assertEqual(ctx.get("foo"), None)

    def test_get_returns_specified_default_if_item_does_not_exist(self):
        ctx = Context()
        self.assertEqual(ctx.get("foo", "bar"), "bar")

    def test_get_returns_value_if_item_exists(self):
        ctx = Context()
        ctx["foo"] = "bar"
        self.assertEqual(ctx.get("foo", "yo"), "bar")

    def test_reprs_itself_when_empty(self):
        ctx = Context()
        self.assertFalse("foo" in repr(ctx))

    def test_reprs_itself_when_not_empty(self):
        ctx = Context()
        ctx["foo"] = "bar"
        self.assertTrue("foo" in repr(ctx))
        self.assertTrue("bar" in repr(ctx))


class ContextMemoryTests(unittest.TestCase):
    def test_recall_raises_exception_for_unremembered_value(self):
        ctx = Context()
        with self.assertRaises(KeyError):
            ctx.recall_value("foo")

    def test_recall_returns_remembered_value(self):
        ctx = Context()
        ctx.remember_value("foo", "bar")
        self.assertEqual(ctx.recall_value("foo"), "bar")

    def test_remember_raises_exception_for_previously_remembered(self):
        ctx = Context()
        ctx.remember_value("foo", "bar")
        with self.assertRaises(KeyError):
            ctx.remember_value("foo", "bar")

    def test_expand_returns_pattern_without_values_as_is(self):
        ctx = Context()
        self.assertEqual(ctx.expand_values("foo"), "foo")

    def test_expand_allows_double_dollar_escapes(self):
        ctx = Context()
        self.assertEqual(ctx.expand_values("$${foo}"), "$${foo}")

    def test_expand_raises_exception_for_empty_name_expansion_as_is(self):
        ctx = Context()
        with self.assertRaises(KeyError):
            ctx.expand_values("${}")

    def test_expand_raises_error_for_unrememebered_values(self):
        ctx = Context()
        with self.assertRaises(KeyError):
            ctx.expand_values("${foo}")

    def test_expands_rememebered_values(self):
        ctx = Context()
        ctx.remember_value("foo", "bar")
        self.assertEqual(ctx.expand_values("${foo}"), "bar")


class ContextNamepaceTests(unittest.TestCase):
    def test_explicit_namespaces_are_empty_dicts_initially(self):
        ctx = Context()
        ns = ctx.declare("foo")
        self.assertEqual(ns.as_dict(), {})

    def test_declaring_explicit_namespaces_is_idempotent(self):
        ctx = Context()
        ns1 = ctx.declare("foo")
        ns2 = ctx.declare("foo")
        self.assertEqual(id(ns1), id(ns2))

    def test_knows_their_name(self):
        ctx = Context()
        ns = ctx.declare("foo")
        self.assertEqual(ns.name, "foo")

    def test_sets_key(self):
        ctx = Context()
        ns = ctx.declare("foo")
        ns["bar"] = "yo"
        self.assertEqual(ns["bar"], "yo")

    def test_gets(self):
        ctx = Context()
        ns = ctx.declare("foo")
        ns["bar"] = "yo"
        self.assertEqual(ns.get("bar", "argh"), "yo")

    def test_get_without_default_doesnt_set(self):
        ctx = Context()
        ns = ctx.declare("foo")
        ns.get("bar")
        self.assertFalse("bar" in ns)

    def test_gets_with_default_sets_as_well(self):
        ctx = Context()
        ns = ctx.declare("foo")
        self.assertEqual(ns.get("bar", "yo"), "yo")
        self.assertEqual(ns["bar"], "yo")

    def test_does_not_contain_key(self):
        ctx = Context()
        ns = ctx.declare("foo")
        self.assertFalse("bar" in ns)

    def test_contains_key(self):
        ctx = Context()
        ns = ctx.declare("foo")
        ns["bar"] = "yo"
        self.assertTrue("bar" in ns)

    def test_deletes(self):
        ctx = Context()
        ns = ctx.declare("foo")
        ns["bar"] = "yo"
        del ns["bar"]
        self.assertFalse("bar" in ns)


unittest.main()
