#!/usr/bin/env python3
"""Test file with various Python errors for integration testing"""

# Error: Undefined variable
print(undefined_variable)

# Error: Import error
from nonexistent_module import something

# Error: Type errors (if using type hints)
def add_numbers(a: int, b: int) -> int:
    return a + b

result: int = add_numbers("5", "10")  # Type error

# Error: Indentation error
def broken_indentation():
    if True:
        print("indented")
       print("wrong indentation")  # IndentationError

# Error: Syntax error
def syntax_error():
    return 5 +  # SyntaxError: invalid syntax

# Error: Name error in class
class MyClass:
    def method1(self):
        self.undefined_method()  # AttributeError
    
    def method2(self):
        return self.undefined_attribute  # AttributeError

# Error: Invalid function call
def takes_two_args(a, b):
    return a + b

takes_two_args(1)  # TypeError: missing required positional argument

# Error: Division by zero (caught by some linters)
def divide(a, b):
    return a / b

result = divide(10, 0)

# Error: Unreachable code
def unreachable():
    return 42
    print("This won't run")  # Unreachable code

# Error: Unused variable (caught by linters)
def unused_vars():
    x = 10
    y = 20  # Unused variable
    return x

# Error: Redefinition
value = 10
value = "string"  # Redefinition with different type

# Error: Missing return
def should_return_int() -> int:
    print("forgot to return")  # Missing return statement