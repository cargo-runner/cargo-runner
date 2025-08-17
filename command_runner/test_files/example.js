// Example JavaScript file for testing the command runner

function main() {
    console.log("Hello from JavaScript!");
    const result = add(2, 3);
    console.log(`2 + 3 = ${result}`);
}

function add(a, b) {
    return a + b;
}

// Jest style tests
test('addition', () => {
    expect(add(2, 2)).toBe(4);
});

test('subtraction', () => {
    expect(5 - 3).toBe(2);
});

describe('Math operations', () => {
    it('should multiply correctly', () => {
        expect(3 * 4).toBe(12);
    });
    
    it('should divide correctly', () => {
        expect(10 / 2).toBe(5);
    });
    
    test('negative numbers', () => {
        expect(add(-1, 1)).toBe(0);
    });
});

// Mocha style tests
describe('Addition function', function() {
    it('should add positive numbers', function() {
        const result = add(3, 4);
        if (result !== 7) {
            throw new Error(`Expected 7 but got ${result}`);
        }
    });
    
    it('should handle zero', function() {
        const result = add(0, 0);
        if (result !== 0) {
            throw new Error(`Expected 0 but got ${result}`);
        }
    });
});

// Node.js module check
if (require.main === module) {
    main();
}

module.exports = { add };