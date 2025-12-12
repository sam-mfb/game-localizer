# game-localizer

Tools for creating and applying binary patches to game files.

## Installation

```
cargo install --path .
```

## Commands

### Patch

Create a patch:
```
game-localizer patch create <original> <modified> <patch-output>
```

Apply a patch:
```
game-localizer patch apply <original> <patch-file> <output>
```

### Hash

Calculate SHA-256 hash of a file:
```
game-localizer hash calculate <file>
```

Compare two files by hash:
```
game-localizer hash compare <file1> <file2>
```

Check if a file matches an expected hash:
```
game-localizer hash check <hash> <file>
```
