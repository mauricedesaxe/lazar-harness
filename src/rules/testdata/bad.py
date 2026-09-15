import os


def load(path):
    print("loading", path)
    try:
        return os.path.exists(path)
    except:
        pass


# TODO wire this into the loader
def done():
    return True
