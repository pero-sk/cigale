# Cigale

[VCS extension now available!](https://marketplace.visualstudio.com/items?itemName=pero-sk.cigale)

A statically-typed, interpreted programming language built in Rust.

```
import stdl.console { cout };

func public static main() {
    cout("Hello, world!");
}
``` 

---

## Table of Contents

- [Install](#install)
- [Usage](#usage)
- [Language Overview](#language-overview)
- [Standard Library](#standard-library)
- [Project Structure](#project-structure)
- [Building from Source](#building-from-source)
- [Contributing](#contributing)

---

## Install

### Requirements
- [Rust/Cargo](https://rustup.rs)
- [Git](https://git-scm.com)

### Linux/Mac
download ```cigale.sh``` from releases.
```bash
chmod +x cigale.sh && ./cigale.sh install
```

### Windows
download ```cigale.bat``` from releases.
```batch
cigale.bat install
```

Please note that by default (running without a specific version), the install subcommand will install 0.1.0 and not latest.

Restart your terminal after installing. Cigale will be available as `cigale` in your PATH.

### Specific version
```bash
./cigale.sh install <branch>      # Linux/Mac
cigale.bat install <branch>       # Windows
```

---

## Usage

```bash
 cigale run                        # run project using build.yml
 cigale run <file.cig>             # run a single file
 cigale run <file.cig> [--no-stdl] # run without stdl
 cigale new <project_name>         # create a new project
 cigale install [version]          # install cigale
 cigale fetch [--global]           # fetch dependencies from cigale.properties
 cigale update                     # update to latest
 cigale version                    # show version
 cigale help                       # show this help
```

---

## Language Overview

### Types

| Type | Description |
|---|---|
| `int` | Integer |
| `float` | 32-bit float — suffix `f` e.g. `3.14f` |
| `double` | 64-bit float — suffix `d` e.g. `3.14d`, default for bare decimals |
| `str` | String |
| `bool` | Boolean |
| `list<T>` | Typed list |
| `list` | Untyped list |

### Variables

```
int x = 2;
str y = "hello";
float z = 3.14f;
double d = 3.14;
bool b = true;
int n = null;

list<str> names = ["alice", "bob"];
list<str|int> mixed = ["hello", 1];
list anything = [1, "two", 3.0f];
```

### Strings

```
str name = "world";
str greeting = $"hello {name}!";     // interpolation
str multiline = """
    hello
    world
""";
str concat = "hello" + " world";
```

### Comments

```
// single line

/"
    multi line
"/
```

### Operators

```
+ - * /         // arithmetic (/ always returns float)
~               // modulo
%               // percentage: 50 % 200 = 100 (200% of 50 or 50% of 200, both are the same so it doesn't matter.)
**              // exponent
== != < > <= >= // comparison
&& || !         // logical
^ & |           // bitwise XOR, AND, OR
<< >>           // bit shift
??              // nullish: x ?? fallback
```

### Control Flow

```
if (x > 0) {
    ...
} else if (x == 0) {
    ...
} else {
    ...
}

for (int i = 0, i < 10, i += 1) {
    ...
}

foreach (str s, names) {
    ...
}

while (x > 0) {
    ...
}

break;
continue;
```

### Match

```
inst colour { RED, GREEN, BLUE }

match (c) {
    colour.RED -> {
        ...
    }
    colour.GREEN | colour.BLUE -> {
        ...
    }
    _ -> {
        ...
    }
}
```

Works with enums, ints, strings, and other types.

### Functions

```
func add<int, int>(x, y) -> int {
    return x + y;
}

func greet<str>(name) -> str {
    return $"hello {name}!";
}

func noArgs() -> str {
    return "hello";
}

// generic
func identity<T>(x) -> T {
    return x;
}
```

### Classes

```
// sealed, non-instantiable
class mathsUtils {
    func square<int>(x) -> int {
        return x * x;
    }
}

// instantiable
class inst shape {
    int sides = 0;
    func blank area<>() -> double;      // must be implemented
    func describe<>() -> str {          // can be overridden
        return "I am a shape";
    }
}

// inheriting
class of<shape> circle {
    int radius = 20;
    func impl area<>() -> double {
        return 3.14159 * (radius ** 2);
    }
}

shape s = shape();
circle c = circle();
double a = c.area();
```

### Enums

```
inst direction { NORTH, SOUTH, EAST, WEST }

direction d = direction.NORTH;
```

### Access Modifiers

```
func public x() { ... }      // accessible anywhere (default)
func private x() { ... }     // own class only
func protected x() { ... }   // own class + inheriting classes

int public age;
int private name;
```

### Static Context

```
// all class functions are static
class inst maths {
    func add<int, int>(x, y) -> int { ... }
}
static {
    maths.add(1, 2);    // must be in static context
}

// static block outside class
static {
    func helper<int>(x) -> int { ... }
}
static {
    helper(42);
}

// static variable
static int GLOBAL = 42;
```

### Error Handling

```
import stdl.err { result, Error };
import stdl.err.funct { ok, err };

func safeDivide<int, int>(x, y) -> result<int> {
    if (y == 0) {
        return err(Error("division by zero"));
    }
    return ok(x ~ y);
}

result<int> r = safeDivide(10, 2);

// check before getting
if (r.is_err()) {
    cout(r.error());    // print the error
} else {
    cout(r.get());      // safe to get value
}

// OR check for null error
if (r.error() != null) {
    // handle error
} else {
    int val = r.get();
}
```

Custom errors:
```
class of<Error> myError { }

result<int, myError> r = err(myError());
```

### Imports

```
import stdl.maths { pi };           // selective
import stdl.maths { * };            // wildcard
import stdl.maths as m;             // aliased
import stdl.maths { sqrt as s };    // aliased selective
import "path/to/file" { myFunc };   // file import
```

### Entry Point

Every program needs exactly one top-level `main`:

```
func public static main() {
    // program starts here
}
```

### Nullish Operator

```
int x = null;
int y = x ?? 0;             // 0
int z = null ?? null ?? 42; // 42 (chains)
```

### Type Checking & Casting

```
bool isInt = typeof<int>(x);

float f = x<float>;     // cast x to float
int i = f<int>;         // warns: auto-rounds
```

---

## Standard Library

### `stdl.console`

```
import stdl.console { cout, couts, cin };

cout("hello");      // print with newline
couts("hello");     // print without newline
str input = cin();  // read line from console
```

### `stdl.io`

```
import stdl.io { open, perm, file };
import stdl.err { result };

result<file> f = open("path/to/file.txt", perm.RWP);
if (f.err != null) {
    // handle error
} else {
    file handle = f.val;
    result<str> content = handle.read();
    handle.write("hello!");
    handle.append(" world");
    handle.close();
}
```

Permissions: any combination of `R`, `W`, `P`, `A` in that order
e.g. `R`, `RW`, `RWP`, `RWPA`, `WP`, `A`, etc.

### `stdl.err`

```
import stdl.err { result, Error };
import stdl.err.funct { ok, err };
```

### `stdl.maths`

```
import stdl.maths { pi, e, tau, sqrt, abs, pow, floor, ceil, round, min, max };

double area = pi * (r ** 2);
double root = sqrt(16);         // 4.0
int floored = floor(3.7);       // 3
```

### `stdl.json`

```
import stdl.json { parse, stringify };

json_object obj = parse("{\"name\": \"cigale\", \"version\": 1}");
cout(obj.name);         // cigale

str json = stringify(obj, true);    // pretty print
```

### `stdl.project`

```
import stdl.project { name, description };

cout(name);         // project name or null
cout(description);  // project description or null
```

---

## Project Structure

```
myProject/
├── project.cfg             // project metadata
├── cigale.properties       // external dependencies
├── build.yml               // build configuration
└── src/
    └── main.cig
```

`project.cfg`:
```
name = "myProject";
description = "my cool project";
version = "1.0.0";
```

Without a project structure, `.cig` files can still be run directly via the interpreter.

use `cigale new <project_name>` to setup a basic project structure.

---

## Building from Source

```bash
# clone
git clone https://github.com/pero-sk/cigale
cd cigale

# build all binaries
cargo build --release --features="stdl" --bin cigale_stdl
cargo build --release --bin cigale_nostdl --bin cigale_cli

# or use the build script
./build.bat     # Windows PowerShell
./build.sh      # Unix Shell
```

Binaries end up in `target/release/`.

---

## Contributing

Contributions are welcome! The codebase is structured as:

```
src/
├── main.rs             // entry point (cigale_stdl)
├── main_nostdl.rs      // entry point (cigale_nostdl)
├── main_cli.rs         // CLI tool entry point (cigale_cli)
├── lexer/              // tokenizer
├── parser/             // AST + parser
├── analyser/           // semantic analysis
├── interpreter/        // tree-walking interpreter
└── stdl/               // standard library
    ├── console.rs
    ├── io.rs
    ├── err/
    ├── maths.rs
    ├── json.rs
    └── project.rs
```

---

## License

[MIT](LICENSE)