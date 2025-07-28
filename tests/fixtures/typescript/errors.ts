// Test file with various TypeScript errors for integration testing

interface User {
    id: number;
    name: string;
    email: string;
}

// Error: Property 'age' does not exist on type 'User'
function getUserAge(user: User): number {
    return user.age;
}

// Error: Cannot find name 'unknownFunction'
const result = unknownFunction();

// Error: Type 'string' is not assignable to type 'number'
const count: number = "not a number";

// Error: Expected 2 arguments, but got 1
function add(a: number, b: number): number {
    return a + b;
}
const sum = add(5);

// Error: Object is possibly 'undefined'
function processData(data?: { value: number }) {
    return data.value * 2;
}

// Multiple related errors from one root cause
class Calculator {
    // Error: Property 'history' has no initializer
    private history: number[];
    
    // Error: Property 'history' is used before being assigned
    constructor() {
        this.history.push(0);
    }
}

// Error: Duplicate identifier 'duplicateVar'
const duplicateVar = 1;
const duplicateVar = 2;

// Error: Unreachable code detected
function testReturn(): number {
    return 42;
    console.log("This won't run");
}