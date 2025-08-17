#!/usr/bin/env python3
"""Example Python file for testing the command runner"""

def main():
    """Main entry point"""
    print("Hello from Python!")
    result = add(2, 3)
    print(f"2 + 3 = {result}")

def add(a, b):
    """Add two numbers"""
    return a + b

def test_addition():
    """Test addition function"""
    assert add(2, 2) == 4
    
def test_subtraction():
    """Test subtraction"""
    assert 5 - 3 == 2

class TestMath:
    """Test class for math operations"""
    
    def test_multiplication(self):
        """Test multiplication"""
        assert 3 * 4 == 12
    
    def test_division(self):
        """Test division"""
        assert 10 / 2 == 5
        
    def test_add_negative(self):
        """Test adding negative numbers"""
        assert add(-1, 1) == 0

# pytest style test
def test_with_pytest_style():
    """This is a pytest-style test function"""
    given = [1, 2, 3]
    expected = 6
    assert sum(given) == expected

# unittest style
import unittest

class TestAddition(unittest.TestCase):
    def test_positive_numbers(self):
        self.assertEqual(add(3, 4), 7)
    
    def test_negative_numbers(self):
        self.assertEqual(add(-3, -4), -7)

if __name__ == "__main__":
    main()