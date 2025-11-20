import base64
import unittest

import encoding


class EncodingTests(unittest.TestCase):
    def test_str_roundtrip(self):
        original = "foo\nbar\0"
        encoded = base64.b64encode(original.encode())
        self.assertEqual(encoding.decode_str(encoded), original)

    def test_bytes_roundtrip(self):
        original = b"foo\nbar\0"
        encoded = base64.b64encode(original)
        self.assertEqual(encoding.decode_bytes(encoded), original)


unittest.main()
