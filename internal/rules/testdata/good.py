import os


def load(path):
    return os.path.exists(path)


# TODO(lazar-harness-2mh) wire this into the loader
def done():
    return True
