# Retrieve an embedded test data file using filename.


class Files:
    def __init__(self):
        self._files = {}

    def set(self, filename, content):
        self._files[filename] = content

    def get(self, filename):
        return self._files[filename]


_files = Files()


def store_file(filename, content):
    _files.set(filename, content)


def get_file(filename):
    return _files.get(filename)
