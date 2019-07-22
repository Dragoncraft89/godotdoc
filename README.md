# GodotDoc

GodotDoc is a documentation generator for GdScript.

It extracts all symbols and the surrounding comments and generates a markdown file for display in, e.g. github.

Consider the following example:  
```
# This comment is a description of the method foo
func foo(id):
	return id + 42

# This function will not show up, because it is prefixed with an underscore
func _bar():
	return 1337

# The only function exempt from this rule is the _init()-function
func _init():
	pass

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
```

You can now invoke `godotdoc /path/to/source/directory -o /path/to/output/directory`

This will be the result:
## test.gd

### Classes:  
* MyClass  
    * **Variables**:  
        * baz  
        * test  
        ```
        Even comments on the same line as the declaration are honored
        ```

  
  
  
    ```
    Besides functions, one can declare classes, enums, variables, constants
    ```

  
### Enums:  
* MyEnum  
    **Values**:  
    * FIRST = 0  
      
    ```
    This is the first entry
    ```

  
    * SECOND = 1  
    * GAP = 42  
      
    ```
    Here we have a gap in the numbering
    ```

  
    * LAST = 43  
  
  
### Exports:  
* my\_export: (int, 1, 8) = `5`  
  
    ```
    Export arguments are honored too
    ```

  
### Constants:  
* MY\_CONST: int = `42`  
  
    ```
    As well as types
    ```

  
### Functions:  
* foo(id)  
  
    ```
    This comment is a description of the method foo
    ```

* \_init()  
  
    ```
    The only function exempt from this rule is the _init()-function
    ```

  
### Variables:  
* my\_var  
**Getter**: \_bar  
**Setter**: foo  
  
    ```
    Setter and getter will be visible in the docs as well
    ```