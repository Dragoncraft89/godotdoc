# This comment is a description of the method foo
func foo(id):
	return id + 42

# We can exclude functions from showing up
# [Hide]
func _bar():
	return 1337

# Besides functions, one can declare classes, enums, variables, constants
class MyClass:
	# One can even comment on individual variables in a class or enum
	var baz
	var test # Even comments on the same line as the declaration are honored

# Enums list all values
enum MyEnum {
	FIRST = 0, # This is the first entry
	SECOND,
	GAP = 42, # Here we have a gap in the numbering
	LAST
}

# Export arguments are honored too
export(int, 1, 8) var my_export = 5

# As well as types
const MY_CONST: int = 42

# Setter and getter will be visible in the docs as well
var my_var setget foo, _bar