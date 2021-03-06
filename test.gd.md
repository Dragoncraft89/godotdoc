## test.gd

### Classes:  
* MyClass  
  
    ```
    Besides functions, one can declare classes, enums, variables, constants
    ```

  
    * **Variables**:  
        * baz  
        ```
        One can even comment on individual variables in a class or enum
        ```

        * test  
        ```
        Even comments on the same line as the declaration are honored
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

  
### Variables:  
* my\_var  
**Getter**: \_bar  
**Setter**: foo  
  
    ```
    Setter and getter will be visible in the docs as well
    ```

  
