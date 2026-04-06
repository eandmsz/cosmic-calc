# COSMIC Calculator
Calculator for the COSMIC desktop

- Native Rust applicaiton: No dependency on any other language (no wrappers for C or Python modules)
- Focusing on simplicity, ease of use, touch friendly operation: We have identified many features missing from kCalc and Gnome Calculator
- IEEE 754 double-precision floating-point precision: We want to keep it fast and lightweight and having more precision wouldn't make any pracital sense

# Features:
- Repeat last operation using =
- Predictable operation: only the = sign evaluates the expressions
- Automatic extension with scientific buttons when the window is in landscape mode
- Logical button layout
- Expressions are more readable, by using superscript, subscript and custom svg radical sign
- Copy/Paste expressions or results
- Trigonometry and radical functions work both before or after inputting an operand
- Customizable random function
- Intuitive Backspace and AC/C functions
- Native f64 Rust variables used to get the highest performance and lowest memory footprint:
		* 1 bit: sign
		* 11 bits: exponent
		* 52 bits: mantissa (fraction) which should give us 15-16 digits precision (we round it to 15 digits)
