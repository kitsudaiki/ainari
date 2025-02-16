# Code Styling Guide

!!! note

    This is a very basic code styling guide with some hints and restrictions, which have to be followed when contributing code. It is still in a very early stage and will be updated over time.

    Also look into other already existing source-files

    If you seen anything in the code, which doesn't follow this guide here, then please open a Feature-issue with `QA` label.

## General rule

!!! info ""

    Source-Code must be as explicit as possible. Try the keep the amount of used language features as basic as possible. Not every feature has to be used only because it exist. Code, which is hard to understand and to maintain, is the greatest risk factor for stability and security.

## Clang-format

For code-styling `clang-format` is used. With the following steps a pre-commit hook can be used,
which styles the coded based on the clang-format file, when ever a `git commit` is done.

Installation:

```
sudo apt-get install clang-format-18
```

Load pre-commit hook

```
cd OpenHanami
git config core.hooksPath .githooks
```

## Limitations

### Length of files

-   soft-cap: `1000` lines

## Indentation

-   4 whitespaces
-   no TAB

## Naming

| localtion      | styling     | example                        |
| -------------- | ----------- | ------------------------------ |
| file-names     | snake case  | `example_file.cpp`             |
| class-names    | pascal case | `ExampleClass()`               |
| function-names | camel case  | `void exampleFunction()`       |
| defines        | upper case  | `#define EXAMPLE_DEFINITION 1` |
| enums          | upper case  | `UNDEFINED_VAL=1`              |

## License

Each source-file need the Apache 2 copy-right header a the beginning. They can be copied of the
other files of the same language within the repository.

## Cpp specific structures

### `if`-conditions

`if`-condition must always use the `{}`, even c++ allows to omit them when the content is only one
line

!!! example

    ```cpp
    if(id == 0) {
    	return;
    }
    ```

`if`-condition must never mix `&&`, `||`, ... . `if`-conditions must always be consistent. Using
different binary operations within the same condition makes it harder to read and can lead easily to
mistakes. If there are different binary operations necessary, then move one type out of the
condition into a bool-variable or use a section if-condition.

!!! example

    ```cpp
    const bool doesMatch = x == 42 || y == 42;
    if(id == 0
    	&& doesMatch)
    {
    	return;
    }
    ```

### loops

Same like `if`-conditions they must always use the `{}`, even if the content of the loop contains
only one line

!!! example

    ```cpp
    for(uint32_t i = 0; i < 42; i++) {
    	itemList[i] *= 2;
    }
    ```

### `const`

Make as many variables `const` as possible.

### `auto`-type

Code must be as explicit as possible and hiding types behind an `auto`-type makes the code harder to
understand for people, who didn't wrote the code. Because of this use the `auto`- as less as
possible and ONLY very locally limited.

For example:

-   find values in maps

!!! example

    ```cpp
    const auto it = m_valueMap.find(key);
    ```

-   iteration over maps:

!!! example

    ```cpp
    for(const auto& [id, content] : m_items)
    ```

-   storing lamda-functions

Hiding primitive types like `int` and so on is not allowed.

### `template`

Same like the `auto`-type. Don't try to use `template` everywhere, because you think it is cool or
"modern" cpp programming. Too many templates making the code harder to read and longer to compile.
Use them only, when you have good reasons for this decision.

### `lambda`-functions

Same like for `auto`-type and `template`. Use them only if necessary. If you can solve the same task
with a normal function, then use the normal function instead of the `lambda`-function.
