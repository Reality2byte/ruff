import sys

if sys.version_info < (3, 8):

    def a():
        if b:
            print(1)
        elif c:
            print(2)
        return None

else:
    pass


import sys

if sys.version_info < (3, 8):
    pass

else:

    def a():
        if b:
            print(1)
        elif c:
            print(2)
        else:
            print(3)
        return None


# https://github.com/astral-sh/ruff/issues/16082

## Errors
if sys.version_info < (3, 12, 0):
    print()

if sys.version_info <= (3, 12, 0):
    print()

if sys.version_info < (3, 12, 11):
    print()

if sys.version_info < (3, 13, 0):
    print()

if sys.version_info <= (3, 13, 100000):
    print()


## No errors

if sys.version_info <= (3, 13, foo):
    print()

if sys.version_info <= (3, 13, 'final'):
    print()

if sys.version_info <= (3, 13, 0):
    print()

if sys.version_info < (3, 13, 37):
    print()

if sys.version_info <= (3, 13, 37):
    print()

if sys.version_info <= (3, 14, 0):
    print()

if sys.version_info <= (3, 14, 15):
    print()

# https://github.com/astral-sh/ruff/issues/18165

if sys.version_info.major >= 3:
    print("3")
else:
    print("2")

if sys.version_info.major > 3:
    print("3")
else:
    print("2")

if sys.version_info.major <= 3:
    print("3")
else:
    print("2")

if sys.version_info.major < 3:
    print("3")
else:
    print("2")

if sys.version_info.major == 3:
    print("3")
else:
    print("2")

# Semantically incorrect, skip fixing

if sys.version_info.major[1] > 3:
    print(3)
else:
    print(2)

if sys.version_info.major > (3, 13):
    print(3)
else:
    print(2)

if sys.version_info.major[:2] > (3, 13):
    print(3)
else:
    print(2)
