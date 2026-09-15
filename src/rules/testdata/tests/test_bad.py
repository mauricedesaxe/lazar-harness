from mypkg import thing


def test_thing():
    print("debug in test is fine")
    assert thing() is True
