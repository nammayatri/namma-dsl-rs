# namma-dsl-rs

Rust storage code generator for Namma DSL specs.  
It reads `spec/Storage/*.yaml` and generates:

- domain types
- Diesel `table!` schema
- Diesel models (`Queryable`/`Insertable`)
- typed query functions
- SQL migrations

## Install

```bash
cargo install --path .
```

## If `namma-dsl-rs: command not found` after install

Cargo installs binaries to `~/.cargo/bin`. Add it to `PATH` (zsh/macOS):

```bash
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
source ~/.zshrc
hash -r
```

Verify:

```bash
namma-dsl-rs --help
```

Fallback (without PATH change):

```bash
~/.cargo/bin/namma-dsl-rs --help
```

## Usage

Run from the root of a git repo that contains `spec/` folders with `dsl-config.toml`:

```bash
# changed files only (vs HEAD)
namma-dsl-rs

# force all files
namma-dsl-rs --all

# process one spec folder
namma-dsl-rs --path path/to/spec

# skip cargo fmt
namma-dsl-rs --skip-fmt
```

## Expected service layout

```text
my-service/
├── spec/
│   ├── dsl-config.toml
│   └── Storage/
│       ├── Seat.yaml
│       └── Account.yaml
├── src-read-only/
│   ├── domain/types/
│   └── storage/{schema,models,queries}/
└── migrations/
```

## Minimal `dsl-config.toml`

```toml
[output]
domain_type = "src-read-only/domain/types"
diesel_schema = "src-read-only/storage/schema"
diesel_model = "src-read-only/storage/models"
queries = "src-read-only/storage/queries"

[[output.sql]]
path = "migrations"
database = "my_app"

[storage]
extra_default_fields = [
  { name = "createdAt", rust_type = "chrono::DateTime<chrono::Utc>" },
  { name = "updatedAt", rust_type = "chrono::DateTime<chrono::Utc>" },
]

[generate]
generators = ["DomainType", "DieselSchema", "DieselModel", "Queries", "SQL"]
```

## Type mapping (Haskell DSL -> Rust DSL)

- `Text` -> `String`
- `Int` -> `i32`
- `Double` -> `f64`
- `Bool` -> `bool`
- `Maybe X` -> `Option<X>`
- `Id X` -> `Id<X>`
- `UTCTime` -> `chrono::DateTime<chrono::Utc>`

## kvFunction mapping

Use the `*WithKV` names in YAML:

- `findOneWithKV`
- `findAllWithKV` (or `findAllWithOptionsKV`)
- `updateManyWithKV` (alias of `updateWithKV`)
- `updateOneWithKV`
- `deleteWithKV`

`createWithKV` / `createManyWithKV` are generated as default helpers (`create`, `create_many`) and do not need to be declared in the YAML `queries` block.

## Quick integration

```bash
#!/usr/bin/env bash
set -euo pipefail
namma-dsl-rs "$@"
cargo fmt
```

## License

AGPL-3.0-or-later
