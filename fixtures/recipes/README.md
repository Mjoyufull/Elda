# Recipe Fixtures

Concrete `pkg.lua` examples used to validate Elda's maintained-package path.

- `fsel/`: maintained binary-lane recipe shape against a real upstream release layout.
- `yoka-core-profile/`: first-class `kind = "profile"` recipe with machine-shape policy.
- `flag-suite-demo/`: extended flag-system fixture with descriptions, cardinality groups,
  conditional dependency predicates, implies, and conflicts.

For broad authoring examples, see `examples/recipes/`. Generated/imported local
metadata preserves existing files unless the command includes `--replace`.
