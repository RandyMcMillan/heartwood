# Decode a base64 encoded string. Result is binary or unicode string.


import base64


def decode_bytes(s):
    return base64.b64decode(s)


def decode_str(s):
    return base64.b64decode(s).decode()
