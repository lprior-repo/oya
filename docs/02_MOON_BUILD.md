# Build Pipeline: Moon

Use `moon run` for all build/test/lint flows in this repo.

## Common Tasks

```bash
moon run :quick         # fast validation
moon run :check         # type checking
moon run :test          # test suite
moon run :build         # release build
moon run :fmt-fix       # format fixes
moon run :coverage      # coverage run
moon run :mutants-quick # mutation smoke
moon run :ci            # clippy + test + release build
```

## Absolute Verification

To ensure no cached success masks a subtle regression, run:

```bash
moon run :ci --force
```

Do not run `cargo` directly for repository build/test/lint workflows; use moon tasks as the single entrypoint.
