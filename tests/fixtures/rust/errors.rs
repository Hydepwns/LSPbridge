// Test file with various Rust errors for integration testing

// Error: cannot find value `undefined_var` in this scope
fn use_undefined() {
    println!("{}", undefined_var);
}

// Error: mismatched types
fn type_mismatch() -> i32 {
    "not a number"
}

// Error: cannot borrow `x` as mutable more than once
fn borrow_checker_error() {
    let mut x = vec![1, 2, 3];
    let y = &mut x;
    let z = &mut x; // Second mutable borrow
    println!("{:?} {:?}", y, z);
}

// Error: missing lifetime specifier
struct Container {
    value: &str, // Missing lifetime
}

// Error: the trait `Display` is not implemented
struct CustomType {
    data: i32,
}

fn print_custom(c: CustomType) {
    println!("{}", c); // Display not implemented
}

// Error: cannot move out of borrowed content
fn move_error(s: &String) -> String {
    *s // Cannot move out of borrowed content
}

// Error: unreachable pattern
fn match_error(x: Option<i32>) {
    match x {
        Some(n) => println!("{}", n),
        None => println!("None"),
        Some(5) => println!("Five"), // Unreachable
    }
}

// Multiple related errors
fn multiple_errors() {
    let mut vec = Vec::new();
    vec.push(); // Error: missing argument
    vec.unknown_method(); // Error: no method named `unknown_method`
    vec[100]; // Error: index out of bounds (runtime, but caught by some analyzers)
}

// Error: recursion without base case (infinite recursion)
fn infinite_recursion(n: i32) -> i32 {
    infinite_recursion(n + 1)
}