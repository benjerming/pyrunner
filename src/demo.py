def some_python_func(callback, obj):
    total = 100
    for done in range(0, total + 1, 10):
        callback(done, total, obj)
        print(f"Python side: {done}/{total}")