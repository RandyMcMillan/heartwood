# import logging
import re


# Store context between steps.
class Context:
    def __init__(self):
        self._vars = {}
        self._ns = {}

    def as_dict(self):
        return dict(self._vars)

    def get(self, key, default=None):
        return self._vars.get(key, default)

    def __getitem__(self, key):
        return self._vars[key]

    def __setitem__(self, key, value):
        #        logging.debug("Context: key {!r} set to {!r}".format(key, value))
        self._vars[key] = value

    def keys(self):
        return self._vars.keys()

    def __contains__(self, key):
        return key in self._vars

    def __delitem__(self, key):
        del self._vars[key]

    def __repr__(self):
        return repr({"vars": self._vars, "namespaces": self._ns})

    def declare(self, name):
        if name not in self._ns:
            self._ns[name] = NameSpace(name)
        return self._ns[name]

    def remember_value(self, name, value):
        ns = self.declare("_values")
        if name in ns:
            raise KeyError(name)
        ns[name] = value

    def recall_value(self, name):
        ns = self.declare("_values")
        if name not in ns:
            raise KeyError(name)
        return ns[name]

    def expand_values(self, pattern):
        parts = []
        while pattern:
            m = re.search(r"(?<!\$)\$\{(?P<name>\S*)\}", pattern)
            if not m:
                parts.append(pattern)
                break
            name = m.group("name")
            if not name:
                raise KeyError("empty name in expansion")
            value = self.recall_value(name)
            parts.append(value)
            pattern = pattern[m.end() :]
        return "".join(parts)


class NameSpace:
    def __init__(self, name):
        self.name = name
        self._dict = {}

    def as_dict(self):
        return dict(self._dict)

    def get(self, key, default=None):
        if key not in self._dict:
            if default is None:
                return None
            self._dict[key] = default
        return self._dict[key]

    def __setitem__(self, key, value):
        self._dict[key] = value

    def __getitem__(self, key):
        return self._dict[key]

    def keys(self):
        return self._dict.keys()

    def __contains__(self, key):
        return key in self._dict

    def __delitem__(self, key):
        del self._dict[key]

    def __repr__(self):
        return repr(self._dict)
